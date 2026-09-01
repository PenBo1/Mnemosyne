/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Evaluation IPC Service - 评估系统 IPC 调用封装
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * 为评估器提供通用的 IPC 调用封装，避免在评估器中直接导入 invoke。
 */

import { ipc } from "@/services/ipc";

/**
 * 通用的 IPC 命令调用
 *
 * 用于评估系统中需要动态调用不同命令的场景。
 *
 * @param commandName - IPC 命令名称
 * @param args - 命令参数
 * @returns 命令返回的数据
 * @throws 当命令调用失败时抛出错误
 *
 * @example
 * ```typescript
 * const data = await callIpcCommand("some_command", {
 *   sessionId: "abc",
 *   workspaceId: "def"
 * });
 * ```
 */
export async function callIpcCommand(
  commandName: string,
  args: Record<string, unknown>
): Promise<unknown> {
  return ipc<unknown>(commandName, args);
}