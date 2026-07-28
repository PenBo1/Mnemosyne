//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Types - 数据结构与常量
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：短篇 pipeline 的常量、输出结构（ShortFictionOutline / ShortFictionChapter /
//! ShortFictionBatchDraft / ShortFictionSalesPackage / ShortFictionReference）与输入结构。

use crate::domain::pipeline::types::Language;

// ── 常量 ────────────────────────────────────────────────────────────────────

pub const SHORT_FICTION_DEFAULT_CHAPTERS: u32 = 12;
pub const SHORT_FICTION_MIN_CHAPTERS: u32 = 12;
pub const SHORT_FICTION_MAX_CHAPTERS: u32 = 18;
pub const SHORT_FICTION_DEFAULT_CHARS_PER_CHAPTER: u32 = 1000;
pub const SHORT_FICTION_EN_DEFAULT_WORDS_PER_CHAPTER: u32 = 650;

/// 漏章补写最大重试次数。
pub const SHORT_FICTION_DRAFT_COMPLETION_ATTEMPTS: u32 = 3;

// ── 数据结构 ─────────────────────────────────────────────────

/// 短篇大纲
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionOutline {
    pub story_title: String,
    pub raw_content: String,
}

/// 短篇章节
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionChapter {
    pub number: u32,
    pub title: String,
    pub content: String,
    pub char_count: u32,
}

/// 短篇批量草稿
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionBatchDraft {
    pub story_title: String,
    pub opening_hook: Option<String>,
    pub chapters: Vec<ShortFictionChapter>,
    pub raw_content: String,
}

/// 短篇销售包装
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionSalesPackage {
    pub title: String,
    pub intro: String,
    pub selling_points: Vec<String>,
    pub cover_prompt: String,
    pub raw_content: String,
}

/// 参考文本
#[derive(Debug, Clone)]
pub struct ShortFictionReference {
    pub text: String,
}

// ── 输入结构 ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShortFictionOutlineInput {
    pub direction: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub reference: Option<ShortFictionReference>,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionOutlineReviewInput {
    pub direction: String,
    pub outline: ShortFictionOutline,
    pub reference: Option<ShortFictionReference>,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionOutlineRevisionInput {
    pub direction: String,
    pub outline: ShortFictionOutline,
    pub review: String,
    pub reference: Option<ShortFictionReference>,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftContinuationInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub draft: ShortFictionBatchDraft,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftReviewInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub draft: ShortFictionBatchDraft,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftRevisionInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub draft: ShortFictionBatchDraft,
    pub review: String,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionPackageInput {
    pub direction: String,
    pub outline_markdown: String,
    pub draft: ShortFictionBatchDraft,
    pub language: Language,
}
