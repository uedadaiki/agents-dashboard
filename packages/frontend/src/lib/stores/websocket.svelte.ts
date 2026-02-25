import { WebSocketClient } from "../services/websocket-client.js";
import { notificationService } from "../services/notification-service.js";
import { agentsStore } from "./agents.svelte.js";
import { WS_URL } from "../config.js";

class WebSocketStore {
  connected = $state(false);
  private client: WebSocketClient | null = null;
  private unsubscribe: (() => void) | null = null;
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  connect(url: string): void {
    if (this.client) return;

    this.client = new WebSocketClient(url);

    this.unsubscribe = this.client.onEvent((event) => {
      agentsStore.handleEvent(event);

      // Trigger notifications on state change
      if (event.type === "session:state_changed") {
        const session = agentsStore.getSession(event.sessionId);
        const projectName = session?.projectName ?? event.sessionId.slice(0, 8);
        notificationService.notifyStateChange(event.sessionId, projectName, event.current);
      }
    });

    this.client.connect();

    // Poll connection state
    this.pollTimer = setInterval(() => {
      this.connected = this.client?.connected ?? false;
    }, 500);
  }

  disconnect(): void {
    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
    this.unsubscribe?.();
    this.client?.disconnect();
    this.client = null;
    this.connected = false;
  }

  subscribe(sessionId: string): void {
    this.client?.send({ type: "subscribe:session", sessionId });
  }

  unsubscribeSession(sessionId: string): void {
    this.client?.send({ type: "unsubscribe:session", sessionId });
  }
}

export const wsStore = new WebSocketStore();

// Auto-connect in the browser (skip in mock mode)
if (typeof window !== "undefined" && !new URLSearchParams(window.location.search).has("mock")) {
  wsStore.connect(WS_URL);
}
