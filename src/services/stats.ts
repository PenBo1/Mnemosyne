import { ipc } from "@/lib/ipc";

export interface DailyActivity {
  date: string;
  count: number;
}

export interface ActivityData {
  chatActivity: DailyActivity[];
  novelActivity: DailyActivity[];
}

export interface StatsData {
  promptCount: number;
  novelCount: number;
  trendCount: number;
  totalWords: number;
}

export async function getStats(): Promise<StatsData> {
  return ipc<StatsData>("get_stats");
}

export async function getDailyActivity(): Promise<DailyActivity[]> {
  const data = await ipc<ActivityData>("get_daily_activity");
  return data.chatActivity;
}
