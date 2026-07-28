// 短期记忆设置页 —— 按日期查看 Agent 的 session 级摘要。
//
// 核心功能:
// 1. 按日期选择查看当日所有 session 摘要
// 2. 显示统计信息(今日/7日总数)
// 3. 支持手动重新生成 session 摘要

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import {
  RefreshCwIcon,
  BrainIcon,
  SparklesIcon,
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
import {
  listShortTermMemoryByDate,
  listShortTermMemoryByRange,
  getShortTermMemoryStats,
  regenerateShortTermMemory,
} from "@/features/memory/services/short-term-memory";
import {
  parseKeyTopics,
  type ShortTermMemoryRow,
  type ShortTermMemoryStats,
} from "@/features/memory/types/short-term-memory";

function todayStr(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function yesterdayStr(): string {
  const d = new Date(Date.now() - 24 * 60 * 60 * 1000);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function last7DaysRange(): { start: string; end: string } {
  const end = todayStr();
  const d = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return { start: `${y}-${m}-${day}`, end };
}

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

type ViewMode = "date" | "range";

export function ShortTermMemorySettings() {
  const { t } = useI18n();
  const [rows, setRows] = useState<ShortTermMemoryRow[]>([]);
  const [stats, setStats] = useState<ShortTermMemoryStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [viewMode, setViewMode] = useState<ViewMode>("date");
  const [date, setDate] = useState(todayStr());
  const [rangeStart, setRangeStart] = useState(last7DaysRange().start);
  const [rangeEnd, setRangeEnd] = useState(last7DaysRange().end);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [data, s] = await Promise.all([
        viewMode === "date"
          ? listShortTermMemoryByDate(date)
          : listShortTermMemoryByRange(rangeStart, rangeEnd),
        getShortTermMemoryStats(),
      ]);
      setRows(data);
      setStats(s);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load short-term memory";
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, [viewMode, date, rangeStart, rangeEnd]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const handleRegenerate = useCallback(async (row: ShortTermMemoryRow) => {
    if (!window.confirm(t.settings.shortTermMemory.regenerateConfirm)) return;
    try {
      setActionLoading(row.id);
      await regenerateShortTermMemory(row.sessionId, row.bookId, row.agentRole);
      toast.success(t.settings.shortTermMemory.regeneratedToast);
      await loadData();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Regenerate failed";
      toast.error(msg);
    } finally {
      setActionLoading(null);
    }
  }, [loadData, t.settings.shortTermMemory.regenerateConfirm, t.settings.shortTermMemory.regeneratedToast]);

  const handleQuickDate = useCallback((preset: "today" | "yesterday" | "last7days") => {
    if (preset === "today") {
      setViewMode("date");
      setDate(todayStr());
    } else if (preset === "yesterday") {
      setViewMode("date");
      setDate(yesterdayStr());
    } else {
      setViewMode("range");
      const r = last7DaysRange();
      setRangeStart(r.start);
      setRangeEnd(r.end);
    }
  }, []);

  if (loading) {
    return (
      <PageContainer>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <BrainIcon className="size-4 text-violet-500" />
            {t.settings.shortTermMemory.label}
          </PageTitle>
          <PageDescription>{t.settings.shortTermMemory.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={loadData}
          >
            <RefreshCwIcon className="size-3.5" />
            {t.settings.shortTermMemory.refresh}
          </Button>
        </PageActions>
      </PageHeader>

      {/* 1. 统计 */}
      <SettingsSection title={t.settings.shortTermMemory.stats}>
        <SettingsRow label={t.settings.shortTermMemory.statsToday}>
          <Badge variant="default">{stats?.todayCount ?? 0}</Badge>
        </SettingsRow>
        <SettingsRow label={t.settings.shortTermMemory.stats7days}>
          <Badge variant="secondary">{stats?.last7DaysCount ?? 0}</Badge>
        </SettingsRow>
        <SettingsRow label={t.settings.shortTermMemory.description}>
          <Badge variant="outline">{stats?.total ?? 0}</Badge>
        </SettingsRow>
      </SettingsSection>

      {/* 2. 日期选择 */}
      <SettingsSection
        title={t.settings.shortTermMemory.dateSelect}
        description={t.settings.shortTermMemory.hint}
      >
        <SettingsRow label={t.settings.shortTermMemory.date} description={t.settings.shortTermMemory.dateHint}>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              onClick={() => handleQuickDate("today")}
            >
              {t.settings.shortTermMemory.today}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              onClick={() => handleQuickDate("yesterday")}
            >
              {t.settings.shortTermMemory.yesterday}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              onClick={() => handleQuickDate("last7days")}
            >
              {t.settings.shortTermMemory.last7days}
            </Button>
          </div>
        </SettingsRow>
        {viewMode === "date" ? (
          <SettingsRow label={t.settings.shortTermMemory.date}>
            <Input
              type="date"
              value={date}
              onChange={(e) => {
                setViewMode("date");
                setDate(e.target.value);
              }}
              className="h-8 w-44 text-xs"
            />
          </SettingsRow>
        ) : (
          <>
            <SettingsRow label="Start">
              <Input
                type="date"
                value={rangeStart}
                onChange={(e) => setRangeStart(e.target.value)}
                className="h-8 w-44 text-xs"
              />
            </SettingsRow>
            <SettingsRow label="End">
              <Input
                type="date"
                value={rangeEnd}
                onChange={(e) => setRangeEnd(e.target.value)}
                className="h-8 w-44 text-xs"
              />
            </SettingsRow>
          </>
        )}
      </SettingsSection>

      {/* 3. 当日摘要列表 */}
      <SettingsSection title={t.settings.shortTermMemory.list}>
        {rows.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t.settings.shortTermMemory.noEntries}
          </div>
        ) : (
          <ScrollArea className="h-[420px]">
            <div className="divide-y">
              {rows.map((r) => {
                const topics = parseKeyTopics(r.keyTopics);
                return (
                  <div key={r.id} className="flex flex-col gap-2 px-4 py-3">
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex min-w-0 flex-1 flex-col gap-1">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium">
                            {t.settings.shortTermMemory.sessionId}: {r.sessionId}
                          </span>
                          {r.agentRole && (
                            <Badge variant="outline">{r.agentRole}</Badge>
                          )}
                        </div>
                        <span className="text-xs text-muted-foreground">
                          {t.settings.shortTermMemory.createdAt}: {formatRelativeTime(r.createdAt)}
                          {" · "}
                          {t.settings.shortTermMemory.tokenCount}: {r.tokenCount}
                          {" · "}
                          {t.settings.shortTermMemory.messageCount}: {r.messageCount}
                          {r.bookId && (
                            <>
                              {" · "}
                              {t.settings.shortTermMemory.bookId}: {r.bookId}
                            </>
                          )}
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 gap-1.5 px-2 text-xs"
                        disabled={actionLoading === r.id}
                        onClick={() => handleRegenerate(r)}
                        title={t.settings.shortTermMemory.regenerate}
                      >
                        <SparklesIcon className="size-3.5" />
                        {t.settings.shortTermMemory.regenerate}
                      </Button>
                    </div>
                    {topics.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {topics.map((topic, i) => (
                          <Badge key={`${topic}-${i}`} variant="outline" className="text-xs">
                            {topic}
                          </Badge>
                        ))}
                      </div>
                    )}
                    {r.summary && (
                      <p className="text-xs text-muted-foreground">
                        {r.summary}
                      </p>
                    )}
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </SettingsSection>
    </PageContainer>
  );
}
