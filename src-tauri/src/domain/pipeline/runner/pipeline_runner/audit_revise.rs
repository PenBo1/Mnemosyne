//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner Audit Revise - 审计与修订阶段
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：audit_draft / revise_draft

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::agents::{continuity, length_normalizer, reviser};
use crate::domain::pipeline::governance::length::{
    build_length_spec, count_chapter_length, is_outside_hard_range,
};
use crate::domain::pipeline::state::manager::StateManager;
use crate::domain::pipeline::types::{ChapterStatus, Language};
use crate::shared::error::AppError;

use super::{PipelineRunner, ReviseResult};
use super::current_iso;

impl PipelineRunner {
    // ── 7. audit_draft ──

    /// 审计指定章节（或最新章）。
    pub async fn audit_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        chapter_number: Option<u32>,
    ) -> Result<continuity::AuditResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "audit_draft", book_id, chapter_number, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();

            let target = match chapter_number {
                Some(n) => n,
                None => self.get_next_chapter_number(&book_dir)?.saturating_sub(1),
            };
            if target < 1 {
                return Err(AppError::invalid_input("No chapters to audit"));
            }

            self.log_stage(language, &format!("审计第{}章", target));
            let content = self.read_chapter_content(&book_dir, target)?
                .ok_or_else(|| AppError::file_not_found(format!("chapter {}", target)))?;

            let ctrl = StateManager::load_control_documents(&book_dir, language)?;
            let story_dir = book_dir.join("story");
            let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();
            let read_outline = |name: &str| {
                let p = story_dir.join("outline").join(name);
                if p.exists() { std::fs::read_to_string(&p).unwrap_or_default() }
                else { std::fs::read_to_string(story_dir.join(name)).unwrap_or_default() }
            };

            let auditor_ctx = continuity::AuditorContext {
                current_state: read_safe("current_state.md"),
                pending_hooks: read_safe("pending_hooks.md"),
                chapter_summaries: read_safe("chapter_summaries.md"),
                volume_map: read_outline("volume_map.md"),
                story_frame: read_outline("story_frame.md"),
                book_rules: read_safe("book_rules.md"),
                style_guide: ctrl.style_guide,
                chapter_memo: String::new(),
                previous_chapter: if target > 1 {
                    self.read_chapter_content(&book_dir, target - 1)?.unwrap_or_default()
                } else {
                    String::new()
                },
                parent_canon: read_safe("parent_canon.md"),
                fanfic_canon: read_safe("fanfic_canon.md"),
            };

            let result = continuity::audit_chapter(engine, &book, target, "", &content, &auditor_ctx).await?;

            let mut index = self.load_chapter_index(&book_dir)?;
            if let Some(entry) = index.iter_mut().find(|c| c.number == target) {
                entry.status = if result.passed { ChapterStatus::AuditPassed } else { ChapterStatus::AuditFailed };
                entry.updated_at = current_iso();
                entry.audit_issues = result.issues.iter().map(|i| format!("[{:?}] {}", i.severity, i.description)).collect();
            }
            self.save_chapter_index(&book_dir, &index)?;

            Ok(result)
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "audit_draft", book_id, duration_ms, passed = r.passed, issue_count = r.issues.len(), "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "audit_draft", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }

    // ── 8. revise_draft ──

    /// 修订指定章节（或最新章）。
    pub async fn revise_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        chapter_number: Option<u32>,
        mode: reviser::ReviseMode,
    ) -> Result<ReviseResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "revise_draft", book_id, chapter_number, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();

            let target = match chapter_number {
                Some(n) => n,
                None => self.get_next_chapter_number(&book_dir)?.saturating_sub(1),
            };
            if target < 1 {
                return Err(AppError::invalid_input("No chapters to revise"));
            }

            self.log_stage(language, &format!("加载第{}章修订上下文", target));
            let content = self.read_chapter_content(&book_dir, target)?
                .ok_or_else(|| AppError::file_not_found(format!("chapter {}", target)))?;

            let ctrl = StateManager::load_control_documents(&book_dir, language)?;
            let story_dir = book_dir.join("story");
            let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();
            let read_outline = |name: &str| {
                let p = story_dir.join("outline").join(name);
                if p.exists() { std::fs::read_to_string(&p).unwrap_or_default() }
                else { std::fs::read_to_string(story_dir.join(name)).unwrap_or_default() }
            };

            let auditor_ctx = continuity::AuditorContext {
                current_state: read_safe("current_state.md"),
                pending_hooks: read_safe("pending_hooks.md"),
                chapter_summaries: read_safe("chapter_summaries.md"),
                volume_map: read_outline("volume_map.md"),
                story_frame: read_outline("story_frame.md"),
                book_rules: read_safe("book_rules.md"),
                style_guide: ctrl.style_guide.clone(),
                chapter_memo: String::new(),
                previous_chapter: String::new(),
                parent_canon: read_safe("parent_canon.md"),
                fanfic_canon: read_safe("fanfic_canon.md"),
            };

            let pre_audit = continuity::audit_chapter(engine, &book, target, "", &content, &auditor_ctx).await?;

            if pre_audit.issues.is_empty() {
                return Ok(ReviseResult {
                    chapter_number: target,
                    word_count: count_chapter_length(&content, build_length_spec(book.chapter_word_count, language).counting_mode),
                    fixed_issues: vec![],
                    applied: false,
                    status: "unchanged".to_string(),
                    skipped_reason: Some("No issues to fix.".to_string()),
                });
            }

            let reviser_ctx = reviser::ReviserContext {
                current_state: read_safe("current_state.md"),
                pending_hooks: read_safe("pending_hooks.md"),
                chapter_summaries: read_safe("chapter_summaries.md"),
                volume_map: read_outline("volume_map.md"),
                story_frame: read_outline("story_frame.md"),
                book_rules: read_safe("book_rules.md"),
                style_guide: ctrl.style_guide,
                chapter_memo: String::new(),
            };

            self.log_stage(language, &format!("修订第{}章", target));
            let revise_output = reviser::revise_chapter(engine, &book, target, &content, &pre_audit.issues, mode, &reviser_ctx).await?;

            if revise_output.revised_content.is_empty() {
                return Err(AppError::internal("Reviser returned empty content"));
            }

            let length_spec = build_length_spec(book.chapter_word_count, language);
            let normalized = if is_outside_hard_range(revise_output.word_count, &length_spec) {
                let n = length_normalizer::normalize_chapter(engine, &revise_output.revised_content, length_spec.target, length_spec.soft_min, length_spec.soft_max).await?;
                (n.normalized_content, n.final_count)
            } else {
                (revise_output.revised_content.clone(), revise_output.word_count)
            };

            let post_audit = continuity::audit_chapter(engine, &book, target, "", &normalized.0, &auditor_ctx).await?;

            let should_apply = post_audit.issues.len() <= pre_audit.issues.len();
            if should_apply {
                let chapters_dir = book_dir.join("chapters");
                let padded = format!("{:04}", target);
                let entries = std::fs::read_dir(&chapters_dir)?;
                let existing = entries.into_iter()
                    .filter_map(|e| e.ok())
                    .find(|e| e.file_name().to_string_lossy().starts_with(&padded) && e.file_name().to_string_lossy().ends_with(".md"))
                    .ok_or_else(|| AppError::file_not_found(format!("chapter {} file", target)))?;

                let heading = match language {
                    Language::Zh => format!("# 第{}章 {}\n\n{}", target, self.get_chapter_title(&book_dir, target), normalized.0),
                    Language::En => format!("# Chapter {}: {}\n\n{}", target, self.get_chapter_title(&book_dir, target), normalized.0),
                };
                std::fs::write(existing.path(), &heading)?;

                if !revise_output.updated_state.is_empty() {
                    std::fs::write(story_dir.join("current_state.md"), &revise_output.updated_state)?;
                }
                if !revise_output.updated_hooks.is_empty() {
                    std::fs::write(story_dir.join("pending_hooks.md"), &revise_output.updated_hooks)?;
                }

                let mut index = self.load_chapter_index(&book_dir)?;
                if let Some(entry) = index.iter_mut().find(|c| c.number == target) {
                    entry.status = if post_audit.passed { ChapterStatus::ReadyForReview } else { ChapterStatus::AuditFailed };
                    entry.word_count = normalized.1;
                    entry.updated_at = current_iso();
                    entry.audit_issues = post_audit.issues.iter().map(|i| format!("[{:?}] {}", i.severity, i.description)).collect();
                }
                self.save_chapter_index(&book_dir, &index)?;

                self.snapshot_state(&book_dir, target)?;

                Ok(ReviseResult {
                    chapter_number: target,
                    word_count: normalized.1,
                    fixed_issues: revise_output.fixed_issues,
                    applied: true,
                    status: if post_audit.passed { "ready-for-review".to_string() } else { "audit-failed".to_string() },
                    skipped_reason: None,
                })
            } else {
                Ok(ReviseResult {
                    chapter_number: target,
                    word_count: count_chapter_length(&content, length_spec.counting_mode),
                    fixed_issues: vec![],
                    applied: false,
                    status: "unchanged".to_string(),
                    skipped_reason: Some("Revision did not improve audit".to_string()),
                })
            }
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "revise_draft", book_id, duration_ms, chapter_number = r.chapter_number, applied = r.applied, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "revise_draft", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }
}
