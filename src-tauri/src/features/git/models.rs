use serde::{Deserialize, Serialize};

/// A single git commit entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    pub date: String, // ISO 8601
    pub message: String,
}

/// Snapshot of the working tree state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus {
    pub branch: String,
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub untracked: Vec<String>,
    pub is_clean: bool,
}

/// A single file change in the working tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub status: String, // "modified" | "added" | "deleted" | "renamed"
    pub staged: bool,
}

/// Aggregated diff result across one or more files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub files: Vec<FileDiff>,
}

/// Per-file diff information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
    pub patch: String, // unified diff text
}

/// Per-repository git configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitConfig {
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub auto_stage: bool, // whether to stage all changes automatically before commit
    pub commit_message_template: Option<String>,
    pub enable_remote: bool, // default false
}

/// Result of an installation attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub version: Option<String>,
}

/// Rollback strategy for `git reset`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackMode {
    Soft, // git reset --soft (preserve working tree changes)
    Hard, // git reset --hard (discard working tree changes, high risk)
}

/// Result of `git init` on a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInitResult {
    pub initialized: bool, // true = newly initialized, false = already a git repo
    pub path: String,
}
