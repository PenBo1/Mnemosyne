// ── Novel / Book ───────────────────────────────────────────

import type { AuditResult } from "./audit";

export interface Novel {
  id: string;
  workspace_id: string;
  title: string;
  genre: string;
  platform: string;
  status: "drafting" | "paused" | "completed" | "archived";
  language: "zh" | "en";
  word_count: number;
  chapter_count: number;
  target_chapters: number;
  chapter_words: number;
  created_at: string;
  updated_at: string;
}

export interface BookConfig {
  id: string;
  title: string;
  genre: string;
  platform: string;
  status: "drafting" | "writing" | "reviewing" | "completed" | "paused";
  language: string;
  chapter_words: number;
  target_chapters: number;
  created_at: string;
  updated_at: string;
}

export interface WriteResult {
  chapter_number: number;
  title: string;
  content: string;
  word_count: number;
  audit: AuditResult;
}
