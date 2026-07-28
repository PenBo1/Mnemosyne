//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Agents - 短篇创作代理模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：短篇 pipeline 的 6 个 agent（create_outline / review_outline / revise_outline /
//! write_draft / review_draft / revise_draft / generate_package）+ 输出解析。
//!
//! 约束：AgentEngine.prompt_once 只支持单轮对话。多轮对话（revise 系列）
//! 合并为单轮：在 user message 中嵌入原 v1 输出 + review + 修订指令。
//!
//! 模块拆分：
//! - types: 常量与数据/输入结构
//! - agents: 7 个 agent 执行函数（调用 prompt_once + 解析）
//! - helpers: 渲染、校验与计量辅助函数
//! - prompts_outline: 大纲阶段 prompt 构造
//! - prompts_draft: 草稿阶段 prompt 构造
//! - prompts_package: 销售包装阶段 prompt 构造
//! - parsing: 输出解析（标签区块提取 / Markdown 回退 / 标题归一化）

// ── 模块声明 ────────────────────────────────────────────────────────────────

mod agents;
mod helpers;
mod parsing;
mod prompts_draft;
mod prompts_outline;
mod prompts_package;
mod types;

// ── 公共 API 重导出（与原 flat 文件保持一致） ─────────────────

pub use agents::{continue_draft, create_outline, generate_package, review_draft,
    review_outline, revise_draft, revise_outline, write_draft};
pub use helpers::{count_chapter_length, estimate_short_fiction_max_tokens,
    find_empty_chapters, format_chapter_heading, render_draft_markdown,
    validate_draft_for_final};
pub use parsing::{extract_tagged_block, parse_batch_draft, parse_outline, parse_sales_package};
pub use types::{ShortFictionBatchDraft, ShortFictionChapter, ShortFictionDraftContinuationInput,
    ShortFictionDraftInput, ShortFictionDraftReviewInput, ShortFictionDraftRevisionInput,
    ShortFictionOutline, ShortFictionOutlineInput, ShortFictionOutlineReviewInput,
    ShortFictionOutlineRevisionInput, ShortFictionPackageInput, ShortFictionReference,
    ShortFictionSalesPackage, SHORT_FICTION_DEFAULT_CHARS_PER_CHAPTER,
    SHORT_FICTION_DEFAULT_CHAPTERS, SHORT_FICTION_DRAFT_COMPLETION_ATTEMPTS,
    SHORT_FICTION_EN_DEFAULT_WORDS_PER_CHAPTER, SHORT_FICTION_MAX_CHAPTERS,
    SHORT_FICTION_MIN_CHAPTERS};
