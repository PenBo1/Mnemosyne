// Audit & Security Interception 前端类型 —— 与 Rust 端 IPC 类型对齐。
//
// 对应 Rust IPC:
// - audit_events_query / audit_events_query_filtered → AuditEventRow[]
// - audit_event_stats → AuditEventStats
// - audit_event_histogram → AuditHistogramBucket[]
// - kernel_stats → KernelStats
// - approval_list_pending → ApprovalTokenDto[]
// - approval_stats → ApprovalStatsDto
// - approval_grant / approval_reject / approval_cleanup_expired → void / usize

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

export interface AuditEventStats {
  total: number;
  denied: number;
  securityRelated: number;
  byType: AuditTypeCount[];
}

export interface AuditTypeCount {
  eventType: string;
  count: number;
}

/** 过滤查询参数(对齐 Rust AuditEventFilter,所有字段可选) */
export interface AuditEventFilter {
  workspaceId?: string;
  /** LIKE 模糊匹配,便于按前缀查 fs_/git_ 等 */
  operation?: string;
  /** 精确匹配 */
  eventType?: string;
  /** RFC3339 字符串 */
  since?: string;
  until?: string;
  onlyDenied?: boolean;
  onlySecurity?: boolean;
  offset?: number;
  limit?: number;
}

/** 直方图桶 —— bucket 为时间起始(RFC3339),count 为事件数,denied 为拒绝数 */
export interface AuditHistogramBucket {
  bucket: string;
  count: number;
  denied: number;
}

export type HistogramGranularity = "hour" | "day" | "month";

/** Kernel 全局统计 —— 反映 SecurityKernel 内部各子系统规模 */
export interface KernelStats {
  policyWorkspaces: number;
  policyUsers: number;
  policyTemporary: number;
  ratePolicies: number;
  rateRecords: number;
  approvalPending: number;
  approvalApproved: number;
  resourceQuotas: number;
  resourceUsageEntries: number;
  permissionSessions: number;
  auditEntries: number;
}

/** Approval Token DTO(前端展示用,已剔除内部字段) */
export interface ApprovalTokenDto {
  id: string;
  workspace: string;
  riskLevel: string;
  createdAt: string;
  expire: string;
  remainingSeconds: number;
  isExpired: boolean;
}

export interface ApprovalStatsDto {
  pending: number;
  approved: number;
  rejected: number;
  expired: number;
  oldestPendingAge: number | null;
}

/** Tauri 事件 payload —— 由 security://event 推送 */
export interface SecurityEventPayload {
  eventId: string;
  eventType: string;
  recordedAt: string;
  operation: string | null;
  workspaceId: string | null;
  isDenied: boolean;
  isSecurityRelated: boolean;
  /** 完整 SecurityEvent JSON */
  event: unknown;
}

/** SecurityEvent 事件类型 —— 用于 UI 显示分组 */
export const SECURITY_EVENT_TYPES = [
  "operation_start",
  "operation_complete",
  "approval_requested",
  "approval_granted",
  "approval_rejected",
  "policy_denied",
  "rate_limited",
  "resource_exceeded",
  "plugin_loaded",
  "plugin_unloaded",
  "quota_set",
  "override_applied",
  "override_expired",
] as const;

export type SecurityEventType = typeof SECURITY_EVENT_TYPES[number];

/** 事件类型分类(用于颜色编码) */
export const DENIED_EVENT_TYPES: ReadonlySet<string> = new Set([
  "policy_denied",
  "rate_limited",
  "resource_exceeded",
  "approval_rejected",
]);

export const SECURITY_EVENT_TYPES_SET: ReadonlySet<string> = new Set([
  "approval_requested",
  "approval_rejected",
  "policy_denied",
  "rate_limited",
  "resource_exceeded",
]);
