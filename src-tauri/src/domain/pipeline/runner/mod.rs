//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner - Pipeline 编排层
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 子模块：
//! - chapter_review_cycle: Audit↔Revise 评分循环（带最佳快照回退）
//! - chapter_state_recovery: 状态校验失败恢复（重试结算 + 降级问题构建 + review note 解析）
//! - chapter_truth_validation: 真相文件持久化校验（含状态恢复与降级标记）
//! - ai_tells: AI 味结构检测（纯规则）
//! - sensitive_words: 敏感词检测（基础词表 + 字面匹配）
//! - post_write_checks: 写后确定性校验（normalize + assert_not_empty + 规则检查）

// ── 模块声明 ────────────────────────────────────────────────────────────────

pub mod chapter_review_cycle;
pub mod chapter_state_recovery;
pub mod chapter_truth_validation;
pub mod pipeline_runner;
pub mod short_fiction_runner;
pub mod script_storyboard_runner;
pub mod ai_tells;
pub mod sensitive_words;
pub mod post_write_checks;

pub use pipeline_runner::{
    ChapterPipelineResult, ComposeChapterResult, PipelineConfig, PipelineRunner,
    PlanChapterResult, ReviseResult,
};
