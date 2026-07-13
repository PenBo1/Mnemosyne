// ── Session & Agent Events ─────────────────────────────────
//
// 跨模块共享的会话与 Agent 事件类型。
// 历史位置：原 src/features/chat/types/session.ts。
// 上移到 @/types/ 以打破 chat ↔ agent 循环依赖（agent 模块需要引用这些
// 类型，但 chat 模块不应被 agent 反向依赖）。

export interface Session {
  id: string;
  novel_id: string | null;
  workspace_id: string | null;
  session_type: "chat" | "pipeline" | "review";
  title: string;
  summary: string | null;
  message_count: number;
  input_tokens: number;
  output_tokens: number;
  cost: number;
  status: "active" | "paused" | "completed" | "archived";
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  tool_calls: string | null;
  tool_results: string | null;
  token_count: number | null;
  created_at: string;
}

export interface AgentEvent {
  type:
    | "TurnStarted"
    | "StreamDelta"
    | "ReasoningDelta"
    | "ToolCallBegin"
    | "ToolCallDelta"
    | "ToolCallEnd"
    | "TurnCompleted"
    | "Error"
    | "CompactionTriggered"
    | "ConfirmationRequired";
  session_id: string;
  content?: string;
  tool_call_id?: string;
  tool?: string;
  args?: string;
  args_delta?: string;
  output?: string;
  is_error?: boolean;
  input_tokens?: number;
  output_tokens?: number;
  error?: string;
  // ConfirmationRequired 专属字段
  step_id?: number;
  description?: string;
  details?: string;
  risk_level?: string;
}

// Tool approval request from Rust agent engine
export interface PendingConfirmation {
  toolCallId: string;
  toolName: string;
  args: Record<string, unknown>;
}
