use serde::{Deserialize, Serialize};

// RevisionMode、ChapterVersion、CreateVersionRequest 已下沉到
// crate::shared::version（修复 infra → features/version 反向依赖）。
// 这里通过 re-export 保持 `crate::features::version::RevisionMode` 等路径兼容。
pub use crate::shared::version::{RevisionMode, ChapterVersion, CreateVersionRequest};

/// Diff line type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Added,
    Removed,
    Context,
}

/// Single diff line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
}

/// Diff hunk - a contiguous block of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

/// Diff statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    pub lines_added: u32,
    pub lines_removed: u32,
    pub lines_modified: u32,
    pub chars_added: u32,
    pub chars_removed: u32,
}

/// Line-level diff result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineDiffResult {
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,
}