// 工具调用协议 —— 纯逻辑内核
// 工具调用状态机：pending → approved/rejected → running → succeeded/failed

export type ToolCallStatus = "pending" | "approved" | "rejected" | "running" | "succeeded" | "failed";

export interface ToolCallState {
  id: string;
  name: string;
  status: ToolCallStatus;
  // TODO: 待补充
}

/**
 * 工具调用状态转移函数（纯函数）
 */
export function transitionToolCall(current: ToolCallState, _action: ToolCallAction): ToolCallState {
  // TODO: 实现状态机
  return current;
}

export type ToolCallAction =
  | { type: "approve" }
  | { type: "reject" }
  | { type: "start" }
  | { type: "succeed"; result: unknown }
  | { type: "fail"; error: string };
