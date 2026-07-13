import { ipc, ipcVoid } from "@/services/ipc";

/** 日志文件信息 */
export interface LogFileInfo {
  name: string;
  size: number;
  modified: string | null;
}

/** 列出所有日志文件（按日期降序） */
export async function listLogFiles(): Promise<LogFileInfo[]> {
  return ipc<LogFileInfo[]>("list_log_files");
}

/** 读取指定日志文件内容（超过 2MB 时只返回尾部） */
export async function readLogFile(name: string): Promise<string> {
  return ipc<string>("read_log_file", { name });
}

/** 清空指定日志文件 */
export async function clearLogFile(name: string): Promise<void> {
  return ipcVoid("clear_log_file", { name });
}
