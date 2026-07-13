// ── Chapter Version ────────────────────────────────────────

export type RevisionMode = "auto" | "polish" | "rewrite" | "rework" | "spot_fix" | "manual";

export interface ChapterVersion {
  id: string;
  novel_id: string;
  chapter_number: number;
  version_number: number;
  content: string;
  content_hash: string;
  word_count: number;
  revision_reason: string;
  revision_mode: RevisionMode;
  created_at: string;
}

export type DiffLineType = "added" | "removed" | "context";

export interface DiffLine {
  line_type: DiffLineType;
  content: string;
  old_number: number | null;
  new_number: number | null;
}

export interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLine[];
}

export interface DiffStats {
  lines_added: number;
  lines_removed: number;
  lines_modified: number;
  chars_added: number;
  chars_removed: number;
}

export interface LineDiffResult {
  hunks: DiffHunk[];
  stats: DiffStats;
}
