// 使用统计服务 —— 封装 IPC 调用

import { ipc } from "@/services/ipc";

export interface UsageStats {
  activeDays: number;
  currentStreak: number;
  heatmap: HeatmapCell[];
  tokenTrend: TokenTrendPoint[];
  sessionCount: number;
  messageCount: number;
  totalTokens: number;
  mostUsedModel: MostUsedModel | null;
  modelUsage: ModelUsageRow[];
}

export interface HeatmapCell {
  date: string;
  count: number;
}

export interface TokenTrendPoint {
  date: string;
  tokens: number;
}

export interface MostUsedModel {
  model: string;
  calls: number;
  tokens: number;
}

export interface ModelUsageRow {
  provider: string | null;
  model: string;
  calls: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  ratio: number;
}

export type TimeRange = 7 | 30;

export async function getUsageStats(days: TimeRange): Promise<UsageStats> {
  return ipc<UsageStats>("get_usage_stats", { days });
}