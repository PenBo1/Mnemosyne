/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AuditPage - 审计日志可视化主页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  RefreshCwIcon,
  ShieldIcon,
  ShieldXIcon,
  ShieldAlertIcon,
  FilterIcon,
  ActivityIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { LoadingState } from "@/components/shared/state";
import { useAuditStream } from "../hooks/useAuditStream";
import { useAuditData } from "../hooks/use-audit-data";
import type {
  AuditEventFilter,
  AuditEventRow,
  HistogramGranularity,
  SecurityEventPayload,
} from "../types";
import { StatCard } from "../components/stat-card";
import { EventRow, EVENT_TYPE_COLOR } from "../components/event-row";
import { EventDetailDrawer } from "../components/EventDetailDrawer";
import { PendingApprovalsPanel } from "../components/PendingApprovalsPanel";

// ── 常量配置 ────────────────────────────────────────────────────────────────

const GRANULARITY_OPTIONS: ReadonlyArray<{ value: HistogramGranularity }> = [
  { value: "hour" },
  { value: "day" },
  { value: "month" },
];

const LIMIT_OPTIONS = [20, 50, 100, 200, 500] as const;

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 审计日志可视化主页面，展示统计数据、直方图、事件列表和实时事件流
 */
export function AuditPage() {
  const { t } = useI18n();

  // ── 状态管理 ──────────────────────────────────────────────────────────────

  // 过滤状态
  const [filter, setFilter] = useState<AuditEventFilter>({
    limit: 50,
    offset: 0,
  });
  const [granularity, setGranularity] = useState<HistogramGranularity>("day");

  // 详情抽屉
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);

  // ── 数据加载 ──────────────────────────────────────────────────────────────

  // 实时事件流(全局,不受过滤影响)
  const { events: liveEvents, unreadDeniedCount, clearUnread, isConnected } = useAuditStream(true);

  // 审计数据(stats/events/histogram + 加载/刷新)
  const { stats, events, histogram, loading, refreshing, setEvents, loadData } = useAuditData(granularity, filter);

  const selectedEvent = useMemo(() => {
    if (!selectedEventId) return null;
    return events.find((e) => e.id === selectedEventId) ?? null;
  }, [events, selectedEventId]);

  // 事件行标签：所有行共享同一引用，配合 memo 避免每行重渲染
  const eventRowLabels = useMemo(() => ({
    viewDetails: t.audit.viewDetails,
    workspace: t.audit.workspace,
    denied: t.audit.denied,
    security: t.audit.security,
  }), [t.audit.viewDetails, t.audit.workspace, t.audit.denied, t.audit.security]);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  const handleRefresh = () => {
    void loadData();
  };

  /**
   * 将实时事件注入到列表头
   */
  const handleApplyLiveEvent = (live: SecurityEventPayload) => {
    const row: AuditEventRow = {
      id: live.eventId,
      eventType: live.eventType,
      operation: live.operation,
      workspaceId: live.workspaceId,
      isDenied: live.isDenied,
      isSecurityRelated: live.isSecurityRelated,
      payload: live.event,
      recordedAt: live.recordedAt,
    };
    setEvents((prev) => [row, ...prev].slice(0, 200));
  };

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  if (loading || !stats) {
    return (
      <PageContainer>
        <LoadingState label={t.audit.loading} />
      </PageContainer>
    );
  }

  const deniedRate = stats.total > 0 ? (stats.denied / stats.total) * 100 : 0;
  const securityRate = stats.total > 0 ? (stats.securityRelated / stats.total) * 100 : 0;
  const maxBucketCount = Math.max(1, ...histogram.map((b) => b.count));

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <ShieldIcon className="size-5" />
            {t.audit.title}
          </PageTitle>
          <PageDescription>{t.audit.description}</PageDescription>
        </PageHeading>
        <PageActions>
          {isConnected && (
            <Badge variant="outline" className="text-emerald-500">
              <ActivityIcon className="size-3" />
              {t.audit.live}
            </Badge>
          )}
          {unreadDeniedCount > 0 && (
            <Button
              size="sm"
              variant="destructive"
              onClick={() => {
                // 把所有实时事件合并进主列表
                liveEvents.slice().reverse().forEach(handleApplyLiveEvent);
                clearUnread();
              }}
            >
              <ShieldAlertIcon className="size-4" />
              {unreadDeniedCount} {t.audit.newAlerts}
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={handleRefresh} disabled={refreshing}>
            <RefreshCwIcon className={refreshing ? "size-4 animate-spin" : "size-4"} />
            {t.audit.refresh}
          </Button>
        </PageActions>
      </PageHeader>

      {/* ── 统计卡片 ────────────────────────────────────────────────────────── */}
      <div className="grid grid-cols-2 gap-4 px-4 py-4 sm:grid-cols-4">
        <StatCard
          icon={<ShieldIcon className="size-5 text-blue-500" />}
          label={t.audit.totalEvents}
          value={stats.total}
        />
        <StatCard
          icon={<ShieldXIcon className="size-5 text-rose-500" />}
          label={t.audit.deniedEvents}
          value={stats.denied}
          subtitle={`${deniedRate.toFixed(1)}%`}
          subtitleClass="text-rose-500"
        />
        <StatCard
          icon={<ShieldAlertIcon className="size-5 text-amber-500" />}
          label={t.audit.securityEvents}
          value={stats.securityRelated}
          subtitle={`${securityRate.toFixed(1)}%`}
          subtitleClass="text-amber-500"
        />
        <StatCard
          icon={<ActivityIcon className="size-5 text-emerald-500" />}
          label={t.audit.eventTypes}
          value={stats.byType.length}
        />
      </div>

      {/* ── 按类型分布 ──────────────────────────────────────────────────────── */}
      <div className="px-4 py-2">
        <div className="mb-2 text-sm font-medium text-muted-foreground">
          {t.audit.byType}
        </div>
        <div className="flex flex-wrap gap-1.5">
          {stats.byType.map((tc) => (
            <Badge key={tc.eventType} variant="outline" className="gap-1">
              <span className={EVENT_TYPE_COLOR[tc.eventType] ?? "text-muted-foreground"}>
                {tc.eventType}
              </span>
              <span className="text-muted-foreground">·</span>
              <span className="font-mono">{tc.count}</span>
            </Badge>
          ))}
        </div>
      </div>

      {/* ── 直方图 ──────────────────────────────────────────────────────────── */}
      <div className="px-4 py-3">
        <div className="mb-2 flex items-center justify-between">
          <div className="text-sm font-medium text-muted-foreground">
            {t.audit.histogram}
          </div>
          <Select
            value={granularity}
            onValueChange={(v) => setGranularity(v as HistogramGranularity)}
          >
            <SelectTrigger size="sm" className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {GRANULARITY_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {t.audit.granularity[opt.value]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="flex h-32 items-end gap-1 border-b border-border pb-1">
          {histogram.length === 0 ? (
            <div className="flex h-full w-full items-center justify-center text-sm text-muted-foreground">
              {t.audit.noHistogramData}
            </div>
          ) : (
            histogram.map((b) => {
              const h = (b.count / maxBucketCount) * 100;
              const deniedH = b.count > 0 ? (b.denied / b.count) * 100 : 0;
              return (
                <div
                  key={b.bucket}
                  className="flex flex-1 flex-col items-center justify-end gap-1"
                  title={`${b.bucket}\n${t.audit.countLabel}: ${b.count}\n${t.audit.deniedLabel}: ${b.denied}`}
                >
                  <div className="text-[10px] text-muted-foreground">{b.count}</div>
                  <div
                    className="relative w-full overflow-hidden rounded-sm bg-emerald-500/30"
                    style={{ height: `${h}%` }}
                  >
                    {b.denied > 0 && (
                      <div
                        className="absolute bottom-0 w-full bg-rose-500/60"
                        style={{ height: `${deniedH}%` }}
                      />
                    )}
                  </div>
                  <div className="text-[9px] text-muted-foreground/60">{b.bucket.slice(5, 10)}</div>
                </div>
              );
            })
          )}
        </div>
      </div>

      {/* ── 过滤器 ──────────────────────────────────────────────────────────── */}
      <div className="border-t border-border px-4 py-3">
        <div className="mb-2 flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <FilterIcon className="size-4" />
          {t.audit.filters}
        </div>
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 md:grid-cols-4">
          <Input
            placeholder={t.audit.filterOperationPlaceholder}
            value={filter.operation ?? ""}
            onChange={(e) =>
              setFilter((f) => ({
                ...f,
                operation: e.target.value || undefined,
                offset: 0,
              }))
            }
          />
          <Input
            placeholder={t.audit.filterEventTypePlaceholder}
            value={filter.eventType ?? ""}
            onChange={(e) =>
              setFilter((f) => ({
                ...f,
                eventType: e.target.value || undefined,
                offset: 0,
              }))
            }
          />
          <Select
            value={filter.limit?.toString() ?? "50"}
            onValueChange={(v) =>
              setFilter((f) => ({ ...f, limit: Number(v), offset: 0 }))
            }
          >
            <SelectTrigger size="sm">
              <SelectValue placeholder={t.audit.limit} />
            </SelectTrigger>
            <SelectContent>
              {LIMIT_OPTIONS.map((n) => (
                <SelectItem key={n} value={n.toString()}>
                  {n}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="flex gap-1">
            <Button
              size="sm"
              variant={filter.onlyDenied ? "default" : "outline"}
              className="flex-1"
              onClick={() =>
                setFilter((f) => ({
                  ...f,
                  onlyDenied: !f.onlyDenied,
                  offset: 0,
                }))
              }
            >
              <ShieldXIcon className="size-3" />
              {t.audit.deniedOnly}
            </Button>
            <Button
              size="sm"
              variant={filter.onlySecurity ? "default" : "outline"}
              className="flex-1"
              onClick={() =>
                setFilter((f) => ({
                  ...f,
                  onlySecurity: !f.onlySecurity,
                  offset: 0,
                }))
              }
            >
              <ShieldAlertIcon className="size-3" />
              {t.audit.securityOnly}
            </Button>
          </div>
        </div>
      </div>

      {/* ── 事件列表 ────────────────────────────────────────────────────────── */}
      <div className="flex-1 px-4 pb-4">
        <ScrollArea className="h-[calc(100vh-560px)] min-h-[300px]">
          {events.length === 0 ? (
            <div className="flex h-full items-center justify-center py-12 text-sm text-muted-foreground">
              {t.audit.noEvents}
            </div>
          ) : (
            <div className="space-y-1">
              {events.map((e) => (
                <EventRow
                  key={e.id}
                  event={e}
                  onSelect={setSelectedEventId}
                  isSelected={e.id === selectedEventId}
                  labels={eventRowLabels}
                />
              ))}
            </div>
          )}
        </ScrollArea>
      </div>

      {/* ── 详情抽屉 ──────────────────────────────────────────────────────── */}
      <EventDetailDrawer
        event={selectedEvent}
        open={selectedEvent !== null}
        onClose={() => setSelectedEventId(null)}
      />

      {/* ── 安全拦截面板 ──────────────────────────────────────────────────── */}
      <PendingApprovalsPanel />
    </PageContainer>
  );
}