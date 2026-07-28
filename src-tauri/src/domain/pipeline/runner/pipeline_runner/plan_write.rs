//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner Plan Write - 单章前置阶段
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：plan_chapter / compose_chapter / write_draft

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::agents::{planner, writer};
use crate::domain::pipeline::governance::input::{compile_context_package, compile_rule_stack};
use crate::domain::pipeline::governance::length::build_length_spec;
use crate::domain::pipeline::state::manager::StateManager;
use crate::domain::pipeline::types::{ChapterMeta, ChapterStatus, Language};
use crate::shared::error::AppError;

use super::{ComposeChapterResult, DraftResult, PipelineRunner, PlanChapterResult};
use super::{current_iso, sanitize_filename};

impl PipelineRunner {
    // ── 4. plan_chapter ──

    /// 为下一章生成 chapter memo。
    pub async fn plan_chapter(
        &self,
        engine: &AgentEngine,
        book_id: &str,
    ) -> Result<PlanChapterResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "plan_chapter", book_id, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();
            StateManager::ensure_control_documents(&book_dir, language, None)?;

            let chapter_number = self.get_next_chapter_number(&book_dir)?;
            let story_dir = book_dir.join("story");

            let ctrl = StateManager::load_control_documents(&book_dir, language)?;
            let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();
            let read_outline = |name: &str| {
                let p = story_dir.join("outline").join(name);
                if p.exists() { std::fs::read_to_string(&p).unwrap_or_default() }
                else { std::fs::read_to_string(story_dir.join(name)).unwrap_or_default() }
            };

            let ctx = planner::PlannerContext {
                author_intent: ctrl.author_intent,
                current_focus: ctrl.current_focus,
                story_frame: read_outline("story_frame.md"),
                volume_map: read_outline("volume_map.md"),
                book_rules: read_safe("book_rules.md"),
                pending_hooks: read_safe("pending_hooks.md"),
                current_state: read_safe("current_state.md"),
                recent_summaries: self.trim_recent_summaries(&read_safe("chapter_summaries.md"), 10),
                external_context: self.config.external_context.clone(),
            };

            self.log_stage(language, &format!("为第{}章生成 memo", chapter_number));
            let output = planner::plan_chapter(engine, &book, chapter_number, &ctx).await?;

            let runtime_dir = story_dir.join("runtime");
            std::fs::create_dir_all(&runtime_dir)?;
            let intent_path = runtime_dir.join(format!("ch{:04}_intent.md", chapter_number));
            std::fs::write(&intent_path, &output.memo_markdown)?;

            Ok(PlanChapterResult {
                chapter_number,
                intent_path,
                memo_markdown: output.memo_markdown,
            })
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "plan_chapter", book_id, duration_ms, chapter_number = r.chapter_number, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "plan_chapter", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }

    // ── 5. compose_chapter ──

    /// 组装章节上下文（context package + rule stack）。
    pub async fn compose_chapter(
        &self,
        _engine: &AgentEngine,
        book_id: &str,
    ) -> Result<ComposeChapterResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "compose_chapter", book_id, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();
            let chapter_number = self.get_next_chapter_number(&book_dir)?;

            self.log_stage(language, &format!("组装第{}章上下文", chapter_number));
            let context_package = compile_context_package(&book_dir, chapter_number, language)?;
            let rule_stack = compile_rule_stack(&book_dir, language)?;

            let runtime_dir = book_dir.join("story").join("runtime");
            std::fs::create_dir_all(&runtime_dir)?;
            let context_path = runtime_dir.join(format!("ch{:04}_context.md", chapter_number));
            let rule_stack_path = runtime_dir.join(format!("ch{:04}_rules.md", chapter_number));
            std::fs::write(&context_path, &context_package.markdown)?;
            std::fs::write(&rule_stack_path, &rule_stack.markdown)?;

            Ok(ComposeChapterResult {
                chapter_number,
                context_path,
                rule_stack_path,
                context_markdown: context_package.markdown,
                rule_stack_markdown: rule_stack.markdown,
            })
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "compose_chapter", book_id, duration_ms, chapter_number = r.chapter_number, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "compose_chapter", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }

    // ── 6. write_draft ──

    /// 写一章草稿（不含 audit/revise）。
    pub async fn write_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<DraftResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "write_draft", book_id, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();
            StateManager::ensure_control_documents(&book_dir, language, None)?;

            let chapter_number = self.get_next_chapter_number(&book_dir)?;
            let story_dir = book_dir.join("story");

            let ctrl = StateManager::load_control_documents(&book_dir, language)?;
            let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();
            let read_outline = |name: &str| {
                let p = story_dir.join("outline").join(name);
                if p.exists() { std::fs::read_to_string(&p).unwrap_or_default() }
                else { std::fs::read_to_string(story_dir.join(name)).unwrap_or_default() }
            };

            let intent_path = story_dir.join("runtime").join(format!("ch{:04}_intent.md", chapter_number));
            let chapter_memo = std::fs::read_to_string(&intent_path).unwrap_or_default();

            let ctx = writer::WriterContext {
                story_frame: read_outline("story_frame.md"),
                volume_map: read_outline("volume_map.md"),
                current_state: read_safe("current_state.md"),
                pending_hooks: read_safe("pending_hooks.md"),
                chapter_summaries: read_safe("chapter_summaries.md"),
                recent_chapters: self.read_recent_chapters(&book_dir, chapter_number, 1),
                chapter_memo,
                book_rules: read_safe("book_rules.md"),
                style_guide: ctrl.style_guide,
                external_context: self.config.external_context.clone(),
            };

            let _length_spec = build_length_spec(
                word_count_override.unwrap_or(book.chapter_word_count),
                language,
            );

            self.log_stage(language, &format!("撰写第{}章草稿", chapter_number));
            let output = writer::write_chapter(engine, &book, chapter_number, &ctx).await?;

            let chapters_dir = book_dir.join("chapters");
            std::fs::create_dir_all(&chapters_dir)?;
            let padded = format!("{:04}", chapter_number);
            let chapter_filename = format!("{}-{}.md", padded, sanitize_filename(&output.title));
            let chapter_path = chapters_dir.join(&chapter_filename);
            let heading = match language {
                Language::Zh => format!("# 第{}章 {}\n\n{}", chapter_number, output.title, output.content),
                Language::En => format!("# Chapter {}: {}\n\n{}", chapter_number, output.title, output.content),
            };
            std::fs::write(&chapter_path, &heading)?;

            self.persist_truth_files(&story_dir, &output)?;

            let now = current_iso();
            let entry = ChapterMeta {
                number: chapter_number,
                title: output.title.clone(),
                status: ChapterStatus::Drafted,
                word_count: output.word_count,
                created_at: now.clone(),
                updated_at: now,
                audit_issues: vec![],
                length_warnings: vec![],
                review_note: None,
                detection_score: None,
                detection_provider: None,
                detected_at: None,
                token_usage: None,
            };
            let mut index = self.load_chapter_index(&book_dir)?;
            index.push(entry);
            self.save_chapter_index(&book_dir, &index)?;

            self.snapshot_state(&book_dir, chapter_number)?;

            Ok(DraftResult {
                chapter_number,
                title: output.title,
                word_count: output.word_count,
                file_path: chapter_path,
            })
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "write_draft", book_id, duration_ms, chapter_number = r.chapter_number, word_count = r.word_count, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "write_draft", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }
}
