//! ═══════════════════════════════════════════════════════════════════════════
//! 管道类型 - 基础数据类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! book.json + chapters.json 等结构定义。
//!
//! zod schema → Rust serde 转换规则：
//! - zod enum → Rust enum + #[serde(rename_all = "kebab-case")]
//! - zod object → Rust struct
//! - zod optional → Option<T>
//! - zod default → #[serde(default)]

use serde::{Deserialize, Serialize};

// ── 书籍配置 ─────────────────────────────────────────────────

/// 发布平台
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Tomato,
    Feilu,
    Qidian,
    Other,
}

/// 书籍状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BookStatus {
    Incubating,
    Outlining,
    Active,
    Paused,
    Completed,
    Dropped,
}

/// 同人模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanficMode {
    Canon,
    Au,
    Ooc,
    Cp,
}

/// 章节评审模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChapterReviewMode {
    Auto,
    Manual,
}

/// 修订门控
/// - strict: 审计计数不恶化且至少一项改善才应用
/// - lenient: 审计计数不恶化即应用
/// - always: 总是应用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RevisionGate {
    Strict,
    Lenient,
    Always,
}

/// 写作配置（book.json writing 字段）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WritingConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_mode: Option<ChapterReviewMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_gate: Option<RevisionGate>,
}

/// 书籍配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub genre: String,
    pub status: BookStatus,
    #[serde(default = "default_target_chapters")]
    pub target_chapters: u32,
    #[serde(default = "default_chapter_word_count")]
    pub chapter_word_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_book_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fanfic_mode: Option<FanficMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing: Option<WritingConfig>,
}

fn default_target_chapters() -> u32 {
    200
}
fn default_chapter_word_count() -> u32 {
    3000
}

/// 解析章节评审模式：book.writing.review_mode 优先，否则 fallback
pub fn resolve_chapter_review_mode(
    book: &BookConfig,
    project_review_mode: Option<ChapterReviewMode>,
) -> ChapterReviewMode {
    book.writing
        .as_ref()
        .and_then(|w| w.review_mode)
        .or(project_review_mode)
        .unwrap_or(ChapterReviewMode::Auto)
}

/// 解析修订门控：book.writing.revision_gate 优先，否则 fallback
pub fn resolve_revision_gate(
    book: &BookConfig,
    project_revision_gate: Option<RevisionGate>,
) -> RevisionGate {
    book.writing
        .as_ref()
        .and_then(|w| w.revision_gate)
        .or(project_revision_gate)
        .unwrap_or(RevisionGate::Strict)
}

// ── 章节元数据 ───────────────────────────────────────────────

/// 语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Language {
    #[default]
    Zh,
    En,
}


/// 章节状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChapterStatus {
    CardGenerated,
    Drafting,
    Drafted,
    Auditing,
    AuditPassed,
    AuditFailed,
    StateDegraded,
    Revising,
    ReadyForReview,
    Approved,
    Rejected,
    Published,
    Imported,
}

/// token 用量
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

/// 章节元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMeta {
    pub number: u32,
    pub title: String,
    pub status: ChapterStatus,
    #[serde(default)]
    pub word_count: u32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub audit_issues: Vec<String>,
    #[serde(default)]
    pub length_warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
}
