import { ipc } from "@/lib/ipc";

export interface DailyActivity {
  date: string;
  count: number;
}

export interface ActivityData {
  chatActivity: DailyActivity[];
  novelActivity: DailyActivity[];
}

export async function getDailyActivity(): Promise<ActivityData> {
  return ipc<ActivityData>("get_daily_activity");
}