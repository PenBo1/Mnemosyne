/**
 * Novel prompt type definitions
 *
 * These types correspond to the Rust backend services/story/prompts.rs data structures.
 * Frontend-only: IPC communication types, no business logic.
 */

export type Language = "zh" | "en";

export type NarrativePerson = "first" | "third";

export type WriterMode = "full" | "creative";

export type FanficMode = "canon" | "au" | "ooc" | "cp";

export interface GenreConfig {
  id: string;
  name: string;
  language: Language;
  fatigue_words: string[];
  pacing_rule: string;
  chapter_types: string[];
  numerical_system: boolean;
  power_scaling: boolean;
}

export interface LengthSpec {
  target: number;
  soft_min: number;
  soft_max: number;
  hard_min: number;
  hard_max: number;
}

export interface BookRules {
  protagonist_name?: string;
  personality_lock: string[];
  behavioral_constraints: string[];
  prohibitions: string[];
  narrative_person?: NarrativePerson;
  enable_full_cast_tracking: boolean;
  genre_forbidden: string[];
}

export interface WriterPromptParams {
  language: Language;
  genre: GenreConfig;
  chapter_words: number;
  book_rules?: BookRules;
  chapter_number?: number;
  mode?: WriterMode;
}

export interface PlannerPromptParams {
  language: Language;
}

export interface SettlerPromptParams {
  language: Language;
}

export interface ObserverPromptParams {
  language: Language;
}

export interface ShortFictionOutlineParams {
  direction: string;
  chapter_count: number;
  chars_per_chapter: number;
  reference?: string;
}

export interface ShortFictionWriterParams {
  direction: string;
  outline_markdown: string;
  chapter_count: number;
  chars_per_chapter: number;
}

export interface FanficCanonParams {
  fanfic_canon: string;
  mode: FanficMode;
}

export interface PromptBuildResult {
  prompt: string;
  language: Language;
  section_count: number;
}

export interface PromptTemplate {
  id: string;
  name: string;
  description: string;
  category: PromptCategory;
  language: Language;
  content: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export type PromptCategory =
  | "writer"
  | "planner"
  | "settler"
  | "observer"
  | "short_fiction"
  | "fanfic"
  | "custom";

export interface PromptCategoryInfo {
  id: PromptCategory;
  name: string;
  description: string;
  icon: string;
}
