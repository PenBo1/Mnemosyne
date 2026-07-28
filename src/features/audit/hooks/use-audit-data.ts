// useAuditData —— 审计页数据加载 hook，封装 stats/events/histogram 的获取与刷新。
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  getAuditEventStats,
  getAuditHistogram,
  queryAuditEventsFiltered,
} from "../services";
import type {
  AuditEventFilter,
  AuditEventRow,
  AuditEventStats,
  AuditHistogramBucket,
  HistogramGranularity,
} from "../types";

export function useAuditData(granularity: HistogramGranularity, filter: AuditEventFilter) {
  const { t } = useI18n();
  const [stats, setStats] = useState<AuditEventStats | null>(null);
  const [events, setEvents] = useState<AuditEventRow[]>([]);
  const [histogram, setHistogram] = useState<AuditHistogramBucket[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const loadData = useCallback(async () => {
    setRefreshing(true);
    try {
      const [s, h, ev] = await Promise.all([
        getAuditEventStats(),
        getAuditHistogram(granularity),
        queryAuditEventsFiltered(filter),
      ]);
      setStats(s);
      setHistogram(h);
      setEvents(ev);
    } catch (e) {
      toast.error(`${t.audit.loadFailed}: ${String(e)}`);
    } finally {
      setRefreshing(false);
      setLoading(false);
    }
  }, [filter, granularity, t.audit.loadFailed]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  return { stats, events, histogram, loading, refreshing, setEvents, loadData };
}
