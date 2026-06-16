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

function getIntensityLevel(count: number, max: number): number {
  if (count === 0) return 0;
  const ratio = count / max;
  if (ratio < 0.25) return 1;
  if (ratio < 0.5) return 2;
  if (ratio < 0.75) return 3;
  return 4;
}

const INTENSITY_CLASSES = [
  "bg-muted",
  "bg-emerald-200 dark:bg-emerald-900",
  "bg-emerald-400 dark:bg-emerald-700",
  "bg-emerald-500 dark:bg-emerald-500",
  "bg-emerald-700 dark:bg-emerald-300",
];

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
const CELL_SIZE = 13;

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
          <div className="inline-block">
            {/* Month labels */}
            <div className="flex ml-7 mb-1">
              {monthLabels.map((m, i) => (
                <div
                  key={i}
                  className="text-[10px] text-muted-foreground"
                  style={{ position: "absolute", marginLeft: `${m.weekIndex * CELL_SIZE}px` }}
                >
                  {m.label}
                </div>
              ))}
            </div>

            {/* Grid */}
            <div className="flex">
              <div className="flex flex-col mr-1">
                {WEEKDAYS.map((label, i) => (
                  <div
                    key={i}
                    className="text-[10px] text-muted-foreground flex items-center justify-end"
                    style={{ width: "24px", height: `${CELL_SIZE}px` }}
                  >
                    {label}
                  </div>
                ))}
              </div>
              {weeks.map((week, wi) => (
                <div key={wi} className="flex flex-col">
                  {week.map((date, di) => {
                    if (!date) {
                      return (
                        <div
                          key={di}
                          style={{ width: CELL_SIZE, height: CELL_SIZE }}
                        />
                      );
                    }
                    const count = activityMap.get(date) || 0;
                    const level = getIntensityLevel(count, maxCount);
                    return (
                      <div
                        key={di}
                        className={`rounded-[3px] ${INTENSITY_CLASSES[level]}`}
                        style={{ width: CELL_SIZE, height: CELL_SIZE }}
                        title={`${date}: ${count}`}
                      />
                    );
                  })}
                </div>
              ))}
            </div>

            {/* Legend */}
            <div className="flex items-center gap-1 mt-2 ml-7">
              <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.less}</span>
              {INTENSITY_CLASSES.map((cls, i) => (
                <div
                  key={i}
                  className={`rounded-[3px] ${cls}`}
                  style={{ width: CELL_SIZE, height: CELL_SIZE }}
                />
              ))}
              <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.more}</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
