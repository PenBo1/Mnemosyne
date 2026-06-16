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

const COLORS = [
  "#161b22",
  "#0e4429",
  "#006d32",
  "#26a641",
  "#39d353",
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
const LABEL_W = 28;

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
        <CardHeader className="pb-3"><CardTitle className="text-sm font-medium">{title}</CardTitle></CardHeader>
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
          <CardTitle className="text-sm font-medium">{title}</CardTitle>
          <Badge variant="secondary" className="text-xs">{total.toLocaleString()} {t.dashboard.heatmap.contributions}</Badge>
        </div>
      </CardHeader>
      <CardContent className="overflow-x-auto">
        <div style={{ display: "inline-flex", flexDirection: "column", gap: GAP }}>
          {/* month labels */}
          <div style={{ display: "flex", marginLeft: LABEL_W }}>
            {months.map((m, i) => {
              const nextCol = i + 1 < months.length ? months[i + 1]!.col : weeks.length;
              return (
                <div key={i} style={{ width: (nextCol - m.col) * (CELL + GAP), fontSize: 10, color: "#8b949e" }}>
                  {m.label}
                </div>
              );
            })}
          </div>

          {/* grid */}
          <div style={{ display: "flex", gap: GAP }}>
            <div style={{ display: "flex", flexDirection: "column", gap: GAP, marginRight: 4 }}>
              {DOW.map((l, i) => (
                <div key={i} style={{ width: LABEL_W, height: CELL, fontSize: 10, color: "#8b949e", display: "flex", alignItems: "center", justifyContent: "flex-end", paddingRight: 4 }}>{l}</div>
              ))}
            </div>
            {weeks.map((w, wi) => (
              <div key={wi} style={{ display: "flex", flexDirection: "column", gap: GAP }}>
                {w.map((d, di) => (
                  <div
                    key={di}
                    title={d ? `${d}: ${map.get(d) || 0}` : ""}
                    style={{
                      width: CELL, height: CELL, borderRadius: 2,
                      backgroundColor: d ? getColor(map.get(d) || 0, max) : "transparent",
                    }}
                  />
                ))}
              </div>
            ))}
          </div>

          {/* legend */}
          <div style={{ display: "flex", alignItems: "center", gap: 4, marginLeft: LABEL_W, marginTop: 4 }}>
            <span style={{ fontSize: 10, color: "#8b949e" }}>{t.dashboard.heatmap.less}</span>
            {COLORS.map((c, i) => (
              <div key={i} style={{ width: CELL, height: CELL, borderRadius: 2, backgroundColor: c }} />
            ))}
            <span style={{ fontSize: 10, color: "#8b949e" }}>{t.dashboard.heatmap.more}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
