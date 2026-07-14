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
import { useI18n } from "@/locales/i18n";
import type { DailyActivity } from "@/features/stats/types";

function getActivityMap(activities: DailyActivity[]): Map<string, number> {
  const map = new Map<string, number>();
  for (const act of activities) {
    map.set(act.date, act.count);
  }
  return map;
}

const COLORS = [
  "rgba(224, 226, 242, 0.04)",
  "rgba(50, 240, 140, 0.15)",
  "rgba(50, 240, 140, 0.30)",
  "rgba(50, 240, 140, 0.55)",
  "rgba(50, 240, 140, 0.80)",
];

function getColor(count: number, max: number): string {
  if (count === 0) return COLORS[0];
  const ratio = count / max;
  if (ratio < 0.25) return COLORS[1];
  if (ratio < 0.5) return COLORS[2];
  if (ratio < 0.75) return COLORS[3];
  return COLORS[4];
}

const CELL = 11;
const GAP = 3;

function buildGrid(dates: string[], activityMap: Map<string, number>) {
  const weeks: (string | null)[][] = [];
  let week: (string | null)[] = [];
  for (const d of dates) {
    const dow = new Date(d + "T00:00:00").getDay();
    if (dow === 0 && week.length > 0) {
      while (week.length < 7) week.push(null);
      weeks.push(week);
      week = [];
    }
    week.push(d);
  }
  if (week.length > 0) {
    while (week.length < 7) week.push(null);
    weeks.push(week);
  }

  const months: { label: string; col: number }[] = [];
  let lastM = -1;
  for (let c = 0; c < weeks.length; c++) {
    for (const d of weeks[c]) {
      if (d) {
        const m = new Date(d + "T00:00:00").getMonth();
        if (m !== lastM) {
          months.push({ label: new Date(d + "T00:00:00").toLocaleDateString("en-US", { month: "short" }), col: c });
          lastM = m;
        }
        break;
      }
    }
  }

  const max = Math.max(1, ...dates.map((d) => activityMap.get(d) || 0));
  return { weeks, months, max };
}

const DOW = ["", "Mon", "", "Wed", "", "Fri", ""];

export function HeatmapGrid({ data, title, emptyMessage }: { data: DailyActivity[]; title: string; emptyMessage: string }) {
  const { t } = useI18n();
  const map = useMemo(() => getActivityMap(data), [data]);

  const dates = useMemo(() => {
    const r: string[] = [];
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const s = new Date(today);
    s.setFullYear(s.getFullYear() - 1);
    s.setDate(s.getDate() + 1);
    const c = new Date(s);
    while (c <= today) {
      r.push(c.toISOString().split("T")[0]);
      c.setDate(c.getDate() + 1);
    }
    return r;
  }, []);

  const { weeks, months, max } = useMemo(() => buildGrid(dates, map), [dates, map]);
  const total = data.reduce((s, d) => s + d.count, 0);

  if (data.length === 0) {
    return (
      <Card>
        <CardHeader className="pb-3"><CardTitle className="trae-card-eyebrow">{title}</CardTitle></CardHeader>
        <CardContent>
          <Empty><EmptyHeader><EmptyMedia><EmptyTitle>{emptyMessage}</EmptyTitle></EmptyMedia>
            <EmptyDescription>{t.dashboard.heatmap.startHint}</EmptyDescription>
          </EmptyHeader></Empty>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="trae-card-eyebrow">{title}</CardTitle>
          <Badge variant="secondary" className="text-xs">{total.toLocaleString()} {t.dashboard.heatmap.contributions}</Badge>
        </div>
      </CardHeader>
      <CardContent className="overflow-x-auto">
        <div className="inline-flex flex-col gap-[3px]">
          {/* month labels */}
          <div className="flex ml-[28px]">
            {months.map((m, i) => {
              const nextCol = i + 1 < months.length ? months[i + 1]!.col : weeks.length;
              return (
                <div
                  key={i}
                  className="text-[10px] text-muted-foreground"
                  style={{ width: (nextCol - m.col) * (CELL + GAP) }}
                >
                  {m.label}
                </div>
              );
            })}
          </div>

          {/* grid */}
          <div className="flex gap-[3px]">
            <div className="flex flex-col gap-[3px] mr-1">
              {DOW.map((l, i) => (
                <div
                  key={i}
                  className="w-[28px] h-[11px] text-[10px] text-muted-foreground flex items-center justify-end pr-1"
                >
                  {l}
                </div>
              ))}
            </div>
            {weeks.map((w, wi) => (
              <div key={wi} className="flex flex-col gap-[3px]">
                {w.map((d, di) => (
                  <div
                    key={di}
                    title={d ? `${d}: ${map.get(d) || 0}` : ""}
                    className="w-[11px] h-[11px] rounded-[2px]"
                    style={{
                      backgroundColor: d ? getColor(map.get(d) || 0, max) : "transparent",
                    }}
                  />
                ))}
              </div>
            ))}
          </div>

          {/* legend */}
          <div className="flex items-center gap-1 ml-[28px] mt-1">
            <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.less}</span>
            {COLORS.map((c, i) => (
              <div
                key={i}
                className="w-[11px] h-[11px] rounded-[2px]"
                style={{ backgroundColor: c }}
              />
            ))}
            <span className="text-[10px] text-muted-foreground">{t.dashboard.heatmap.more}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
