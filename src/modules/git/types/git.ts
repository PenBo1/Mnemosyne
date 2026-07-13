// ── Git ────────────────────────────────────────────────────

export interface Commit {
  hash: string;
  short_hash: string;
  author: string;
  email: string;
  date: string;
  message: string;
}

export type FileChangeStatus = "modified" | "added" | "deleted" | "renamed";

export interface FileChange {
  path: string;
  status: FileChangeStatus;
  staged: boolean;
}

export interface GitStatus {
  branch: string;
  staged: FileChange[];
  unstaged: FileChange[];
  untracked: string[];
  is_clean: boolean;
}

export interface FileDiff {
  path: string;
  additions: number;
  deletions: number;
  patch: string;
}

export interface Diff {
  files: FileDiff[];
}

export interface GitConfig {
  user_name: string | null;
  user_email: string | null;
  auto_stage: boolean;
  commit_message_template: string | null;
  enable_remote: boolean;
}

export interface InstallResult {
  success: boolean;
  message: string;
  version: string | null;
}

export interface GitInitResult {
  initialized: boolean;
  path: string;
}

export type RollbackMode = "Soft" | "Hard";
