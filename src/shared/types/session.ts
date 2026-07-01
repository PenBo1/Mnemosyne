// ── Session & Agent Events ─────────────────────────────────

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

// SafetyGate 确认请求（与后端 AgentEvent::ConfirmationRequired 对齐）
export interface PendingConfirmation {
  toolCallId: string;
  tool: string;
  description: string;
  details: string;
  riskLevel: string;
}


