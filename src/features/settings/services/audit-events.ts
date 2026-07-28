// 审计事件服务 —— 封装 IPC 调用

import { ipc } from "@/services/ipc";

export interface AuditEventRow {
  id: number;
  eventType: string;
  operation: string | null;
  workspaceId: string | null;
  isDenied: boolean;
  isSecurityRelated: boolean;
  payload: string | null;
  recordedAt: string;
}

export interface AuditEventStats {
  total: number;
  denied: number;
  allowed: number;
  securityRelated: number;
  byType: { eventType: string; count: number }[];
}

export async function getAuditEventStats(): Promise<AuditEventStats> {
  return ipc<AuditEventStats>("audit_event_stats", {});
}

export async function getAuditEvents(limit: number): Promise<AuditEventRow[]> {
  return ipc<AuditEventRow[]>("audit_events_query", { limit });
}