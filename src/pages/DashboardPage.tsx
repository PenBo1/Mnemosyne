import { useMemo } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { EmptyState } from "@/components/shared/state";
import {
  BarChart3Icon,
  BookOpenIcon,
  FileTextIcon,
  TrendingUpIcon,
  CoinsIcon,
  HammerIcon,
  ActivityIcon,
  ShieldAlertIcon,
  BrainIcon,
  ClockIcon,
  CheckCircleIcon,
  XCircleIcon,
  AlertTriangleIcon,
} from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { useDashboard } from "@/features/workspace/hooks/useDashboard";
import { useAiAnalytics } from "@/features/stats/hooks/useAiAnalytics";
import { useAgentStore } from "@/stores/agent";
import { HeatmapGrid } from "@/features/workspace/components";

export function DashboardPage() {
  const { t } = useI18n();
  const { stats, activity, loading: dashboardLoading } = useDashboard();
  const currentSessionId = useAgentStore((s) => s.currentSessionId);
  const { llmCalls, toolExecutions, tokenUsage, sandboxViolations, loading: aiLoading } =
    useAiAnalytics(currentSessionId);

  if (dashboardLoading || aiLoading) {
    return (
      <PageContainer>
        <Skeleton className="h-8 w-48" />
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
        <Skeleton className="h-48" />
      </PageContainer>
    );
  }

  const totalTokens = tokenUsage?.total_tokens ?? 0;
  const totalToolCalls = tokenUsage?.tools.total_calls ?? 0;
  const toolErrorRate = useMemo(
    () =>
      tokenUsage?.tools.success_rate
        ? ((1 - tokenUsage.tools.success_rate) * 100).toFixed(1)
        : "0",
    [tokenUsage?.tools.success_rate],
  );
  const avgLatency = useMemo(
    () =>
      tokenUsage?.models.length
        ? Math.round(
            tokenUsage.models.reduce((s, m) => s + (m.avg_latency_ms ?? 0), 0) /
              tokenUsage.models.length,
          )
        : 0,
    [tokenUsage?.models],
  );

  const statCards = [
    { icon: BookOpenIcon, label: t.dashboard.stats.novels, value: stats?.novelCount ?? 0 },
    { icon: FileTextIcon, label: t.dashboard.stats.prompts, value: stats?.promptCount ?? 0 },
    { icon: TrendingUpIcon, label: t.dashboard.stats.trends, value: stats?.trendCount ?? 0 },
    { icon: BarChart3Icon, label: t.dashboard.stats.words, value: stats?.totalWords ?? 0 },
    { icon: CoinsIcon, label: t.dashboard.stats.tokens, value: totalTokens },
    { icon: HammerIcon, label: t.dashboard.stats.toolCalls, value: totalToolCalls },
    { icon: ActivityIcon, label: t.dashboard.stats.llmCalls, value: llmCalls.length },
    { icon: ShieldAlertIcon, label: t.dashboard.stats.violations, value: sandboxViolations.length },
  ];

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.dashboard.title}</PageTitle>
          <PageDescription>{t.dashboard.description}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* 概览卡片 */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {statCards.map((card) => (
          <Card key={card.label}>
            <CardContent className="flex items-center gap-3 py-4">
              <div className="rounded-[var(--radius-4)] bg-muted p-2">
                <card.icon className="size-4 text-primary" />
              </div>
              <div>
                <p className="text-2xl font-bold">{card.value.toLocaleString()}</p>
                <p className="text-xs text-muted-foreground">{card.label}</p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <Separator />

      {/* 活跃度热力图 */}
      <div className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t.dashboard.heatmap.title}</h2>
        <HeatmapGrid
          data={activity}
          title={t.dashboard.heatmap.overview}
          emptyMessage={t.dashboard.heatmap.empty}
        />
      </div>

      {/* 模型用量 */}
      {tokenUsage?.models && tokenUsage.models.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <BrainIcon className="size-5" /> {t.dashboard.ai.modelUsage}
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {tokenUsage.models.map((model) => (
                <Card key={model.model}>
                  <CardContent className="flex flex-col gap-2 py-3">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-sm truncate">{model.model}</span>
                      <Badge variant="secondary">{model.calls} calls</Badge>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
                      <div>In: {(model.input_tokens ?? 0).toLocaleString()}</div>
                      <div>Out: {(model.output_tokens ?? 0).toLocaleString()}</div>
                      <div>Latency: {Math.round(model.avg_latency_ms ?? 0)}ms</div>
                      <div>
                        Cost: ~$
                        {(
                          ((model.input_tokens ?? 0) * 0.00001 +
                            (model.output_tokens ?? 0) * 0.00003)
                        ).toFixed(4)}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        </>
      )}

      {/* 工具统计 */}
      {tokenUsage?.tools && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <HammerIcon className="size-5" /> {t.dashboard.ai.toolStats}
            </h2>
            <div className="grid grid-cols-3 gap-4">
              <Card>
                <CardContent className="flex flex-col items-center gap-1 py-4">
                  <p className="text-3xl font-bold text-[var(--status-success-default)]">
                    {tokenUsage.tools.total_calls - tokenUsage.tools.errors}
                  </p>
                  <p className="text-xs text-muted-foreground">{t.dashboard.ai.successful}</p>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="flex flex-col items-center gap-1 py-4">
                  <p className="text-3xl font-bold text-destructive">
                    {tokenUsage.tools.errors}
                  </p>
                  <p className="text-xs text-muted-foreground">{t.dashboard.ai.errors}</p>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="flex flex-col items-center gap-1 py-4">
                  <p className="text-3xl font-bold text-muted-foreground">
                    {tokenUsage.tools.sandbox_blocked}
                  </p>
                  <p className="text-xs text-muted-foreground">{t.dashboard.ai.sandboxBlocked}</p>
                </CardContent>
              </Card>
            </div>
            <p className="text-xs text-muted-foreground">
              Error rate: {toolErrorRate}% | Avg latency: {avgLatency}ms
            </p>
          </div>
        </>
      )}

      {/* 最近 LLM 调用 */}
      {llmCalls.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <ActivityIcon className="size-5" /> {t.dashboard.ai.recentCalls}
            </h2>
            <ScrollArea className="h-72">
              <div className="flex flex-col gap-2">
                {llmCalls.slice(0, 20).map((call) => (
                  <Card key={call.id}>
                    <CardContent className="py-2 px-3 flex items-center gap-3 text-xs">
                      {call.status === "completed" ? (
                        <CheckCircleIcon className="size-4 text-[var(--status-success-default)] shrink-0" />
                      ) : call.status === "failed" ? (
                        <XCircleIcon className="size-4 text-destructive shrink-0" />
                      ) : (
                        <ClockIcon className="size-4 text-muted-foreground shrink-0" />
                      )}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-medium">{call.model}</span>
                          <Badge variant="outline" className="text-[10px]">
                            {call.agent_role}
                          </Badge>
                        </div>
                        <p className="text-muted-foreground truncate">
                          {call.response_content?.slice(0, 80) ?? "(no response)"}
                        </p>
                      </div>
                      <div className="text-right shrink-0">
                        <div>
                          {call.input_tokens + call.output_tokens} tokens
                        </div>
                        <div className="text-muted-foreground">
                          {call.latency_ms ?? "-"}ms
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </div>
        </>
      )}

      {/* 沙箱违规 */}
      {sandboxViolations.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <AlertTriangleIcon className="size-5 text-muted-foreground" /> {t.dashboard.ai.sandboxViolations}
            </h2>
            <ScrollArea className="h-48">
              <div className="flex flex-col gap-2">
                {sandboxViolations.map((v) => (
                  <Card key={v.id}>
                    <CardContent className="py-2 px-3 flex items-center gap-3 text-xs">
                      <ShieldAlertIcon className="size-4 text-destructive shrink-0" />
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <Badge variant="destructive" className="text-[10px]">
                            {v.violation_type}
                          </Badge>
                          <span className="text-muted-foreground">{v.action}</span>
                        </div>
                        <p className="text-muted-foreground truncate">{v.resource}</p>
                      </div>
                      <div className="text-right shrink-0 text-muted-foreground">
                        {new Date(v.detected_at).toLocaleTimeString()}
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </div>
        </>
      )}

      {/* AI 空状态 */}
      {llmCalls.length === 0 && toolExecutions.length === 0 && (
        <>
          <Separator />
          <EmptyState
            icon={<BrainIcon />}
            title={t.dashboard.ai.noData}
            description={t.dashboard.ai.noDataHint}
          />
        </>
      )}
    </PageContainer>
  );
}
