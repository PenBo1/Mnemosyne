import { ipc } from "@/infrastructure/api";

const STORAGE_KEY = "mnemosyne-notifications-enabled";

export function isNotificationsEnabled(): boolean {
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored !== "false";
}

export function setNotificationsEnabled(enabled: boolean): void {
  localStorage.setItem(STORAGE_KEY, String(enabled));
}

export async function sendNotification(title: string, body: string): Promise<void> {
  if (!isNotificationsEnabled()) return;
  await ipc<void>("send_notification", { payload: { title, body } });
}
