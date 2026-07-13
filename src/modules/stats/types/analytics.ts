/** LLM 调用记录 */
export interface LlmCall {
  id: string;
  session_id: string;
  agent_role: string;
  model: string;
  provider: string;
  system_prompt: string | null;
  messages_json: string;
  tools_json: string | null;
  temperature: number | null;
  max_tokens: number | null;
  response_content: string | null;
  response_tool_calls: string | null;
  finish_reason: string | null;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  started_at: string;
  completed_at: string | null;
  latency_ms: number | null;
  status: string;
  error_message: string | null;
  metadata: string;
  created_at: string;
}

/** 工具执行记录 */
export interface ToolExecution {
  id: string;
  session_id: string;
  llm_call_id: string | null;
  tool_name: string;
  arguments_json: string;
  result_content: string | null;
  is_error: boolean;
  error_message: string | null;
  started_at: string;
  completed_at: string | null;
  duration_ms: number | null;
  sandbox_allowed: boolean;
  sandbox_violation: string | null;
  pve_blocked: boolean;
  metadata: string;
  created_at: string;
}

/** Token 用量统计 */
export interface TokenUsageStats {
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  tools: {
    total_calls: number;
    errors: number;
    sandbox_blocked: number;
    success_rate: number;
  };
  models: Array<{
    model: string;
    calls: number;
    input_tokens: number;
    output_tokens: number;
    avg_latency_ms: number | null;
  }>;
}

/** 沙箱违规记录 */
export interface SandboxViolation {
  id: string;
  session_id: string | null;
  violation_type: string;
  resource: string;
  action: string;
  rule_matched: string | null;
  tool_name: string | null;
  arguments_json: string | null;
  detected_at: string;
  created_at: string;
}

/** 单日活跃度 */
export interface DailyActivity {
  date: string;
  count: number;
}

/** 仪表盘统计数据 */
export interface StatsData {
  promptCount: number;
  novelCount: number;
  trendCount: number;
  totalWords: number;
}
