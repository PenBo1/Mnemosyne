import { ipc } from "@/services/ipc";
import type {
  AiStats,
  AuditEventStats,
  AuditEventRow,
} from "@/features/stats/types";

// ── API Functions ──────────────────────────────────────────

/** AI 指标聚合:token 总量 / LLM 调用数 / 工具调用数 / 模型用量分组。 */
export async function getAiStats(): Promise<AiStats> {
  return ipc<AiStats>("get_ai_stats");
}

/** 审计事件聚合统计:总数 / 拒绝数 / 安全相关数 / 按类型分组。 */
export async function getAuditEventStats(): Promise<AuditEventStats> {
  return ipc<AuditEventStats>("audit_event_stats");
}

/** 查询最近审计事件(按 recordedAt 倒序)。 */
export async function getAuditEvents(limit = 50): Promise<AuditEventRow[]> {
  return ipc<AuditEventRow[]>("audit_events_query", { limit });
}
