use serde::{Deserialize, Serialize};

/// Revision mode for chapter versions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionMode {
    Auto,
    Polish,
    Rewrite,
    Rework,
    SpotFix,
    Manual,
}

impl Default for RevisionMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl std::fmt::Display for RevisionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RevisionMode::Auto => write!(f, "auto"),
            RevisionMode::Polish => write!(f, "polish"),
            RevisionMode::Rewrite => write!(f, "rewrite"),
            RevisionMode::Rework => write!(f, "rework"),
            RevisionMode::SpotFix => write!(f, "spot_fix"),
            RevisionMode::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for RevisionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(RevisionMode::Auto),
            "polish" => Ok(RevisionMode::Polish),
            "rewrite" => Ok(RevisionMode::Rewrite),
            "rework" => Ok(RevisionMode::Rework),
            "spot_fix" => Ok(RevisionMode::SpotFix),
            "manual" => Ok(RevisionMode::Manual),
            _ => Err(format!("Unknown revision mode: {}", s)),
        }
    }
}

/// Chapter version - a snapshot of chapter content after revision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVersion {
    pub id: String,
    pub novel_id: String,
    pub chapter_number: u32,
    pub version_number: u32,
    pub content: String,
    pub content_hash: String,
    pub word_count: u32,
    pub revision_reason: String,
    pub revision_mode: RevisionMode,
    pub created_at: String,
}

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

/// Create version request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub novel_id: String,
    pub chapter_number: u32,
    pub content: String,
    pub revision_mode: RevisionMode,
    pub revision_reason: String,
}