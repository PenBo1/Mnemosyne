// ── Git 类型定义 ────────────────────────────────────────────────────

export interface Commit {
  id: string;
  short_id: string;
  author: string;
  author_email: string;
  time: number;
  message: string;
}

export type FileStatusType =
  | "unmodified"
  | "added"
  | "modified"
  | "deleted"
  | "untracked"
  | "renamed"
  | "copied"
  | "conflicted";

export interface FileChange {
  path: string;
  status: FileStatusType;
}

export interface GitStatus {
  branch: string;
  files: FileChange[];
  ahead: number;
  behind: number;
  staged: number;
  unstaged: number;
}

export interface FileDiff {
  path: string;
  additions: number;
  deletions: number;
  binary: boolean;
}

export interface Diff {
  files: FileDiff[];
  total_additions: number;
  total_deletions: number;
}

export interface GitConfig {
  user_name: string | null;
  user_email: string | null;
  custom: Record<string, string>;
}

export interface GitInitResult {
  initialized: boolean;
  path: string;
}

export type RollbackMode = "soft" | "mixed" | "hard";