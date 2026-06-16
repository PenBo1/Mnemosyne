import { useMemo } from "react";
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { useI18n } from "@/lib/i18n";
import type { Novel, ChapterSummary, HookRecord } from "@/types";

interface DashboardProps {
  novel: Novel;
  summaries: ChapterSummary[];
  hooks: HookRecord[];
  totalWords: number;
  chapterCount: number;
}

const HOOK_STATUS_COLORS: Record<string, string> = {
  Open: "#f59e0b",
  Progressing: "#3b82f6",
  Deferred: "#6b7280",
  Resolved: "#10b981",
};

export function WritingDashboard({ summaries, hooks, totalWords, chapterCount }: DashboardProps) {
  const { t } = useI18n();

  const wordTrend = useMemo(() => {
    return summaries.map((s) => ({
      chapter: `Ch.${s.chapter}`,
      words: s.events.length * 800 + s.characters.length * 200,
    }));
  }, [summaries]);

  const hookStatusData = useMemo(() => {
    const counts: Record<string, number> = {};
    hooks.forEach((h) => {
      counts[h.status] = (counts[h.status] || 0) + 1;
    });
    return Object.entries(counts).map(([status, count]) => ({
      name: status === "Open" ? t.viz.hooks.open : status === "Progressing" ? t.viz.hooks.progressing : status === "Resolved" ? t.viz.hooks.resolved : t.viz.hooks.deferred,
      value: count,
      color: HOOK_STATUS_COLORS[status] || "#6b7280",
    }));
  }, [hooks]);

  const chapterProgress = useMemo(() => {
    const completed = summaries.filter((s) => s.chapter_type === "finalized").length;
    return { completed, total: chapterCount, percentage: chapterCount > 0 ? Math.round((completed / chapterCount) * 100) : 0 };
  }, [summaries, chapterCount]);

  const moodDistribution = useMemo(() => {
    const counts: Record<string, number> = {};
    summaries.forEach((s) => {
      if (s.mood) counts[s.mood] = (counts[s.mood] || 0) + 1;
    });
    return Object.entries(counts)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6)
      .map(([mood, count]) => ({ mood, count }));
  }, [summaries]);

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatCard label={t.viz.stats.totalWords} value={totalWords.toLocaleString()} />
        <StatCard label={t.viz.stats.chapterCount} value={`${chapterCount}`} />
        <StatCard label={t.viz.stats.hookCount} value={`${hooks.length}`} />
        <StatCard label={t.viz.stats.completed} value={`${chapterProgress.percentage}%`} />
      </div>

      <div className="rounded-lg border p-4">
        <h3 className="text-sm font-medium text-muted-foreground mb-3">{t.viz.dashboard.chapterProgress}</h3>
        <div className="relative h-6 bg-muted rounded-full overflow-hidden">
          <div
            className="absolute inset-y-0 left-0 bg-primary rounded-full transition-all duration-500"
            style={{ width: `${chapterProgress.percentage}%` }}
          />
          <div className="absolute inset-0 flex items-center justify-center text-xs font-medium">
            {t.viz.dashboard.chapterProgressDetail
              .replace("{completed}", String(chapterProgress.completed))
              .replace("{total}", String(chapterProgress.total))
              .replace("{percentage}", String(chapterProgress.percentage))}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {wordTrend.length > 0 && (
          <div className="rounded-lg border p-4">
            <h3 className="text-sm font-medium text-muted-foreground mb-3">{t.viz.dashboard.wordTrend}</h3>
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={wordTrend}>
                <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
                <XAxis dataKey="chapter" tick={{ fontSize: 11 }} />
                <YAxis tick={{ fontSize: 11 }} />
                <Tooltip />
                <Line type="monotone" dataKey="words" stroke="hsl(var(--primary))" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}

        {hookStatusData.length > 0 && (
          <div className="rounded-lg border p-4">
            <h3 className="text-sm font-medium text-muted-foreground mb-3">{t.viz.dashboard.hookStatus}</h3>
            <ResponsiveContainer width="100%" height={200}>
              <PieChart>
                <Pie
                  data={hookStatusData}
                  cx="50%"
                  cy="50%"
                  innerRadius={50}
                  outerRadius={80}
                  paddingAngle={3}
                  dataKey="value"
                >
                  {hookStatusData.map((entry) => (
                    <Cell key={entry.name} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip />
                <Legend />
              </PieChart>
            </ResponsiveContainer>
          </div>
        )}

        {moodDistribution.length > 0 && (
          <div className="rounded-lg border p-4">
            <h3 className="text-sm font-medium text-muted-foreground mb-3">{t.viz.dashboard.moodDistribution}</h3>
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={moodDistribution}>
                <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
                <XAxis dataKey="mood" tick={{ fontSize: 11 }} />
                <YAxis tick={{ fontSize: 11 }} />
                <Tooltip />
                <Bar dataKey="count" fill="hsl(var(--primary))" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}

        {summaries.length > 0 && (
          <div className="rounded-lg border p-4">
            <h3 className="text-sm font-medium text-muted-foreground mb-3">{t.viz.dashboard.characterAppearances}</h3>
            <div className="space-y-2 max-h-[200px] overflow-y-auto">
              {getCharacterStats(summaries).map(({ name, count }) => (
                <div key={name} className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground truncate w-20">{name}</span>
                  <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary rounded-full"
                      style={{ width: `${(count / summaries.length) * 100}%` }}
                    />
                  </div>
                  <span className="text-xs text-muted-foreground">{count}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border p-4">
      <div className="text-sm text-muted-foreground">{label}</div>
      <div className="text-2xl font-bold mt-1">{value}</div>
    </div>
  );
}

function getCharacterStats(summaries: ChapterSummary[]): { name: string; count: number }[] {
  const counts: Record<string, number> = {};
  summaries.forEach((s) => {
    s.characters.forEach((c) => {
      counts[c] = (counts[c] || 0) + 1;
    });
  });
  return Object.entries(counts)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 8)
    .map(([name, count]) => ({ name, count }));
}
