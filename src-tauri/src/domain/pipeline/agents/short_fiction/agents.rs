//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Agents - 短篇创作执行函数
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：短篇 pipeline 的 7 个 agent 执行入口（create_outline / review_outline /
//! revise_outline / write_draft / continue_draft / review_draft / revise_draft /
//! generate_package）。每个函数构造 prompt、调用 AgentEngine.prompt_once、解析输出。
//!
//! 约束：AgentEngine.prompt_once 只支持单轮对话。多轮对话（revise 系列）
//! 合并为单轮：在 user message 中嵌入原 v1 输出 + review + 修订指令。

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::helpers::{find_empty_chapters, render_draft_markdown};
use super::parsing::{parse_batch_draft, parse_outline, parse_sales_package};
use super::prompts_draft::{
    build_draft_continuation_user_prompt, build_draft_review_system_prompt,
    build_draft_review_user_prompt, build_draft_revision_followup, build_writer_system_prompt,
    build_writer_user_prompt, v1_draft_header,
};
use super::prompts_outline::{
    build_outline_review_system_prompt, build_outline_review_user_prompt,
    build_outline_revision_followup, build_outline_system_prompt, build_outline_user_prompt,
    v1_outline_header,
};
use super::prompts_package::{build_package_system_prompt, build_package_user_prompt};
use super::types::{
    ShortFictionBatchDraft, ShortFictionDraftContinuationInput, ShortFictionDraftInput,
    ShortFictionDraftReviewInput, ShortFictionDraftRevisionInput, ShortFictionOutline,
    ShortFictionOutlineInput, ShortFictionOutlineReviewInput, ShortFictionOutlineRevisionInput,
    ShortFictionPackageInput, ShortFictionSalesPackage,
};

// ═══════════════════════════════════════════════════════════════
//  Agent 执行函数
// ═══════════════════════════════════════════════════════════════

/// 1. 生成短篇大纲（temp=0.55, maxTokens=8192）。
pub async fn create_outline(
    engine: &AgentEngine,
    input: &ShortFictionOutlineInput,
) -> Result<ShortFictionOutline, AppError> {
    let start = Instant::now();
    tracing::info!(function = "create_outline", chapter_count = input.chapter_count, chars_per_chapter = input.chars_per_chapter, language = ?input.language, "入口");

    let system_prompt = build_outline_system_prompt(input.language);
    let user_message = build_outline_user_prompt(input);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let outline = parse_outline(&response, input.language);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "create_outline", duration_ms, title = %outline.story_title, "出口");
            Ok(outline)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "create_outline", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 2. 审核短篇大纲（temp=0.3, maxTokens=4096）。
pub async fn review_outline(
    engine: &AgentEngine,
    input: &ShortFictionOutlineReviewInput,
) -> Result<String, AppError> {
    let start = Instant::now();
    tracing::info!(function = "review_outline", title = %input.outline.story_title, "入口");

    let system_prompt = build_outline_review_system_prompt(input.language);
    let user_message = build_outline_review_user_prompt(input);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "review_outline", duration_ms, "出口");
            Ok(response.trim().to_string())
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "review_outline", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 3. 修订短篇大纲（temp=0.45, maxTokens=8192）。
///    多轮对话合并为单轮：user message 包含原 prompt + v1 outline + 修订指令。
pub async fn revise_outline(
    engine: &AgentEngine,
    input: &ShortFictionOutlineRevisionInput,
) -> Result<ShortFictionOutline, AppError> {
    let start = Instant::now();
    tracing::info!(function = "revise_outline", title = %input.outline.story_title, chapter_count = input.chapter_count, "入口");

    let system_prompt = build_outline_system_prompt(input.language);
    let original_user = build_outline_user_prompt(&ShortFictionOutlineInput {
        direction: input.direction.clone(),
        chapter_count: input.chapter_count,
        chars_per_chapter: input.chars_per_chapter,
        reference: input.reference.clone(),
        language: input.language,
    });
    let followup = build_outline_revision_followup(input);
    let user_message = format!(
        "{original}\n\n---\n\n{v1_header}\n\n{v1}\n\n---\n\n{followup}",
        original = original_user,
        v1_header = v1_outline_header(input.language),
        v1 = input.outline.raw_content.trim(),
        followup = followup,
    );
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let outline = parse_outline(&response, input.language);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "revise_outline", duration_ms, title = %outline.story_title, "出口");
            Ok(outline)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "revise_outline", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 4. 一次性写出整篇短篇草稿（temp=0.58）。
pub async fn write_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftInput,
) -> Result<ShortFictionBatchDraft, AppError> {
    let start = Instant::now();
    tracing::info!(function = "write_draft", chapter_count = input.chapter_count, chars_per_chapter = input.chars_per_chapter, "入口");

    let system_prompt = build_writer_system_prompt(input.language);
    let user_message = build_writer_user_prompt(input);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let draft = parse_batch_draft(&response, input.chapter_count, input.language);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "write_draft", duration_ms, title = %draft.story_title, chapter_count = draft.chapters.len(), "出口");
            Ok(draft)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "write_draft", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 4b. 补写缺失章节（temp=0.68）。
pub async fn continue_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftContinuationInput,
) -> Result<ShortFictionBatchDraft, AppError> {
    let start = Instant::now();
    let missing = find_empty_chapters(&input.draft);
    
    tracing::info!(function = "continue_draft", title = %input.draft.story_title, missing_count = missing.len(), "入口");

    if missing.is_empty() {
        tracing::info!(function = "continue_draft", duration_ms = 0u64, reason = "no_missing_chapters", "出口");
        return Ok(input.draft.clone());
    }
    let system_prompt = build_writer_system_prompt(input.language);
    let user_message = build_draft_continuation_user_prompt(input, &missing);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let combined = format!(
                "{}\n\n{}",
                input.draft.raw_content.trim(),
                response.trim()
            );
            let draft = parse_batch_draft(&combined, input.chapter_count, input.language);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "continue_draft", duration_ms, filled_chapters = missing.len(), "出口");
            Ok(draft)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "continue_draft", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 5. 审核整篇草稿（temp=0.3, maxTokens=8192）。
pub async fn review_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftReviewInput,
) -> Result<String, AppError> {
    let start = Instant::now();
    tracing::info!(function = "review_draft", title = %input.draft.story_title, chapter_count = input.chapter_count, "入口");

    let system_prompt = build_draft_review_system_prompt(input.language);
    let user_message = build_draft_review_user_prompt(input);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "review_draft", duration_ms, "出口");
            Ok(response.trim().to_string())
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "review_draft", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 6. 修订整篇草稿（temp=0.45）。
///    多轮对话合并为单轮。
pub async fn revise_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftRevisionInput,
) -> Result<ShortFictionBatchDraft, AppError> {
    let start = Instant::now();
    tracing::info!(function = "revise_draft", title = %input.draft.story_title, chapter_count = input.chapter_count, "入口");

    let system_prompt = build_writer_system_prompt(input.language);
    let original_user = build_writer_user_prompt(&ShortFictionDraftInput {
        direction: input.direction.clone(),
        outline_markdown: input.outline_markdown.clone(),
        chapter_count: input.chapter_count,
        chars_per_chapter: input.chars_per_chapter,
        language: input.language,
    });
    let followup = build_draft_revision_followup(input);
    let v1_raw = if !input.draft.raw_content.trim().is_empty() {
        input.draft.raw_content.trim().to_string()
    } else {
        render_draft_markdown(&input.draft, input.language)
    };
    let user_message = format!(
        "{original}\n\n---\n\n{v1_header}\n\n{v1}\n\n---\n\n{followup}",
        original = original_user,
        v1_header = v1_draft_header(input.language),
        v1 = v1_raw,
        followup = followup,
    );
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let draft = parse_batch_draft(&response, input.chapter_count, input.language);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "revise_draft", duration_ms, title = %draft.story_title, "出口");
            Ok(draft)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "revise_draft", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 7. 生成销售包装（temp=0.45, maxTokens=4096）。
pub async fn generate_package(
    engine: &AgentEngine,
    input: &ShortFictionPackageInput,
) -> Result<ShortFictionSalesPackage, AppError> {
    let start = Instant::now();
    tracing::info!(function = "generate_package", title = %input.draft.story_title, "入口");

    let system_prompt = build_package_system_prompt(input.language);
    let user_message = build_package_user_prompt(input);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let pkg = parse_sales_package(&response, &input.draft.story_title);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "generate_package", duration_ms, title = %pkg.title, "出口");
            Ok(pkg)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "generate_package", duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}
