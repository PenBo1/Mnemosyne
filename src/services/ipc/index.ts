import { invoke } from "@tauri-apps/api/core";

export interface IpcResponse<T> {
  status: number;
  code: string;
  message: string;
  data: T | null;
}

const SUCCESS_CODES = new Set([0, 1, 2, 3, 4, 5]);

export async function ipc<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const response = await invoke<IpcResponse<T>>(command, args);
  if (!SUCCESS_CODES.has(response.status)) {
    throw new Error(response.message || `IPC error [${response.status}|${response.code}]`);
  }
  if (response.data === null || response.data === undefined) {
    throw new Error(`IPC command "${command}" returned null data`);
  }
  return response.data;
}

export async function ipcVoid(command: string, args?: Record<string, unknown>): Promise<void> {
  const response = await invoke<IpcResponse<void>>(command, args);
  if (!SUCCESS_CODES.has(response.status)) {
    throw new Error(response.message || `IPC error [${response.status}|${response.code}]`);
  }
}
