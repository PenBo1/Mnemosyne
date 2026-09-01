/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ActivityHeatmap - GitHub 风格活跃度热力图组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import React, { useState, useMemo, useCallback } from "react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";

// ── 常量定义 ────────────────────────────────────────────────────────────────

const CELL_SIZE = 12;
const CELL_GAP = 3;
const CELL_STEP = CELL_SIZE + CELL_GAP;
const LABEL_WIDTH = 36;
const HEADER_HEIGHT = 20;

const DAY_LABELS = ["", "周一", "", "周三", "", "周五", ""];
const DAY_LABELS_EN = ["", "Mon", "", "Wed", "", "Fri", ""];

const LEVEL_COLORS_LIGHT = [
  "var(--color-muted)",
  "#9be9a8",
  "#40c463",
  "#30a14e",
  "#216e39",
];

const LEVEL_COLORS_DARK = [
  "var(--color-muted)",
  "#0e4429",
  "#006d32",
  "#26a641",
  "#39d353",
];

// ── 类型定义 ────────────────────────────────────────────────────────────────

export type HeatmapMetric = "messages" | "sessions" | "output_tokens" | "tokens";

export interface HeatmapEntry {
  /** 日期 (YYYY-MM-DD) */
  date: string;
  /** 数值 */
  value: number;
  /** 热度等级 (0-4) */
  level: number;
}

export interface HeatmapData {
  /** 热力图数据条目 */
  entries: HeatmapEntry[];
  /** 数据起始日期 */
  entries_from?: number;
}

interface DayCell {
  date: string;
  value: number;
  level: number;
  dayOfWeek: number;
}

interface ActivityHeatmapProps {
  /** 热力图数据 */
  data: HeatmapData | null;
  /** 当前选中的指标 */
  metric?: HeatmapMetric;
  /** 选中日期回调 */
  onSelectDate?: (date: string) => void;
  /** 选中的日期 */
  selectedDate?: string | null;
  /** 是否显示指标切换器 */
  showMetricToggle?: boolean;
  /** 指标变更回调 */
  onMetricChange?: (metric: HeatmapMetric) => void;
  /** 是否支持 output_tokens 指标 */
  supportsOutputTokens?: boolean;
  /** 错误信息 */
  error?: string | null;
  /** 重试回调 */
  onRetry?: () => void;
  /** 自定义类名 */
  className?: string;
}

// ── 工具函数 ────────────────────────────────────────────────────────────────

function getLevelColor(level: number, isDark: boolean): string {
  const colors = isDark ? LEVEL_COLORS_DARK : LEVEL_COLORS_LIGHT;
  return colors[level] ?? colors[0]!;
}

function getMetricLabel(metric: HeatmapMetric, locale: string): string {
  const labels: Record<HeatmapMetric, Record<string, string>> = {
    messages: { zh: "消息数", en: "Messages" },
    sessions: { zh: "会话数", en: "Sessions" },
    output_tokens: { zh: "输出 Tokens", en: "Output Tokens" },
    tokens: { zh: "Tokens", en: "Tokens" },
  };
  return labels[metric]?.[locale] ?? labels[metric]?.en ?? metric;
}

// ── ActivityHeatmap 组件 ────────────────────────────────────────────────────

export function ActivityHeatmap({
  data,
  metric = "messages",
  onSelectDate,
  selectedDate,
  showMetricToggle = true,
  onMetricChange,
  error,
  onRetry,
  className,
}: ActivityHeatmapProps): React.ReactElement {
  const { t, locale } = useI18n();
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    text: string;
  } | null>(null);

  const isDark = document.documentElement.classList.contains("dark");

  // 计算网格数据
  const grid = useMemo(() => {
    const entries = data?.entries;
    if (!entries || entries.length === 0) {
      return { cols: [] as DayCell[][], months: [] as { col: number; label: string }[] };
    }

    const cols: DayCell[][] = [];
    let currentCol: DayCell[] = [];
    let lastMonth = "";
    const monthLabels: { col: number; label: string }[] = [];

    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i]!;
      const d = new Date(entry.date + "T00:00:00");
      const dow = d.getDay();
      const cell: DayCell = {
        date: entry.date,
        value: entry.value,
        level: entry.level,
        dayOfWeek: dow,
      };

      // 周日开始新列
      if (i > 0 && dow === 0) {
        cols.push(currentCol);
        currentCol = [];
      }

      const month = d.toLocaleString(locale === "zh" ? "zh-CN" : "en", { month: "short" });
      if (month !== lastMonth && dow <= 3) {
        monthLabels.push({ col: cols.length, label: month });
        lastMonth = month;
      }

      currentCol.push(cell);
    }
    if (currentCol.length > 0) {
      cols.push(currentCol);
    }

    return { cols, months: monthLabels };
  }, [data, locale]);

  // SVG 尺寸
  const svgWidth = grid.cols.length * CELL_STEP + LABEL_WIDTH + 4;
  const svgHeight = 7 * CELL_STEP + HEADER_HEIGHT + 4;

  // 处理单元格悬停
  const handleCellHover = useCallback((e: React.MouseEvent<SVGRectElement>, cell: DayCell) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const d = new Date(cell.date + "T00:00:00");
    const dateLabel = d.toLocaleDateString(locale === "zh" ? "zh-CN" : "en", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
    const metricLabel = getMetricLabel(metric, locale);
    setTooltip({
      x: rect.left + rect.width / 2,
      y: rect.top - 4,
      text: `${dateLabel}: ${cell.value.toLocaleString()} ${metricLabel}`,
    });
  }, [metric, locale]);

  // 处理单元格点击
  const handleCellClick = useCallback((cell: DayCell) => {
    if (cell.value > 0 || selectedDate === cell.date) {
      onSelectDate?.(cell.date);
    }
  }, [onSelectDate, selectedDate]);

  const dayLabels = locale === "zh" ? DAY_LABELS : DAY_LABELS_EN;

  return (
    <div className={cn("relative", className)}>
      {/* 标题和指标切换器 */}
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-xs font-semibold text-foreground">
          {(t as unknown as { analyticsActivityTitle?: string }).analyticsActivityTitle || "活跃度"}
        </h3>
        {showMetricToggle && (
          <div className="flex gap-0.5">
            <button
              className={cn(
                "h-5 px-2 rounded text-[10px] font-medium transition-colors",
                metric === "messages"
                  ? "bg-muted text-foreground"
                  : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
              )}
              onClick={() => onMetricChange?.("messages")}
            >
              {getMetricLabel("messages", locale)}
            </button>
            <button
              className={cn(
                "h-5 px-2 rounded text-[10px] font-medium transition-colors",
                metric === "sessions"
                  ? "bg-muted text-foreground"
                  : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
              )}
              onClick={() => onMetricChange?.("sessions")}
            >
              {getMetricLabel("sessions", locale)}
            </button>
            <button
              className={cn(
                "h-5 px-2 rounded text-[10px] font-medium transition-colors",
                metric === "tokens"
                  ? "bg-muted text-foreground"
                  : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
              )}
              onClick={() => onMetricChange?.("tokens")}
            >
              {getMetricLabel("tokens", locale)}
            </button>
          </div>
        )}
      </div>

      {/* 错误状态 */}
      {error ? (
        <div className="flex items-center gap-2 p-3 text-sm text-destructive">
          <span>{error}</span>
          {onRetry && (
            <button
              className="px-2 py-0.5 border border-current rounded text-xs hover:bg-destructive/10"
              onClick={onRetry}
            >
              {(t as unknown as { sharedRetry?: string }).sharedRetry || "重试"}
            </button>
          )}
        </div>
      ) : grid.cols.length > 0 ? (
        <>
          {/* 热力图 SVG */}
          <div className="overflow-x-auto pb-1">
            <svg
              width={svgWidth}
              height={svgHeight}
              className="block mx-auto"
            >
              {/* 周标签 */}
              {dayLabels.map((label, i) =>
                label ? (
                  <text
                    key={i}
                    x={LABEL_WIDTH - 4}
                    y={i * CELL_STEP + HEADER_HEIGHT + CELL_SIZE - 1}
                    className="text-[9px] fill-muted-foreground"
                    textAnchor="end"
                  >
                    {label}
                  </text>
                ) : null
              )}

              {/* 月标签 */}
              {grid.months.map((m, idx) => (
                <text
                  key={idx}
                  x={m.col * CELL_STEP + LABEL_WIDTH}
                  y={HEADER_HEIGHT - 4}
                  className="text-[9px] fill-muted-foreground"
                >
                  {m.label}
                </text>
              ))}

              {/* 热力图单元格 */}
              {grid.cols.map((col, colIdx) =>
                col.map((cell) => (
                  <rect
                    key={cell.date}
                    x={colIdx * CELL_STEP + LABEL_WIDTH}
                    y={cell.dayOfWeek * CELL_STEP + HEADER_HEIGHT}
                    width={CELL_SIZE}
                    height={CELL_SIZE}
                    rx={2}
                    fill={getLevelColor(cell.level, isDark)}
                    className={cn(
                      "transition-opacity",
                      cell.value > 0 || selectedDate === cell.date
                        ? "cursor-pointer hover:opacity-80"
                        : "cursor-default"
                    )}
                    stroke={selectedDate === cell.date ? "hsl(var(--foreground))" : undefined}
                    strokeWidth={selectedDate === cell.date ? 2 : 0}
                    onMouseEnter={(e) => handleCellHover(e, cell)}
                    onMouseLeave={() => setTooltip(null)}
                    onClick={() => handleCellClick(cell)}
                  />
                ))
              )}
            </svg>
          </div>

          {/* Tooltip */}
          {tooltip && (
            <div
              className="fixed z-50 px-2 py-1 text-[10px] bg-foreground text-background rounded whitespace-nowrap pointer-events-none"
              style={{
                left: tooltip.x,
                top: tooltip.y,
                transform: "translateX(-50%) translateY(-100%)",
              }}
            >
              {tooltip.text}
            </div>
          )}
        </>
      ) : (
        <div className="py-6 text-sm text-center text-muted-foreground">
          {(t as unknown as { sharedNoDataForPeriod?: string }).sharedNoDataForPeriod || "该时间段内暂无数据"}
        </div>
      )}
    </div>
  );
}