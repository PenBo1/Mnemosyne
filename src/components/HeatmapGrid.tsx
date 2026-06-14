import { useMemo } from "react";
import { BarChart3Icon } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
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

function getIntensityClass(level: number): string {
  switch (level) {
    case 0: return "bg-muted";
    case 1: return "bg-chart-5";
    case 2: return "bg-chart-4";
    case 3: return "bg-chart-3";
    case 4: return "bg-chart-2";
    default: return "bg-muted";
  }
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
      while (currentWeek.length < 7) {
        currentWeek.push(null);
      }
      weeks.push(currentWeek);
      currentWeek = [];
    }
    currentWeek.push(date);
  }
  if (currentWeek.length > 0) {
    while (currentWeek.length < 7) {
      currentWeek.push(null);
    }
    weeks.push(currentWeek);
  }

  const monthLabels: { label: string; weekIndex: number }[] = [];
  let lastMonth = -1;
  for (let w = 0; w < weeks.length; w++) {
    for (const date of weeks[w]) {
      if (date) {
        const d = new Date(date + "T00:00:00");
        const month = d.getMonth();
        if (month !== lastMonth) {
          monthLabels.push({
            label: d.toLocaleDateString("en-US", { month: "short" }),
            weekIndex: w,
          });
          lastMonth = month;
        }
        break;
      }
    }
  }

  const counts = dates.map((d) => activityMap.get(d) || 0);
  const maxCount = Math.max(1, ...counts);

  return { weeks, monthLabels, maxCount };
}

const WEEKDAY_LABELS = ["", "Mon", "", "Wed", "", "Fri", ""];

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
    const endDate = new Date(today);

    const startDate = new Date(endDate);
    startDate.setFullYear(startDate.getFullYear() - 1);
    startDate.setDate(startDate.getDate() + 1);

    const current = new Date(startDate);
    while (current <= endDate) {
      result.push(current.toISOString().split("T")[0]);
      current.setDate(current.getDate() + 1);
    }
    return result;
  }, []);

  const { weeks, monthLabels, maxCount } = useMemo(
    () => buildHeatmapData(dates, activityMap),
    [dates, activityMap]
  );

  if (data.length === 0) {
    return (
      <div className="rounded-lg border bg-card p-5">
        <div className="flex items-center gap-2 mb-3">
          <BarChart3Icon className="size-4 text-muted-foreground" />
          <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
            {title}
          </h3>
        </div>
        <Empty>
          <EmptyHeader>
            <EmptyMedia>
              <EmptyTitle>{emptyMessage}</EmptyTitle>
            </EmptyMedia>
            <EmptyDescription>{t.dashboard.heatmap.startHint}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      </div>
    );
  }

  const CELL_SIZE = 13;

  return (
    <div className="rounded-lg border bg-card p-5">
      <div className="flex items-center gap-2 mb-4">
        <BarChart3Icon className="size-4 text-muted-foreground" />
        <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
          {title}
        </h3>
      </div>

      <div className="overflow-x-auto">
        <div className="inline-block">
          <div className="flex ml-7 mb-1">
            {monthLabels.map((m, i) => (
              <div
                key={i}
                className="text-[10px] text-muted-foreground"
                style={{
                  position: "absolute",
                  marginLeft: `${m.weekIndex * CELL_SIZE}px`,
                }}
              >
                {m.label}
              </div>
            ))}
          </div>

          <div className="flex">
            <div className="flex flex-col mr-1">
              {[0, 1, 2, 3, 4, 5, 6].map((dayIndex) => (
                <div
                  key={dayIndex}
                  className="text-[10px] text-muted-foreground flex items-center justify-end"
                  style={{ width: "24px", height: `${CELL_SIZE}px` }}
                >
                  {WEEKDAY_LABELS[dayIndex]}
                </div>
              ))}
            </div>
            {weeks.map((week, weekIndex) => (
              <div key={weekIndex} className="flex flex-col">
                {week.map((date, dayIndex) => {
                  if (!date) {
                    return (
                      <div
                        key={dayIndex}
                        style={{ width: `${CELL_SIZE}px`, height: `${CELL_SIZE}px` }}
                      />
                    );
                  }
                  const count = activityMap.get(date) || 0;
                  const level = getIntensityLevel(count, maxCount);
                  return (
                    <div
                      key={dayIndex}
                      className={`rounded-[3px] ${getIntensityClass(level)}`}
                      style={{ width: `${CELL_SIZE}px`, height: `${CELL_SIZE}px` }}
                      title={`${date}: ${count}`}
                    />
                  );
                })}
              </div>
            ))}
          </div>

          <div className="flex items-center gap-1 mt-3 ml-7">
            <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.less}</span>
            {[0, 1, 2, 3, 4].map((level) => (
              <div
                key={level}
                className={`rounded-[3px] ${getIntensityClass(level)}`}
                style={{ width: `${CELL_SIZE}px`, height: `${CELL_SIZE}px` }}
              />
            ))}
            <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.more}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
