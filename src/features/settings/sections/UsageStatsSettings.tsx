// 使用统计设置页 —— Tokens 用量、活跃热力图、模型统计

import { useState, useEffect, useMemo } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  SectionTitle,
} from "@/components/shared/page-layout";
import { EmptyState } from "@/components/shared/state";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import {
  CoinsIcon,
  MessageSquareIcon,
  CalendarIcon,
  FlameIcon,
  BrainIcon,
  BarChart3Icon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { getUsageStats, type UsageStats, type TimeRange } from "@/features/settings/services/usage-stats";

// 热力图颜色等级
const HEAT_LEVELS = [
  "bg-[var(--chart-5)]/10",
  "bg-[var(--chart-5)]/30",
  "bg-[var(--chart-5)]/50",
  "bg-[var(--chart-5)]/70",
  "bg-[var(--chart-5)]",
];

export function UsageStatsSettings() {
  const { t } = useI18n();
  const tu = t.settings.usageStats;
  const [timeRange, setTimeRange] = useState<TimeRange>(7);
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getUsageStats(timeRange)
      .then((data) => {
        if (!cancelled) setStats(data);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [timeRange]);

  const trendData = useMemo(() => {
    if (!stats?.tokenTrend) return [];
    return stats.tokenTrend.map((point) => ({
      date: new Date(point.date + "T00:00:00").toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
      }),
      tokens: point.tokens,
    }));
  }, [stats?.tokenTrend]);

  const chartConfig = useMemo(
    () => ({
      tokens: {
        label: tu?.tokens ?? "Tokens",
        color: "var(--chart-1)",
      },
    }),
    [tu?.tokens]
  );

  // 热力图数据转换：将 heatmap 数组转换为日历网格
  const heatmapGrid = useMemo(() => {
    if (!stats?.heatmap || stats.heatmap.length === 0) return null;

    // 找出最大活跃度用于计算颜色等级
    const maxCount = Math.max(...stats.heatmap.map((c) => c.count), 1);

    // 创建日期到活跃度的映射
    const dateMap = new Map<string, number>();
    for (const cell of stats.heatmap) {
      dateMap.set(cell.date, cell.count);
    }

    // 计算显示的周数（按时间范围）
    const weeksToShow = timeRange === 7 ? 1 : 5;
    const grid: { date: string; count: number; level: number }[][] = [];

    const today = new Date();
    today.setHours(0, 0, 0, 0);

    for (let week = weeksToShow - 1; week >= 0; week--) {
      const weekData: { date: string; count: number; level: number }[] = [];
      for (let day = 6; day >= 0; day--) {
        const d = new Date(today);
        d.setDate(d.getDate() - week * 7 - day);
        const dateStr = d.toISOString().slice(0, 10);
        const count = dateMap.get(dateStr) || 0;
        const level = count === 0 ? -1 : Math.min(Math.floor((count / maxCount) * HEAT_LEVELS.length), HEAT_LEVELS.length - 1);
        weekData.push({ date: dateStr, count, level });
      }
      grid.push(weekData);
    }

    return grid;
  }, [stats?.heatmap, timeRange]);

  if (loading) {
    return (
      <PageContainer>
        <PageHeader>
          <PageHeading>
            <PageTitle>{tu?.title ?? "Usage Stats"}</PageTitle>
            <PageDescription>{tu?.description ?? "View usage statistics"}</PageDescription>
          </PageHeading>
        </PageHeader>
        <div className="flex flex-col gap-4">
          <Skeleton className="h-10 w-32" />
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-24" />
            ))}
          </div>
          <Skeleton className="h-48" />
          <Skeleton className="h-48" />
        </div>
      </PageContainer>
    );
  }

  if (error) {
    return (
      <PageContainer>
        <PageHeader>
          <PageHeading>
            <PageTitle>{tu?.title ?? "Usage Stats"}</PageTitle>
            <PageDescription>{tu?.description ?? "View usage statistics"}</PageDescription>
          </PageHeading>
        </PageHeader>
        <EmptyState
          icon={<BarChart3Icon className="size-6" />}
          title={t.common.error}
          description={error}
        />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{tu?.title ?? "Usage Stats"}</PageTitle>
          <PageDescription>{tu?.description ?? "View usage statistics"}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* 时间范围选择器 */}
      <div className="flex items-center gap-2">
        <span className="text-sm text-muted-foreground">{tu?.timeRange ?? "Time Range"}</span>
        <Select value={String(timeRange)} onValueChange={(v) => setTimeRange(Number(v) as TimeRange)}>
          <SelectTrigger className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="7">{tu?.last7Days ?? "Last 7 Days"}</SelectItem>
            <SelectItem value="30">{tu?.last30Days ?? "Last 30 Days"}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
              <CoinsIcon className="size-4 text-[var(--text-brand)]" />
            </div>
            <div className="min-w-0">
              <p className="trae-stat-value">{(stats?.totalTokens ?? 0).toLocaleString()}</p>
              <p className="trae-eyebrow mt-0.5">{tu?.tokens ?? "Tokens"}</p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
              <MessageSquareIcon className="size-4 text-[var(--text-brand)]" />
            </div>
            <div className="min-w-0">
              <p className="trae-stat-value">{(stats?.sessionCount ?? 0).toLocaleString()}</p>
              <p className="trae-eyebrow mt-0.5">{tu?.sessions ?? "Sessions"}</p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
              <MessageSquareIcon className="size-4 text-[var(--text-brand)]" />
            </div>
            <div className="min-w-0">
              <p className="trae-stat-value">{(stats?.messageCount ?? 0).toLocaleString()}</p>
              <p className="trae-eyebrow mt-0.5">{tu?.messages ?? "Messages"}</p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
              <CalendarIcon className="size-4 text-[var(--text-brand)]" />
            </div>
            <div className="min-w-0">
              <p className="trae-stat-value">{stats?.activeDays ?? 0}</p>
              <p className="trae-eyebrow mt-0.5">{tu?.activeDays ?? "Active Days"}</p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
              <FlameIcon className="size-4 text-[var(--text-brand)]" />
            </div>
            <div className="min-w-0">
              <p className="trae-stat-value">{stats?.currentStreak ?? 0}</p>
              <p className="trae-eyebrow mt-0.5">{tu?.currentStreak ?? "Current Streak"}</p>
            </div>
          </CardContent>
        </Card>

        {stats?.mostUsedModel && (
          <Card>
            <CardContent className="flex items-center gap-3 py-4">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                <BrainIcon className="size-4 text-[var(--text-brand)]" />
              </div>
              <div className="min-w-0">
                <p className="text-sm font-medium truncate">{stats.mostUsedModel.model}</p>
                <p className="trae-eyebrow mt-0.5">{tu?.mostUsedModel ?? "Most Used"}</p>
              </div>
            </CardContent>
          </Card>
        )}
      </div>

      {/* 活跃热力图 */}
      {heatmapGrid && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <SectionTitle className="flex items-center gap-2">
              <CalendarIcon className="size-5" />
              {tu?.heatmap ?? "Activity Heatmap"}
            </SectionTitle>
            <Card>
              <CardContent className="py-4">
                <div className="flex flex-col gap-1">
                  {/* 星期标签 */}
                  <div className="flex gap-1 text-[10px] text-muted-foreground mb-1">
                    <span className="w-8" />
                    {["Mon", "Wed", "Fri", "Sun"].map((d) => (
                      <span key={d} className="w-6 text-center">{d}</span>
                    ))}
                  </div>
                  {/* 热力图网格 */}
                  <div className="flex gap-1">
                    {heatmapGrid.map((week, weekIdx) => (
                      <div key={weekIdx} className="flex flex-col gap-0.5">
                        {week.map((cell, dayIdx) => (
                          <div
                            key={`${weekIdx}-${dayIdx}`}
                            className={`size-6 rounded-sm ${
                              cell.level >= 0 ? HEAT_LEVELS[cell.level] : "bg-muted/30"
                            }`}
                            title={`${cell.date}: ${cell.count} sessions`}
                          />
                        ))}
                      </div>
                    ))}
                  </div>
                </div>
                {/* 颜色说明 */}
                <div className="flex items-center justify-end gap-1 mt-3 text-xs text-muted-foreground">
                  <span>{tu?.less ?? "Less"}</span>
                  {HEAT_LEVELS.map((_, i) => (
                    <div
                      key={i}
                      className={`size-3 rounded-sm ${HEAT_LEVELS[i]}`}
                    />
                  ))}
                  <span>{tu?.more ?? "More"}</span>
                </div>
              </CardContent>
            </Card>
          </div>
        </>
      )}

      {/* Token 趋势图 */}
      {trendData.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <SectionTitle className="flex items-center gap-2">
              <CoinsIcon className="size-5" />
              {tu?.tokenTrend ?? "Token Trend"}
            </SectionTitle>
            <Card>
              <CardContent className="pt-4">
                <ChartContainer config={chartConfig} className="aspect-auto h-[180px]">
                  <AreaChart accessibilityLayer data={trendData} margin={{ left: 0, right: 12, top: 8 }}>
                    <defs>
                      <linearGradient id="tokenFill" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stopColor="var(--chart-1)" stopOpacity={0.4} />
                        <stop offset="100%" stopColor="var(--chart-1)" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <CartesianGrid vertical={false} strokeDasharray="3 3" />
                    <XAxis
                      dataKey="date"
                      tickLine={false}
                      axisLine={false}
                      tickMargin={8}
                      tick={{ fontSize: 10 }}
                      minTickGap={20}
                    />
                    <YAxis
                      tickLine={false}
                      axisLine={false}
                      tick={{ fontSize: 11 }}
                      width={48}
                    />
                    <ChartTooltip
                      cursor={false}
                      content={<ChartTooltipContent indicator="line" />}
                    />
                    <Area
                      dataKey="tokens"
                      type="monotone"
                      stroke="var(--chart-1)"
                      strokeWidth={2}
                      fill="url(#tokenFill)"
                    />
                  </AreaChart>
                </ChartContainer>
              </CardContent>
            </Card>
          </div>
        </>
      )}

      {/* 模型用量明细 */}
      {stats?.modelUsage && stats.modelUsage.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <SectionTitle className="flex items-center gap-2">
              <BrainIcon className="size-5" /> {tu?.modelUsage ?? "Model Usage"}
            </SectionTitle>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {stats.modelUsage.map((model) => (
                <Card key={`${model.provider ?? "unknown"}-${model.model}`}>
                  <CardContent className="flex flex-col gap-2 py-3">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-sm truncate">{model.model}</span>
                      <Badge variant="secondary">{model.ratio}%</Badge>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
                      <div className="trae-num">{tu?.inputTokens ?? "Input"}: {model.inputTokens.toLocaleString()}</div>
                      <div className="trae-num">{tu?.outputTokens ?? "Output"}: {model.outputTokens.toLocaleString()}</div>
                      <div className="trae-num">{tu?.totalTokens ?? "Total"}: {model.totalTokens.toLocaleString()}</div>
                      <div>{model.calls} {tu?.calls ?? "calls"}</div>
                    </div>
                    {/* 占比进度条 */}
                    <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full bg-[var(--chart-1)] transition-all"
                        style={{ width: `${model.ratio}%` }}
                      />
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        </>
      )}

      {/* 空状态 */}
      {stats && stats.totalTokens === 0 && (
        <>
          <Separator />
          <EmptyState
            icon={<BarChart3Icon className="size-6" />}
            title={tu?.noData ?? "No data"}
            description={tu?.noDataHint ?? "Start using the app to see stats"}
          />
        </>
      )}
    </PageContainer>
  );
}