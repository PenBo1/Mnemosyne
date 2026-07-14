// Audit & Security Interception 前端服务 —— IPC 命令封装。
//
// 暴露给 hooks/components 调用。所有函数返回 Promise<T>。

import { ipc, ipcVoid } from "@/services/ipc";
import type {
  AuditEventFilter,
  AuditEventRow,
  AuditEventStats,
  AuditHistogramBucket,
  HistogramGranularity,
  KernelStats,
  ApprovalTokenDto,
  ApprovalStatsDto,
} from "./types";

/** 查询最近审计事件(按 recorded_at 倒序) */
export function getAuditEvents(limit = 50): Promise<AuditEventRow[]> {
  return ipc<AuditEventRow[]>("audit_events_query", { limit });
}

/** 审计事件聚合统计(总数/拒绝数/安全相关数/按类型分组) */
export function getAuditEventStats(): Promise<AuditEventStats> {
  return ipc<AuditEventStats>("audit_event_stats");
}

/** 按过滤条件查询审计事件 */
export function queryAuditEventsFiltered(
  filter: AuditEventFilter,
): Promise<AuditEventRow[]> {
  return ipc<AuditEventRow[]>("audit_events_query_filtered", { filter });
}

/** 直方图(granularity ∈ {hour, day, month}) */
export function getAuditHistogram(
  granularity: HistogramGranularity,
  since?: string,
  until?: string,
): Promise<AuditHistogramBucket[]> {
  return ipc<AuditHistogramBucket[]>("audit_event_histogram", {
    granularity,
    since: since ?? null,
    until: until ?? null,
  });
}

/** Kernel 全局统计 */
export function getKernelStats(): Promise<KernelStats> {
  return ipc<KernelStats>("kernel_stats");
}

/** 列出 pending approval tokens */
export function listPendingApprovals(): Promise<ApprovalTokenDto[]> {
  return ipc<ApprovalTokenDto[]>("approval_list_pending");
}

/** Approval 统计 */
export function getApprovalStats(): Promise<ApprovalStatsDto> {
  return ipc<ApprovalStatsDto>("approval_stats");
}

/** 批准 pending approval */
export async function grantApproval(approvalId: string, approvedBy: string): Promise<void> {
  await ipcVoid("approval_grant", { approvalId, approvedBy });
}

/** 拒绝 pending approval */
export async function rejectApproval(
  approvalId: string,
  rejectedBy: string,
  reason: string,
): Promise<void> {
  await ipcVoid("approval_reject", { approvalId, rejectedBy, reason });
}

/** 清理过期的 pending approvals */
export function cleanupExpiredApprovals(): Promise<number> {
  return ipc<number>("approval_cleanup_expired");
}
