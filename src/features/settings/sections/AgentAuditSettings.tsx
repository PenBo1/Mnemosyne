// Agent 审计设置页 —— 展示 audit_events 表数据,提供完整的操作追溯能力。
//
// 核心功能:
// 1. 统计区:总数 / 拒绝数 / 安全相关数
// 2. 最近事件列表(按 recordedAt 倒序)
// 3. 支持调整返回条数上限

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  RefreshCwIcon,
  ScrollIcon,
  ShieldAlertIcon,
  ShieldXIcon,
  ShieldCheckIcon,
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
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { LoadingState } from "@/components/shared/state";
import { getAuditEventStats, getAuditEvents } from "@/features/stats/services/ai-logs";
import type {
  AuditEventRow,
  AuditEventStats,
} from "@/features/stats/types/ai-logs";

function formatRelativeTime(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const mins = Math.floor(diffMs / (1000 * 60));
    if (mins < 1) return "刚刚";
    if (mins < 60) return `${mins} 分钟前`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours} 小时前`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days} 天前`;
    return `${Math.floor(days / 30)} 月前`;
  } catch {
    return iso;
  }
}

function formatPayload(payload: unknown): string {
  try {
    if (typeof payload === "string") return payload;
    return JSON.stringify(payload, null, 2);
  } catch {
    return String(payload);
  }
}

export function AgentAuditSettings() {
  const { t } = useI18n();
  const [events, setEvents] = useState<AuditEventRow[]>([]);
  const [stats, setStats] = useState<AuditEventStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [limit, setLimit] = useState(100);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [evs, s] = await Promise.all([
        getAuditEvents(limit),
        getAuditEventStats(),
      ]);
      setEvents(evs);
      setStats(s);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load audit events";
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, [limit]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  const deniedRate = stats && stats.total > 0
    ? `${((stats.denied / stats.total) * 100).toFixed(1)}%`
    : "0.0%";
  const securityRate = stats && stats.total > 0
    ? `${((stats.securityRelated / stats.total) * 100).toFixed(1)}%`
    : "0.0%";

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <ScrollIcon className="size-4 text-slate-500" />
            {t.settings.agentAudit.label}
          </PageTitle>
          <PageDescription>{t.settings.agentAudit.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={loadData}
          >
            <RefreshCwIcon className="size-3.5" />
            {t.settings.agentAudit.refresh}
          </Button>
        </PageActions>
      </PageHeader>

      {/* 1. 统计 */}
      <SettingsSection title={t.settings.agentAudit.stats} description={t.settings.agentAudit.hint}>
        <SettingsRow
          label={t.settings.agentAudit.statsTotal}
          description={t.settings.agentAudit.hint}
        >
          <Badge variant="default">{stats?.total ?? 0}</Badge>
        </SettingsRow>
        <SettingsRow label={t.settings.agentAudit.statsDenied}>
          <div className="flex items-center gap-2">
            <ShieldXIcon className="size-3.5 text-red-500" />
            <Badge variant="destructive">{stats?.denied ?? 0}</Badge>
            <span className="text-xs text-muted-foreground">{deniedRate}</span>
          </div>
        </SettingsRow>
        <SettingsRow label={t.settings.agentAudit.statsSecurity}>
          <div className="flex items-center gap-2">
            <ShieldAlertIcon className="size-3.5 text-amber-500" />
            <Badge variant="secondary">{stats?.securityRelated ?? 0}</Badge>
            <span className="text-xs text-muted-foreground">{securityRate}</span>
          </div>
        </SettingsRow>
        {stats && stats.byType.length > 0 && (
          <div className="px-4 py-3 border-t">
            <div className="mb-2 text-xs font-medium text-muted-foreground">By Type</div>
            <div className="flex flex-wrap gap-1.5">
              {stats.byType.map((b) => (
                <Badge key={b.eventType} variant="outline" className="text-xs">
                  {b.eventType}: {b.count}
                </Badge>
              ))}
            </div>
          </div>
        )}
      </SettingsSection>

      {/* 2. 最近事件 */}
      <SettingsSection title={t.settings.agentAudit.recentEvents}>
        <SettingsRow
          label={t.settings.agentAudit.limit}
          description={t.settings.agentAudit.limitHint}
        >
          <Select
            value={String(limit)}
            onValueChange={(v) => setLimit(Number(v))}
          >
            <SelectTrigger className="w-28">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="50">50</SelectItem>
              <SelectItem value="100">100</SelectItem>
              <SelectItem value="200">200</SelectItem>
              <SelectItem value="500">500</SelectItem>
              <SelectItem value="1000">1000</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
        {events.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t.settings.agentAudit.noEvents}
          </div>
        ) : (
          <ScrollArea className="h-[480px]">
            <div className="divide-y">
              {events.map((e) => (
                <div key={e.id} className="flex flex-col gap-1.5 px-4 py-2.5">
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="text-sm font-medium">{e.eventType}</span>
                        {e.operation && (
                          <Badge variant="outline" className="text-xs">
                            {e.operation}
                          </Badge>
                        )}
                        {e.isDenied && (
                          <Badge variant="destructive" className="text-xs">
                            {t.settings.agentAudit.isDenied}
                          </Badge>
                        )}
                        {e.isSecurityRelated && (
                          <Badge variant="secondary" className="text-xs">
                            {t.settings.agentAudit.isSecurity}
                          </Badge>
                        )}
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {formatRelativeTime(e.recordedAt)}
                        {e.workspaceId && ` · ${t.settings.agentAudit.workspace}: ${e.workspaceId}`}
                      </span>
                    </div>
                    {e.isDenied ? (
                      <ShieldXIcon className="size-4 text-red-500" />
                    ) : e.isSecurityRelated ? (
                      <ShieldAlertIcon className="size-4 text-amber-500" />
                    ) : (
                      <ShieldCheckIcon className="size-4 text-emerald-500" />
                    )}
                  </div>
                  {e.payload != null && (() => {
                    const formatted = formatPayload(e.payload);
                    return (
                      <pre className="overflow-x-auto rounded bg-muted/50 p-2 text-xs">
                        {formatted.slice(0, 800)}
                        {formatted.length > 800 ? "…" : ""}
                      </pre>
                    );
                  })()}
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </SettingsSection>
    </PageContainer>
  );
}
