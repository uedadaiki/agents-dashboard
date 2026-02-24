import { notificationService, type NotificationSettings } from "../services/notification-service.js";

class NotificationsStore {
  settings = $state<NotificationSettings>(notificationService.getSettings());
  permissionState = $state<NotificationPermission | "unsupported">(
    notificationService.getPermissionState(),
  );

  toggle(key: "enabled" | "soundEnabled" | "desktopEnabled"): void {
    notificationService.updateSettings({ [key]: !this.settings[key] });
    this.settings = notificationService.getSettings();
  }

  toggleNotifyOn(key: "idle" | "permissionWaiting" | "error"): void {
    notificationService.updateNotifyOn({ [key]: !this.settings.notifyOn[key] });
    this.settings = notificationService.getSettings();
  }

  async requestPermission(): Promise<void> {
    await notificationService.requestPermission();
    this.permissionState = notificationService.getPermissionState();
  }
}

export const notificationsStore = new NotificationsStore();
