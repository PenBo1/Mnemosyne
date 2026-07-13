// ── AI 统计与审计事件 ──────────────────────────────────────────
//
// 对应 Rust IPC:
// - get_ai_stats        → AiStats
// - audit_event_stats   → AuditEventStats
// - audit_events_query  → AuditEventRow[]

/** AI 指标聚合(从 messages 表中 assistant 消息统计)。 */
export interface AiStats {
  /** LLM 调用次数(model 非空的 assistant 消息数) */
  llmCalls: number;
  /** token 总量(input + output) */
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  /** 工具调用次数(tool_calls JSON 数组长度之和) */
  toolCalls: number;
  /** 按 provider+model 分组的用量 */
  modelUsage: ModelUsageRow[];
}

/** 单个模型的用量统计。 */
export interface ModelUsageRow {
  provider: string | null;
  model: string;
  calls: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

/** 审计事件聚合统计(供 Violations 卡片)。 */
export interface AuditEventStats {
  total: number;
  /** 被拒绝/拦截/超额的事件数 */
  denied: number;
  /** 安全相关事件数 */
  securityRelated: number;
  byType: AuditTypeCount[];
}

export interface AuditTypeCount {
  eventType: string;
  count: number;
}

/** 一行审计事件(最近事件列表展示用)。 */
export interface AuditEventRow {
  id: string;
  eventType: string;
  operation: string | null;
  workspaceId: string | null;
  isDenied: boolean;
  isSecurityRelated: boolean;
  /** 完整 SecurityEvent JSON */
  payload: unknown;
  recordedAt: string;
}
