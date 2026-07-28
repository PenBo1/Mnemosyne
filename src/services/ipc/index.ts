/**
 * ═══════════════════════════════════════════════════════════════════════════
 * IPC 通信服务 - 提供与 Rust 后端的通信接口
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { invoke } from "@tauri-apps/api/core";

// ── 类型定义 ────────────────────────────────────────────────────────────────

export interface IpcResponse<T> {
  status: number;
  code: string;
  message: string;
  data: T | null;
}

// ── 常量定义 ────────────────────────────────────────────────────────────────

// IPC 成功状态码集合（与 Rust 端 shared::error::status 对应）
// 0 = OK（成功）
// 1 = CREATED（资源已创建）
// 2 = UPDATED（资源已更新）
// 3 = DELETED（资源已删除）
// 4 = NO_CONTENT（成功但无返回数据）
// 5 = ACCEPTED（请求已接受，异步处理中）
const SUCCESS_CODES = new Set([0, 1, 2, 3, 4, 5]);

// ── IPC 调用 ────────────────────────────────────────────────────────────────

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