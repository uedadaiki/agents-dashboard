import type { AgentStateType } from "@agents-dashboard/shared";

type NotifiableState = "idle" | "permissionWaiting" | "error";

export interface NotificationSettings {
  enabled: boolean;
  soundEnabled: boolean;
  desktopEnabled: boolean;
  notifyOn: {
    idle: boolean;
    permissionWaiting: boolean;
    error: boolean;
  };
}

const DEFAULT_SETTINGS: NotificationSettings = {
  enabled: true,
  soundEnabled: true,
  desktopEnabled: true,
  notifyOn: {
    idle: true,
    permissionWaiting: true,
    error: true,
  },
};

// Place custom sound files in static/sounds/ to override generated tones.
// Supported: idle.mp3, permission-waiting.mp3, error.mp3
const SOUND_PATHS: Record<NotifiableState, string> = {
  idle: "/sounds/idle.mp3",
  permissionWaiting: "/sounds/permission-waiting.mp3",
  error: "/sounds/error.mp3",
};

const STORAGE_KEY = "agents-dashboard-notifications";

export class NotificationService {
  private settings: NotificationSettings;
  private soundBuffers = new Map<NotifiableState, ArrayBuffer>();
  private audioContext: AudioContext | null = null;
  private audioUnlocked = false;

  constructor() {
    this.settings = this.loadSettings();
    this.preloadSounds();
    this.setupUserInteractionUnlock();
  }

  /**
   * Safari and Firefox require a user gesture before AudioContext can produce
   * sound. We listen for the first interaction event, resume the AudioContext,
   * and play a silent buffer to fully "unlock" audio for later programmatic use.
   */
  private setupUserInteractionUnlock(): void {
    const unlock = () => {
      if (this.audioUnlocked) return;
      this.audioUnlocked = true;

      const ctx = this.getAudioContext();
      if (ctx.state === "suspended") {
        ctx.resume();
      }

      // Play a silent buffer to fully unlock audio on Safari/Firefox.
      const silent = ctx.createBuffer(1, 1, ctx.sampleRate);
      const src = ctx.createBufferSource();
      src.buffer = silent;
      src.connect(ctx.destination);
      src.start();

      for (const event of ["click", "touchstart", "keydown"]) {
        document.removeEventListener(event, unlock, true);
      }
    };

    for (const event of ["click", "touchstart", "keydown"]) {
      document.addEventListener(event, unlock, { capture: true, once: false });
    }
  }

  private getAudioContext(): AudioContext {
    if (!this.audioContext) {
      this.audioContext = new AudioContext();
    }
    return this.audioContext;
  }

  /**
   * Preload sounds as ArrayBuffers so we can decode fresh AudioBufferSourceNodes
   * on each play. This avoids HTMLAudioElement reuse issues on Safari.
   */
  private preloadSounds(): void {
    for (const [state, path] of Object.entries(SOUND_PATHS)) {
      fetch(path)
        .then((res) => {
          if (res.ok) return res.arrayBuffer();
          throw new Error(`HTTP ${res.status}`);
        })
        .then((buf) => {
          this.soundBuffers.set(state as NotifiableState, buf);
        })
        .catch(() => {
          // Sound file not available — will fall back to generated tone
        });
    }
  }

  getSettings(): NotificationSettings {
    return { ...this.settings };
  }

  updateSettings(partial: Partial<NotificationSettings>): void {
    this.settings = { ...this.settings, ...partial };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
  }

  updateNotifyOn(partial: Partial<NotificationSettings["notifyOn"]>): void {
    this.settings.notifyOn = { ...this.settings.notifyOn, ...partial };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
  }

  getPermissionState(): NotificationPermission | "unsupported" {
    if (!("Notification" in window)) return "unsupported";
    return Notification.permission;
  }

  async requestPermission(): Promise<boolean> {
    if (!("Notification" in window)) return false;
    if (Notification.permission === "granted") return true;
    const result = await Notification.requestPermission();
    return result === "granted";
  }

  async notifyStateChange(
    sessionId: string,
    projectName: string,
    newState: AgentStateType,
  ): Promise<void> {
    if (!this.settings.enabled) return;

    const shouldNotify =
      (newState === "idle" && this.settings.notifyOn.idle) ||
      (newState === "permission_waiting" && this.settings.notifyOn.permissionWaiting) ||
      (newState === "error" && this.settings.notifyOn.error);

    if (!shouldNotify) return;

    const title = this.getTitle(newState);
    const body = `${projectName} (${sessionId.slice(0, 8)})`;

    // Desktop notification
    if (this.settings.desktopEnabled && Notification.permission === "granted") {
      new Notification(title, { body, tag: `agent-${sessionId}` });
    }

    // Sound notification
    if (this.settings.soundEnabled) {
      await this.playSound(newState);
    }
  }

  private getTitle(state: AgentStateType): string {
    switch (state) {
      case "idle": return "Agent finished turn";
      case "permission_waiting": return "Agent waiting for permission";
      case "error": return "Agent encountered an error";
      default: return "Agent state changed";
    }
  }

  private stateToKey(state: AgentStateType): NotifiableState | null {
    switch (state) {
      case "idle": return "idle";
      case "permission_waiting": return "permissionWaiting";
      case "error": return "error";
      default: return null;
    }
  }

  private async playSound(state: AgentStateType): Promise<void> {
    const ctx = this.getAudioContext();

    // Ensure the context is running (may be suspended without user gesture)
    if (ctx.state === "suspended") {
      try {
        await ctx.resume();
      } catch {
        return;
      }
    }

    const key = this.stateToKey(state);
    if (key) {
      const arrayBuffer = this.soundBuffers.get(key);
      if (arrayBuffer) {
        try {
          // decodeAudioData consumes the buffer, so pass a copy each time
          const audioBuffer = await ctx.decodeAudioData(arrayBuffer.slice(0));
          const source = ctx.createBufferSource();
          source.buffer = audioBuffer;
          source.connect(ctx.destination);
          source.start();
          return;
        } catch {
          // Decode failed — fall through to generated tone
        }
      }
    }
    this.playGeneratedTone(state);
  }

  private playGeneratedTone(state: AgentStateType): void {
    try {
      const ctx = this.getAudioContext();
      const oscillator = ctx.createOscillator();
      const gain = ctx.createGain();
      oscillator.connect(gain);
      gain.connect(ctx.destination);

      gain.gain.setValueAtTime(0.3, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.5);

      switch (state) {
        case "idle":
          oscillator.frequency.setValueAtTime(800, ctx.currentTime);
          oscillator.type = "sine";
          break;
        case "permission_waiting":
          oscillator.frequency.setValueAtTime(600, ctx.currentTime);
          oscillator.frequency.setValueAtTime(900, ctx.currentTime + 0.15);
          oscillator.type = "triangle";
          break;
        case "error":
          oscillator.frequency.setValueAtTime(300, ctx.currentTime);
          oscillator.type = "sawtooth";
          break;
        default:
          oscillator.frequency.setValueAtTime(500, ctx.currentTime);
          oscillator.type = "sine";
      }

      oscillator.start(ctx.currentTime);
      oscillator.stop(ctx.currentTime + 0.5);
    } catch {
      // Audio context not available
    }
  }

  private loadSettings(): NotificationSettings {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        return {
          ...DEFAULT_SETTINGS,
          ...parsed,
          notifyOn: { ...DEFAULT_SETTINGS.notifyOn, ...parsed.notifyOn },
        };
      }
    } catch {
      // Ignore
    }
    return { ...DEFAULT_SETTINGS };
  }
}

export const notificationService = new NotificationService();
