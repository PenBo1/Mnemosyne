import { useState, useEffect } from "react";
import { Spinner } from "@/components/ui/spinner";
import { BarChart3Icon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { getDailyActivity } from "@/services/stats";
import { HeatmapGrid } from "@/components/HeatmapGrid";
import type { DailyActivity } from "@/services/stats";

export function DashboardPage() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [chatActivity, setChatActivity] = useState<DailyActivity[]>([]);
  const [novelActivity, setNovelActivity] = useState<DailyActivity[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadActivity() {
      try {
        setLoading(true);
        const data = await getDailyActivity();
        setChatActivity(data.chatActivity);
        setNovelActivity(data.novelActivity);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load activity");
      } finally {
        setLoading(false);
      }
    }
    loadActivity();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Spinner className="size-6" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
              <BarChart3Icon />
              {t.dashboard.title}
            </h1>
            <p className="text-sm text-muted-foreground">{t.dashboard.description}</p>
          </div>
        </div>
        <div className="rounded-lg border border-destructive/50 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <BarChart3Icon />
            {t.dashboard.title}
          </h1>
          <p className="text-sm text-muted-foreground">{t.dashboard.description}</p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <HeatmapGrid
          data={chatActivity}
          title={t.dashboard.heatmap.chat}
          emptyMessage={t.dashboard.heatmap.emptyChat}
        />
        <HeatmapGrid
          data={novelActivity}
          title={t.dashboard.heatmap.novel}
          emptyMessage={t.dashboard.heatmap.emptyNovel}
        />
      </div>
    </div>
  );
}
