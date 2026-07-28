/**
 * Process Monitor IPC 服务
 */

import { invoke } from "@tauri-apps/api/core";
import type { ProcessInfo, SystemResourceSummary } from "./types";

/** 获取进程列表 */
export async function getProcessList(): Promise<ProcessInfo[]> {
  const response = await invoke<{ data: ProcessInfo[] }>("process_monitor_list");
  return response.data;
}

/** 获取系统资源摘要 */
export async function getResourceSummary(): Promise<SystemResourceSummary> {
  const response = await invoke<{ data: SystemResourceSummary }>(
    "process_monitor_summary"
  );
  return response.data;
}