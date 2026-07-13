// ── Radar ──────────────────────────────────────────────────

export interface RadarScan {
  id: string;
  market_summary: string;
  recommendations: RadarRecommendation[];
  raw_rankings: PlatformRankings[];
  created_at: string;
}

export interface RadarRecommendation {
  platform: string;
  genre: string;
  concept: string;
  confidence: number;
  reasoning: string;
  benchmark_titles: string[];
}

export interface PlatformRankings {
  platform: string;
  entries: RankingEntry[];
}

export interface RankingEntry {
  title: string;
  author: string;
  category: string;
  extra: string;
}
