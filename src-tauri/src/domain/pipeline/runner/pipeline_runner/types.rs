//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner Types - 类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：PipelineConfig + 各阶段结果类型定义

use std::path::PathBuf;

use crate::domain::pipeline::agents::continuity;
use crate::domain::pipeline::types::{ChapterReviewMode, RevisionGate};

// ── PipelineConfig ──────────────────────────────────────────────────────────

/// Pipeline 配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub books_dir: PathBuf,
    pub model: String,
    pub default_review_mode: Option<ChapterReviewMode>,
    pub default_revision_gate: Option<RevisionGate>,
    pub foundation_review_retries: u32,
    pub writing_review_retries: u32,
    pub external_context: Option<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            books_dir: PathBuf::new(),
            model: String::new(),
            default_review_mode: None,
            default_revision_gate: None,
            foundation_review_retries: 2,
            writing_review_retries: 1,
            external_context: None,
        }
    }
}

// ── 结果类型 ─────────────────────────────────────────────────

/// plan_chapter 结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanChapterResult {
    pub chapter_number: u32,
    pub intent_path: PathBuf,
    pub memo_markdown: String,
}

/// compose_chapter 结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComposeChapterResult {
    pub chapter_number: u32,
    pub context_path: PathBuf,
    pub rule_stack_path: PathBuf,
    pub context_markdown: String,
    pub rule_stack_markdown: String,
}

/// write_draft 结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct DraftResult {
    pub chapter_number: u32,
    pub title: String,
    pub word_count: u32,
    pub file_path: PathBuf,
}

/// revise_draft 结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviseResult {
    pub chapter_number: u32,
    pub word_count: u32,
    pub fixed_issues: Vec<String>,
    pub applied: bool,
    pub status: String,
    pub skipped_reason: Option<String>,
}

/// write_next_chapter 结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChapterPipelineResult {
    pub chapter_number: u32,
    pub title: String,
    pub word_count: u32,
    pub audit_result: continuity::AuditResult,
    pub revised: bool,
    pub status: String,
}
