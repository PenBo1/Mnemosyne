import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import {
  getAiStats,
  getAuditEventStats,
  getAuditEvents,
} from "@/features/stats/services";
import type {
  AiStats,
  AuditEventStats,
  AuditEventRow,
} from "@/features/stats/types";

export interface AiAnalyticsData {
  aiStats: AiStats | null;
  auditStats: AuditEventStats | null;
  auditEvents: AuditEventRow[];
}

/**
 * 加载仪表盘 AI 分析数据。
 *
 * 数据为全局聚合(非 session 级别):
 * - aiStats:从 messages 表统计 token / LLM 调用 / 工具调用 / 模型用量
 * - auditStats / auditEvents:从 audit_events 表统计 SecurityKernel 审计事件
 */
export function useAiAnalytics() {
  const [data, setData] = useState<AiAnalyticsData>({
    aiStats: null,
    auditStats: null,
    auditEvents: [],
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [aiStats, auditStats, auditEvents] = await Promise.all([
        getAiStats(),
        getAuditEventStats(),
        getAuditEvents(50),
      ]);
      setData({ aiStats, auditStats, auditEvents });
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load AI analytics";
      setError(msg);
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return { ...data, loading, error, reload: load };
}
