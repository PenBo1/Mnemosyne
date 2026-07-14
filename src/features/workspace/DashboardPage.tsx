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
  SectionTitle,
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
  AlertTriangleIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useDashboard } from "@/features/workspace/hooks/useDashboard";
import { useAiAnalytics } from "@/features/stats/hooks/useAiAnalytics";
import { HeatmapGrid } from "@/features/workspace/components";

export function DashboardPage() {
  const { t } = useI18n();
  const { stats, activity, loading: dashboardLoading } = useDashboard();
  const { aiStats, auditStats, auditEvents, loading: aiLoading } =
    useAiAnalytics();

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

  const statCards = [
    { icon: BookOpenIcon, label: t.dashboard.stats.novels, value: stats?.novelCount ?? 0 },
    { icon: FileTextIcon, label: t.dashboard.stats.prompts, value: stats?.promptCount ?? 0 },
    { icon: TrendingUpIcon, label: t.dashboard.stats.trends, value: stats?.trendCount ?? 0 },
    { icon: BarChart3Icon, label: t.dashboard.stats.words, value: stats?.totalWords ?? 0 },
    { icon: CoinsIcon, label: t.dashboard.stats.tokens, value: aiStats?.totalTokens ?? 0 },
    { icon: HammerIcon, label: t.dashboard.stats.toolCalls, value: aiStats?.toolCalls ?? 0 },
    { icon: ActivityIcon, label: t.dashboard.stats.llmCalls, value: aiStats?.llmCalls ?? 0 },
    { icon: ShieldAlertIcon, label: t.dashboard.stats.violations, value: auditStats?.denied ?? 0 },
  ];

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <TrendingUpIcon />
            {t.dashboard.title}
          </PageTitle>
          <PageDescription>{t.dashboard.description}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* 概览卡片 */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {statCards.map((card) => (
          <Card key={card.label}>
            <CardContent className="flex items-center gap-3 py-4">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)]">
                <card.icon className="size-4 text-[var(--text-brand)]" />
              </div>
              <div className="min-w-0">
                <p className="trae-stat-value">{card.value.toLocaleString()}</p>
                <p className="trae-eyebrow mt-0.5">{card.label}</p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <Separator />

      {/* 活跃度热力图 */}
      <div className="flex flex-col gap-3">
        <SectionTitle>{t.dashboard.heatmap.title}</SectionTitle>
        <HeatmapGrid
          data={activity}
          title={t.dashboard.heatmap.overview}
          emptyMessage={t.dashboard.heatmap.empty}
        />
      </div>

      {/* 模型用量 */}
      {aiStats && aiStats.modelUsage.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <SectionTitle className="flex items-center gap-2">
              <BrainIcon className="size-5" /> {t.dashboard.ai.modelUsage}
            </SectionTitle>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {aiStats.modelUsage.map((model) => (
                <Card key={`${model.provider ?? "unknown"}-${model.model}`}>
                  <CardContent className="flex flex-col gap-2 py-3">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-sm truncate">{model.model}</span>
                      <Badge variant="secondary">{model.calls} calls</Badge>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
                      <div className="trae-num">In: {model.inputTokens.toLocaleString()}</div>
                      <div className="trae-num">Out: {model.outputTokens.toLocaleString()}</div>
                      <div className="trae-num">Total: {model.totalTokens.toLocaleString()}</div>
                      <div>
                        {model.provider ?? "—"}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        </>
      )}

      {/* 最近审计事件 */}
      {auditEvents.length > 0 && (
        <>
          <Separator />
          <div className="flex flex-col gap-3">
            <SectionTitle className="flex items-center gap-2">
              <AlertTriangleIcon className="size-5 text-muted-foreground" />{" "}
              {t.dashboard.ai.recentAuditEvents}
            </SectionTitle>
            <ScrollArea className="h-72">
              <div className="flex flex-col gap-2">
                {auditEvents.slice(0, 20).map((event) => (
                  <Card key={event.id}>
                    <CardContent className="py-2 px-3 flex items-center gap-3 text-xs">
                      {event.isDenied ? (
                        <ShieldAlertIcon className="size-4 text-destructive shrink-0" />
                      ) : (
                        <ActivityIcon className="size-4 text-muted-foreground shrink-0" />
                      )}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <Badge
                            variant={event.isDenied ? "destructive" : "outline"}
                            className="text-[10px]"
                          >
                            {event.eventType}
                          </Badge>
                          {event.operation && (
                            <span className="font-medium truncate">{event.operation}</span>
                          )}
                        </div>
                        {event.workspaceId && (
                          <p className="text-muted-foreground truncate">
                            ws: {event.workspaceId.slice(0, 8)}
                          </p>
                        )}
                      </div>
                      <div className="text-right shrink-0 text-muted-foreground trae-num">
                        {new Date(event.recordedAt).toLocaleTimeString()}
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
      {(aiStats?.llmCalls ?? 0) === 0 && (auditStats?.total ?? 0) === 0 && (
        <>
          <Separator />
          <EmptyState
            icon={<BrainIcon className="size-6" />}
            title={t.dashboard.ai.noData}
            description={t.dashboard.ai.noDataHint}
          />
        </>
      )}
    </PageContainer>
  );
}