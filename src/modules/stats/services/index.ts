import { ipc } from "@/infrastructure/api";
import type { DailyActivity, ActivityData, StatsData } from "@/shared/types";

export async function getStats(): Promise<StatsData> {
  return ipc<StatsData>("get_stats");
}

export async function getDailyActivity(): Promise<DailyActivity[]> {
  const data = await ipc<ActivityData>("get_daily_activity");
  return data.chatActivity;
}

export * from "./ai-logs";
