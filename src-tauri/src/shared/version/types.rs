//! ═══════════════════════════════════════════════════════════════════════════
//! 版本类型 - 章节版本与差异类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ── 修订模式 ──────────────────────────────────────────────────────────────────

/// 版本修订模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RevisionMode {
    /// 小修订（纠错、润色）
    #[default]
    Minor,
    /// 大修订（结构调整）
    Major,
    /// 重写（完全重写）
    Rewrite,
    /// 自动（系统判定）
    Auto,
}

impl std::fmt::Display for RevisionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RevisionMode::Minor => write!(f, "minor"),
            RevisionMode::Major => write!(f, "major"),
            RevisionMode::Rewrite => write!(f, "rewrite"),
            RevisionMode::Auto => write!(f, "auto"),
        }
    }
}

impl FromStr for RevisionMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "minor" => Ok(RevisionMode::Minor),
            "major" => Ok(RevisionMode::Major),
            "rewrite" => Ok(RevisionMode::Rewrite),
            "auto" => Ok(RevisionMode::Auto),
            _ => Err(format!("Invalid revision mode: {}", s)),
        }
    }
}

// ── 章节版本 ──────────────────────────────────────────────────────────────────

/// 章节版本快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVersion {
    /// 版本 ID
    pub id: String,
    /// 小说 ID
    pub novel_id: String,
    /// 章节编号
    pub chapter_number: u32,
    /// 版本号（递增）
    pub version_number: u32,
    /// 章节内容
    pub content: String,
    /// 内容哈希（用于快速比较）
    pub content_hash: String,
    /// 字数统计
    pub word_count: u32,
    /// 修订模式
    pub revision_mode: RevisionMode,
    /// 修订原因
    pub revision_reason: String,
    /// 创建时间
    pub created_at: String,
}

/// 创建版本请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    /// 小说 ID
    pub novel_id: String,
    /// 章节编号
    pub chapter_number: u32,
    /// 章节内容
    pub content: String,
    /// 内容哈希
    pub content_hash: String,
    /// 字数统计
    pub word_count: u32,
    /// 修订模式
    pub revision_mode: RevisionMode,
    /// 修订原因
    pub revision_reason: String,
}

// ── 差异类型 ──────────────────────────────────────────────────────────────────

/// 差异行类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    /// 新增行
    Added,
    /// 删除行
    Removed,
    /// 未变更行（上下文）
    Context,
}

/// 差异行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    /// 行类型
    pub line_type: DiffLineType,
    /// 行内容
    pub content: String,
    /// 旧文件行号（context/removed 有值，added 为 None）
    pub old_number: Option<u32>,
    /// 新文件行号（context/added 有值，removed 为 None）
    pub new_number: Option<u32>,
}

/// 差异块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// 旧文件起始行
    pub old_start: u32,
    /// 旧文件行数
    pub old_lines: u32,
    /// 新文件起始行
    pub new_start: u32,
    /// 新文件行数
    pub new_lines: u32,
    /// 行列表
    pub lines: Vec<DiffLine>,
}

/// 差异统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    /// 新增行数
    pub lines_added: u32,
    /// 删除行数
    pub lines_removed: u32,
    /// 修改行数（按行计，近似）
    pub lines_modified: u32,
    /// 新增字符数
    pub chars_added: u32,
    /// 删除字符数
    pub chars_removed: u32,
}

/// 行级 diff 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDiffResult {
    /// 差异块列表
    pub hunks: Vec<DiffHunk>,
    /// 统计
    pub stats: DiffStats,
}