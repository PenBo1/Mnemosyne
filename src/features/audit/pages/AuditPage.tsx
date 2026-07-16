// AuditPage —— 审计日志可视化主页面。
//
// 功能:
// 1. 顶部统计卡片(总数/拒绝/安全/按类型)
// 2. 直方图(按时间桶聚合,bar chart)
// 3. 过滤器(workspace/operation/eventType/only_denied/only_security/time range)
// 4. 事件列表(点击查看 payload 详情)
// 5. 实时事件徽章(显示未读 denied 数,点击查看新增)
//
// 与 AgentAuditSettings 的区别:
// - AgentAuditSettings 是设置子页(基础只读视图),位于 settings 下
// - AuditPage 是独立的功能页(完整可视化),位于主侧边栏的 Audit 入口
//
// 数据来源:
// - getAuditEventStats + getAuditHistogram + queryAuditEventsFiltered
// - useAuditStream(实时订阅 security://event)

import { useCallback, useEffect, useMemo, useState } from "react";
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
  EyeIcon,
  ActivityIcon,
} from "lucide-react";
import { toast } from "sonner";
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
import {
  getAuditEventStats,
  getAuditHistogram,
  queryAuditEventsFiltered,
} from "../services";
import { useAuditStream } from "../hooks/useAuditStream";
import type {
  AuditEventFilter,
  AuditEventRow,
  AuditEventStats,
  AuditHistogramBucket,
  HistogramGranularity,
  SecurityEventPayload,
} from "../types";
import { DENIED_EVENT_TYPES, SECURITY_EVENT_TYPES_SET } from "../types";
import { EventDetailDrawer } from "../components/EventDetailDrawer";
import { PendingApprovalsPanel } from "../components/PendingApprovalsPanel";

const EVENT_TYPE_COLOR: Record<string, string> = {
  operation_start: "text-emerald-500",
  operation_complete: "text-emerald-600",
  approval_requested: "text-amber-500",
  approval_granted: "text-emerald-500",
  approval_rejected: "text-rose-500",
  policy_denied: "text-rose-600",
  rate_limited: "text-orange-500",
  resource_exceeded: "text-orange-600",
  plugin_loaded: "text-blue-500",
  plugin_unloaded: "text-blue-400",
  quota_set: "text-purple-500",
  override_applied: "text-purple-400",
  override_expired: "text-purple-300",
};

const GRANULARITY_OPTIONS: ReadonlyArray<{ value: HistogramGranularity }> = [
  { value: "hour" },
  { value: "day" },
  { value: "month" },
];

const LIMIT_OPTIONS = [20, 50, 100, 200, 500] as const;

export function AuditPage() {
  const { t } = useI18n();
  const [stats, setStats] = useState<AuditEventStats | null>(null);
  const [events, setEvents] = useState<AuditEventRow[]>([]);
  const [histogram, setHistogram] = useState<AuditHistogramBucket[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  // 过滤状态
  const [filter, setFilter] = useState<AuditEventFilter>({
    limit: 50,
    offset: 0,
  });
  const [granularity, setGranularity] = useState<HistogramGranularity>("day");

  // 详情抽屉
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);

  // 实时事件流(全局,不受过滤影响)
  const { events: liveEvents, unreadDeniedCount, clearUnread, isConnected } = useAuditStream(true);

  const selectedEvent = useMemo(() => {
    if (!selectedEventId) return null;
    return events.find((e) => e.id === selectedEventId) ?? null;
  }, [events, selectedEventId]);

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

  const handleRefresh = () => {
    void loadData();
  };

  const handleApplyLiveEvent = (live: SecurityEventPayload) => {
    // 把实时事件作为 AuditEventRow 注入到列表头(简化展示)
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

      {/* 统计卡片 */}
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

      {/* 按类型分布 */}
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

      {/* 直方图 */}
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

      {/* 过滤器 */}
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

      {/* 事件列表 */}
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
                  onClick={() => setSelectedEventId(e.id)}
                  isSelected={e.id === selectedEventId}
                  labels={{
                    viewDetails: t.audit.viewDetails,
                    workspace: t.audit.workspace,
                    denied: t.audit.denied,
                    security: t.audit.security,
                  }}
                />
              ))}
            </div>
          )}
        </ScrollArea>
      </div>

      {/* 详情抽屉 */}
      <EventDetailDrawer
        event={selectedEvent}
        open={selectedEvent !== null}
        onClose={() => setSelectedEventId(null)}
      />

      {/* 安全拦截面板(pending approvals) */}
      <PendingApprovalsPanel />
    </PageContainer>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: number;
  subtitle?: string;
  subtitleClass?: string;
}

function StatCard({ icon, label, value, subtitle, subtitleClass }: StatCardProps) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex items-center gap-2">
        {icon}
        <div className="text-xs text-muted-foreground">{label}</div>
      </div>
      <div className="mt-2 text-2xl font-semibold">{value.toLocaleString()}</div>
      {subtitle && (
        <div className={`text-xs ${subtitleClass ?? "text-muted-foreground"}`}>
          {subtitle}
        </div>
      )}
    </div>
  );
}

interface EventRowProps {
  event: AuditEventRow;
  onClick: () => void;
  isSelected: boolean;
  labels: {
    viewDetails: string;
    workspace: string;
    denied: string;
    security: string;
  };
}

function EventRow({ event, onClick, isSelected, labels }: EventRowProps) {
  const colorClass = EVENT_TYPE_COLOR[event.eventType] ?? "text-muted-foreground";
  const isDenied = DENIED_EVENT_TYPES.has(event.eventType) || event.isDenied;
  const isSecurity = SECURITY_EVENT_TYPES_SET.has(event.eventType) || event.isSecurityRelated;

  return (
    <button
      onClick={onClick}
      className={`flex w-full items-center gap-3 rounded-md border px-3 py-2 text-left transition hover:bg-accent ${
        isSelected ? "border-primary bg-accent" : "border-border"
      }`}
    >
      <div className={`flex size-8 items-center justify-center rounded-full ${colorClass}`}>
        {isDenied ? (
          <ShieldXIcon className="size-4" />
        ) : isSecurity ? (
          <ShieldAlertIcon className="size-4" />
        ) : (
          <ShieldIcon className="size-4" />
        )}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className={`text-sm font-medium ${colorClass}`}>
            {event.eventType}
          </span>
          {event.operation && (
            <Badge variant="outline" className="font-mono text-xs">
              {event.operation}
            </Badge>
          )}
          {isDenied && (
            <Badge variant="destructive" className="text-xs">
              {labels.denied}
            </Badge>
          )}
          {isSecurity && !isDenied && (
            <Badge variant="secondary" className="text-xs text-amber-500">
              {labels.security}
            </Badge>
          )}
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">
          {event.workspaceId ? (
            <>{labels.workspace}: {event.workspaceId.slice(0, 8)}…</>
          ) : (
            "—"
          )}
        </div>
      </div>
      <div className="text-right text-xs text-muted-foreground">
        <div>{formatTime(event.recordedAt)}</div>
        <div className="mt-0.5 flex items-center justify-end gap-1 text-[10px] opacity-60">
          <EyeIcon className="size-3" />
          {labels.viewDetails}
        </div>
      </div>
    </button>
  );
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffMin = Math.floor(diffMs / 60_000);
  if (diffMin < 1) return "now";
  if (diffMin < 60) return `${diffMin}m`;
  const diffH = Math.floor(diffMin / 60);
  if (diffH < 24) return `${diffH}h`;
  return d.toLocaleDateString();
}
