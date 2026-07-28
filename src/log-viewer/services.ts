/**
 * Log Viewer IPC 服务
 */

import { invoke } from "@tauri-apps/api/core";

export interface LogFileInfo {
  name: string;
  size: number;
  modifiedAt: string;
}

/** 获取日志文件列表 */
export async function getLogFiles(): Promise<LogFileInfo[]> {
  const response = await invoke<{ data: LogFileInfo[] }>("list_log_files");
  return response.data;
}

/** 读取日志文件内容 */
export async function readLogFile(name: string): Promise<string> {
  const response = await invoke<{ data: string }>("read_log_file", { name });
  return response.data;
}

/** 清空日志文件 */
export async function clearLogFile(name: string): Promise<void> {
  await invoke("clear_log_file", { name });
}