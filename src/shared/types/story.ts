// ── Story auxiliary: hooks, chapter summaries, facts ──────

// 注意：后端 HookStatus 用 #[serde(rename_all = "snake_case")]，
// 序列化为 snake_case，前端类型必须匹配。
export type HookStatus = "open" | "progressing" | "deferred" | "resolved";

export interface HookRecord {
  hook_id: string;
  name: string;
  hook_type: string;
  start_chapter: number;
  status: HookStatus;
  expected_payoff: string;
  last_advanced_chapter: number;
  core_hook: boolean;
  created_at: string;
  updated_at: string;
}

export interface ChapterSummary {
  chapter: number;
  title: string;
  characters: string[];
  events: string[];
  state_changes: string[];
  hook_activity: string[];
  mood: string;
  chapter_type: string;
  created_at: string;
}

export interface StoryFact {
  fact_id: string;
  subject: string;
  predicate: string;
  object: string;
  valid_from_chapter: number;
  valid_until_chapter: number | null;
  source_chapter: number;
  created_at: string;
}

/** 故事状态快照 */
export interface StoryState {
  current_chapter: number;
  total_words: number;
  hooks: HookRecord[];
  summaries: ChapterSummary[];
  facts: StoryFact[];
}
