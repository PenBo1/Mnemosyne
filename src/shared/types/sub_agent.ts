// ── Sub-Agent ─────────────────────────────────────────────
//
// 子 Agent 是主 Agent 通过 `spawn_subagent` 工具自主 spawn 的轻量 agent。
// 前端仅查询状态、查看详情、取消运行中的任务 — 不直接 spawn。
//
// 注意：Rust 后端字段为 snake_case，但 Tauri 自动转换为 camelCase，
// 因此前端类型使用 camelCase（task_id → taskId 等）。

export type SubAgentRole = "Researcher" | "Outliner" | "Critic" | "Default";

export type SubAgentStatus = "Pending" | "Running" | "Completed" | "Errored" | "Cancelled";

/**
 * 子 Agent 元信息（registry 条目快照）。
 *
 * 与 Rust 端 `SubAgentInfo` 对应 — 不含执行结果字段，
 * 结果通过 `spawn_subagent` 工具同步回传给主 Agent，不暴露给前端。
 */
export interface SubAgentInfo {
  taskId: string;
  role: SubAgentRole;
  task: string;
  status: SubAgentStatus;
  parentThreadId: string;
  depth: number;
  startedAt: string;
}

/**
 * 子 Agent 执行结果。
 *
 * 与 Rust 端 `SubAgentResult` 对应。当前 IPC 命令不直接返回此结构
 * （结果走工具回传），但保留类型以备后续扩展（如历史结果查询）。
 */
export interface SubAgentResult {
  taskId: string;
  role: SubAgentRole;
  status: SubAgentStatus;
  output: string;
  artifacts: string[];
  error: string | null;
  durationMs: number;
}
