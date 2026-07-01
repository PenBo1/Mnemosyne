import { ipc, ipcVoid } from "@/infrastructure/api";
import type { SubAgentInfo } from "@/shared/types";

// ── Sub-Agent IPC 服务 ───────────────────────────────────
//
// 包装三个 IPC 调用，对应 src-tauri/src/ipc/commands/sub_agent.rs。
// 注意：前端使用 camelCase 参数键（Tauri 自动转换为 Rust 的 snake_case）。

/** 列出指定会话下的所有子 Agent（session_id 即 parent_thread_id）。 */
export async function listSubAgents(sessionId: string): Promise<SubAgentInfo[]> {
  return ipc<SubAgentInfo[]>("sub_agent_list", { sessionId });
}

/** 查询单个子 Agent 的最新元信息。 */
export async function getSubAgent(taskId: string): Promise<SubAgentInfo> {
  return ipc<SubAgentInfo>("sub_agent_get", { taskId });
}

/** 取消运行中的子 Agent（对已完成或不存在的 task_id 幂等成功）。 */
export async function cancelSubAgent(taskId: string): Promise<void> {
  await ipcVoid("sub_agent_cancel", { taskId });
}
