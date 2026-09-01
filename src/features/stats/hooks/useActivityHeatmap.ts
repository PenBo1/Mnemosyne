/**
 * ═══════════════════════════════════════════════════════════════════════════
 * useActivityHeatmap - 活跃度热力图数据获取 Hook
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useEffect, useCallback } from "react";
import { getDailyActivity, type DailyActivityResult } from "../services/activity";
import type { HeatmapData, HeatmapMetric, HeatmapEntry } from "../components/ActivityHeatmap";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface UseActivityHeatmapResult {
  /** 热力图数据 */
  data: HeatmapData | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 刷新数据 */
  refresh: () => Promise<void>;
  /** 当前指标 */
  metric: HeatmapMetric;
  /** 设置指标 */
  setMetric: (metric: HeatmapMetric) => void;
  /** 选中的日期 */
  selectedDate: string | null;
  /** 设置选中日期 */
  setSelectedDate: (date: string | null) => void;
}

// ── 工具函数 ────────────────────────────────────────────────────────────────

/**
 * 计算热度等级 (0-4)
 */
function calculateLevel(value: number, maxValue: number): number {
  if (value === 0) return 0;
  if (maxValue === 0) return 0;

  const ratio = value / maxValue;
  if (ratio >= 0.75) return 4;
  if (ratio >= 0.5) return 3;
  if (ratio >= 0.25) return 2;
  return 1;
}

/**
 * 将原始数据转换为热力图数据
 */
function transformToHeatmap(
  rawData: DailyActivityResult[],
  metric: HeatmapMetric
): HeatmapData {
  if (!rawData || rawData.length === 0) {
    return { entries: [] };
  }

  // 获取最大值用于计算等级
  const values = rawData.map((d) => {
    switch (metric) {
      case "sessions":
        return d.sessions;
      case "output_tokens":
      case "tokens":
        return d.tokens;
      default:
        return d.messages;
    }
  });
  const maxValue = Math.max(...values);

  const entries: HeatmapEntry[] = rawData.map((d) => {
    const value = (() => {
      switch (metric) {
        case "sessions":
          return d.sessions;
        case "output_tokens":
        case "tokens":
          return d.tokens;
        default:
          return d.messages;
      }
    })();

    return {
      date: d.date,
      value,
      level: calculateLevel(value, maxValue),
    };
  });

  return {
    entries,
    entries_from: rawData.length > 0 ? new Date(rawData[0]!.date).getTime() : undefined,
  };
}

// ── useActivityHeatmap Hook ────────────────────────────────────────────────

/**
 * 活跃度热力图数据 Hook
 */
export function useActivityHeatmap(
  days: number = 365
): UseActivityHeatmapResult {
  const [data, setData] = useState<HeatmapData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [metric, setMetric] = useState<HeatmapMetric>("messages");
  const [selectedDate, setSelectedDate] = useState<string | null>(null);

  // 获取数据
  const fetchData = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      // 调用封装的 service 获取每日活跃数据
      const result = await getDailyActivity(days);

      const heatmapData = transformToHeatmap(result || [], metric);
      setData(heatmapData);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, [days, metric]);

  // 初始加载和指标变更时刷新
  useEffect(() => {
    fetchData();
  }, [fetchData]);

  return {
    data,
    isLoading,
    error,
    refresh: fetchData,
    metric,
    setMetric,
    selectedDate,
    setSelectedDate,
  };
}

// ── useHeatmapStats Hook ───────────────────────────────────────────────────

interface HeatmapStats {
  totalDays: number;
  activeDays: number;
  totalMessages: number;
  totalSessions: number;
  totalTokens: number;
  streakDays: number;
  maxStreak: number;
}

/**
 * 计算热力图统计数据
 */
export function useHeatmapStats(data: HeatmapData | null): HeatmapStats {
  const stats: HeatmapStats = {
    totalDays: 0,
    activeDays: 0,
    totalMessages: 0,
    totalSessions: 0,
    totalTokens: 0,
    streakDays: 0,
    maxStreak: 0,
  };

  if (!data?.entries) return stats;

  stats.totalDays = data.entries.length;
  stats.activeDays = data.entries.filter((e) => e.value > 0).length;
  stats.totalMessages = data.entries.reduce((sum, e) => sum + e.value, 0);

  // 计算连续活跃天数
  let currentStreak = 0;
  let maxStreak = 0;
  for (const entry of data.entries) {
    if (entry.value > 0) {
      currentStreak++;
      maxStreak = Math.max(maxStreak, currentStreak);
    } else {
      currentStreak = 0;
    }
  }
  stats.streakDays = currentStreak;
  stats.maxStreak = maxStreak;

  return stats;
}