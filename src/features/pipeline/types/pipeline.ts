// 书籍配置（对应 Rust BookConfig）
export interface PipelineBook {
  id: string;
  title: string;
  platform: "tomato" | "feilu" | "qidian" | "other";
  genre: string;
  status: "incubating" | "outlining" | "active" | "paused" | "completed" | "dropped";
  target_chapters: number;
  chapter_word_count: number;
  language?: "zh" | "en";
  created_at: string;
  updated_at: string;
  parent_book_id?: string;
  fanfic_mode?: "canon" | "au" | "ooc" | "cp";
  writing?: {
    review_mode?: "auto" | "manual";
    revision_gate?: "strict" | "lenient" | "always";
  };
}

// 章节信息
export interface PipelineChapter {
  number: number;
  title: string;
  status: string;
  word_count: number;
  content?: string;
  created_at?: string;
  updated_at?: string;
}

// 真相文件种类
export type TruthFileKind =
  | "current_state" | "pending_hooks" | "chapter_summaries"
  | "volume_map" | "roles" | "book_rules" | "story_frame";

// Pipeline 结果类型
export interface PlanChapterResult {
  book_id: string;
  chapter_number: number;
  intent_path: string;
}

export interface ComposeChapterResult {
  book_id: string;
  chapter_number: number;
  context_path: string;
  rule_stack_path: string;
}

export interface DraftResult {
  book_id: string;
  chapter_number: number;
  draft_path: string;
  word_count: number;
}

export interface AuditResult {
  book_id: string;
  chapter_number: number;
  passed: boolean;
  score: number;
  issues: Array<{
    dimension: string;
    severity: "critical" | "major" | "minor" | "info";
    description: string;
    repair_scope?: "line" | "paragraph" | "section" | "full";
  }>;
}

export interface ReviseResult {
  book_id: string;
  chapter_number: number;
  revised_path: string;
  word_count: number;
  audit_score: number;
  gate_applied: "strict" | "lenient" | "always";
}

export interface ChapterPipelineResult {
  book_id: string;
  chapter_number: number;
  draft_path: string;
  word_count: number;
  final_audit_score: number;
  review_iterations: number;
}

// 调度器
export interface SchedulerStatus {
  running: boolean;
  config: SchedulerConfigSnapshot;
  paused_books: string[];
}

export interface SchedulerConfigSnapshot {
  write_cron: string;
  radar_cron: string;
  max_concurrent_books: number;
  chapters_per_cycle: number;
  max_chapters_per_day: number;
}

export interface SchedulerEvent {
  type: "cycle_started" | "chapter_started" | "chapter_completed" | "chapter_failed" | "book_paused";
  book_id?: string;
  chapter_number?: number;
  error?: string;
  timestamp: string;
}

// 创建书籍请求
export interface CreateBookRequest {
  title: string;
  platform: string;
  genre: string;
  target_chapters?: number;
  chapter_word_count?: number;
  language?: "zh" | "en";
  author_intent?: string;
  external_context?: string;
}
