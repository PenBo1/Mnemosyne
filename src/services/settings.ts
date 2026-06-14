import { ipc, ipcVoid } from "@/lib/ipc";

export async function setWindowTheme(theme: string): Promise<void> {
  return ipcVoid("set_window_theme", { theme });
}

export async function getLogLevel(): Promise<string> {
  return ipc<string>("get_log_level");
}

export async function setLogLevel(level: string): Promise<void> {
  return ipcVoid("set_log_level", { level });
}
