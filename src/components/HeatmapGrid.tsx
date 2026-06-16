import { useMemo } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { useI18n } from "@/lib/i18n";
import type { DailyActivity } from "@/services/stats";

function getActivityMap(activities: DailyActivity[]): Map<string, number> {
  const map = new Map<string, number>();
  for (const act of activities) {
    map.set(act.date, act.count);
  }
  return map;
}

function getIntensityClass(count: number, max: number): string {
  if (count === 0) return "fill-muted";
  const ratio = count / max;
  if (ratio < 0.25) return "fill-emerald-200 dark:fill-emerald-900";
  if (ratio < 0.5) return "fill-emerald-400 dark:fill-emerald-700";
  if (ratio < 0.75) return "fill-emerald-500 dark:fill-emerald-500";
  return "fill-emerald-700 dark:fill-emerald-300";
}

interface HeatmapData {
  weeks: (string | null)[][];
  monthLabels: { label: string; weekIndex: number }[];
  maxCount: number;
}

function buildHeatmapData(dates: string[], activityMap: Map<string, number>): HeatmapData {
  const weeks: (string | null)[][] = [];
  let currentWeek: (string | null)[] = [];

  for (const date of dates) {
    const dayOfWeek = new Date(date + "T00:00:00").getDay();
    if (dayOfWeek === 0 && currentWeek.length > 0) {
      while (currentWeek.length < 7) currentWeek.push(null);
      weeks.push(currentWeek);
      currentWeek = [];
    }
    currentWeek.push(date);
  }
  if (currentWeek.length > 0) {
    while (currentWeek.length < 7) currentWeek.push(null);
    weeks.push(currentWeek);
  }

  const monthLabels: { label: string; weekIndex: number }[] = [];
  let lastMonth = -1;
  for (let w = 0; w < weeks.length; w++) {
    for (const date of weeks[w]) {
      if (date) {
        const month = new Date(date + "T00:00:00").getMonth();
        if (month !== lastMonth) {
          monthLabels.push({
            label: new Date(date + "T00:00:00").toLocaleDateString("en-US", { month: "short" }),
            weekIndex: w,
          });
          lastMonth = month;
        }
        break;
      }
    }
  }

  const maxCount = Math.max(1, ...dates.map((d) => activityMap.get(d) || 0));
  return { weeks, monthLabels, maxCount };
}

const WEEKDAYS = ["", "Mon", "", "Wed", "", "Fri", ""];
const CELL_SIZE = 11;
const CELL_GAP = 3;

export function HeatmapGrid({
  data,
  title,
  emptyMessage,
}: {
  data: DailyActivity[];
  title: string;
  emptyMessage: string;
}) {
  const { t } = useI18n();
  const activityMap = useMemo(() => getActivityMap(data), [data]);

  const dates = useMemo(() => {
    const result: string[] = [];
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const start = new Date(today);
    start.setFullYear(start.getFullYear() - 1);
    start.setDate(start.getDate() + 1);
    const current = new Date(start);
    while (current <= today) {
      result.push(current.toISOString().split("T")[0]);
      current.setDate(current.getDate() + 1);
    }
    return result;
  }, []);

  const { weeks, monthLabels, maxCount } = useMemo(
    () => buildHeatmapData(dates, activityMap),
    [dates, activityMap]
  );

  const totalContributions = data.reduce((sum, d) => sum + d.count, 0);

  if (data.length === 0) {
    return (
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium">{title}</CardTitle>
        </CardHeader>
        <CardContent>
          <Empty>
            <EmptyHeader>
              <EmptyMedia>
                <EmptyTitle>{emptyMessage}</EmptyTitle>
              </EmptyMedia>
              <EmptyDescription>{t.dashboard.heatmap.startHint}</EmptyDescription>
            </EmptyHeader>
          </Empty>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">{title}</CardTitle>
          <Badge variant="secondary" className="text-xs">
            {totalContributions.toLocaleString()} {t.dashboard.heatmap.contributions}
          </Badge>
        </div>
      </CardHeader>
      <CardContent>
        <div className="overflow-x-auto">
          <div className="inline-flex flex-col gap-0.5">
            {/* Month labels row */}
            <div className="flex" style={{ marginLeft: 28 }}>
              {monthLabels.map((m, i) => (
                <div
                  key={i}
                  className="text-[10px] text-muted-foreground"
                  style={{ width: (weeks.length - m.weekIndex) * (CELL_SIZE + CELL_GAP), minWidth: 0 }}
                >
                  {m.label}
                </div>
              ))}
            </div>

            {/* Grid row */}
            <div className="flex items-start gap-0.5">
              {/* Weekday labels */}
              <div className="flex flex-col" style={{ gap: CELL_GAP }}>
                {WEEKDAYS.map((label, i) => (
                  <div
                    key={i}
                    className="text-[10px] text-muted-foreground flex items-center justify-end pr-1"
                    style={{ width: 24, height: CELL_SIZE }}
                  >
                    {label}
                  </div>
                ))}
              </div>

              {/* Grid cells */}
              {weeks.map((week, wi) => (
                <div key={wi} className="flex flex-col" style={{ gap: CELL_GAP }}>
                  {week.map((date, di) => {
                    if (!date) {
                      return (
                        <div
                          key={di}
                          className="fill-transparent"
                          style={{ width: CELL_SIZE, height: CELL_SIZE }}
                        />
                      );
                    }
                    const count = activityMap.get(date) || 0;
                    const cls = getIntensityClass(count, maxCount);
                    return (
                      <svg
                        key={di}
                        width={CELL_SIZE}
                        height={CELL_SIZE}
                        className={cls}
                        style={{ borderRadius: 2 }}
                      >
                        <rect width={CELL_SIZE} height={CELL_SIZE} rx={2} />
                        <title>{`${date}: ${count}`}</title>
                      </svg>
                    );
                  })}
                </div>
              ))}
            </div>

            {/* Legend */}
            <div className="flex items-center gap-1 mt-1" style={{ marginLeft: 28 }}>
              <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.less}</span>
              <svg width={CELL_SIZE} height={CELL_SIZE} className="fill-muted" style={{ borderRadius: 2 }}>
                <rect width={CELL_SIZE} height={CELL_SIZE} rx={2} />
              </svg>
              <svg width={CELL_SIZE} height={CELL_SIZE} className="fill-emerald-200 dark:fill-emerald-900" style={{ borderRadius: 2 }}>
                <rect width={CELL_SIZE} height={CELL_SIZE} rx={2} />
              </svg>
              <svg width={CELL_SIZE} height={CELL_SIZE} className="fill-emerald-400 dark:fill-emerald-700" style={{ borderRadius: 2 }}>
                <rect width={CELL_SIZE} height={CELL_SIZE} rx={2} />
              </svg>
              <svg width={CELL_SIZE} height={CELL_SIZE} className="fill-emerald-500 dark:fill-emerald-500" style={{ borderRadius: 2 }}>
                <rect width={CELL_SIZE} height={CELL_SIZE} rx={2} />
              </svg>
              <svg width={CELL_SIZE} height={CELL_SIZE} className="fill-emerald-700 dark:fill-emerald-300" style={{ borderRadius: 2 }}>
                <rect width={CELL_SIZE} height={CELL_SIZE} rx={2} />
              </svg>
              <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.more}</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
