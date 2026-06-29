// ── Stats ──────────────────────────────────────────────────

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
