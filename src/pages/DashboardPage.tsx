import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { BarChart3Icon, BookOpenIcon, FileTextIcon, TrendingUpIcon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useDashboard } from "@/hooks/useDashboard";
import { HeatmapGrid } from "@/components/HeatmapGrid";

export function DashboardPage() {
  const { t } = useI18n();
  const { stats, activity, loading, error } = useDashboard();

  if (loading) {
    return (
      <div className="flex flex-col gap-6">
        <Skeleton className="h-8 w-48" />
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
        <Skeleton className="h-48" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-6">
        <h1 className="text-2xl font-bold tracking-tight">{t.dashboard.title}</h1>
        <div className="rounded-lg border border-destructive/50 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      </div>
    );
  }

  const statCards = [
    {
      icon: BookOpenIcon,
      label: t.dashboard.stats.novels,
      value: stats?.novelCount ?? 0,
      color: "text-blue-500",
    },
    {
      icon: FileTextIcon,
      label: t.dashboard.stats.prompts,
      value: stats?.promptCount ?? 0,
      color: "text-green-500",
    },
    {
      icon: TrendingUpIcon,
      label: t.dashboard.stats.trends,
      value: stats?.trendCount ?? 0,
      color: "text-purple-500",
    },
    {
      icon: BarChart3Icon,
      label: t.dashboard.stats.words,
      value: stats?.totalWords ?? 0,
      color: "text-orange-500",
    },
  ];

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{t.dashboard.title}</h1>
        <p className="text-sm text-muted-foreground">{t.dashboard.description}</p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {statCards.map((card) => (
          <Card key={card.label}>
            <CardContent className="flex items-center gap-3 py-4">
              <div className={`rounded-md bg-muted p-2`}>
                <card.icon className={`size-4 ${card.color}`} />
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

      <div>
        <h2 className="text-lg font-semibold mb-3">{t.dashboard.heatmap.title}</h2>
        <HeatmapGrid
          data={activity}
          title={t.dashboard.heatmap.overview}
          emptyMessage={t.dashboard.heatmap.empty}
        />
      </div>
    </div>
  );
}
