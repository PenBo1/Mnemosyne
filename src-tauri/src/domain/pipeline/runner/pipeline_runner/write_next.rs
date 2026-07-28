//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner Write Next - 完整 8-agent Cycle
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 流程：plan -> compose -> write -> review cycle -> truth validation -> persist

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::agents::{consolidator, continuity, writer};
use crate::domain::pipeline::governance::input;
use crate::domain::pipeline::governance::length::build_length_spec;
use crate::domain::pipeline::runner::chapter_review_cycle::{self, CycleUsage};
use crate::domain::pipeline::runner::chapter_state_recovery;
use crate::domain::pipeline::runner::chapter_truth_validation::{self, PreviousTruth};
use crate::domain::pipeline::state::manager::StateManager;
use crate::domain::pipeline::types::{
    ChapterMeta, ChapterReviewMode, ChapterStatus, Language, resolve_chapter_review_mode,
};
use crate::domain::pipeline::utils::text_parse::extract_section;
use crate::shared::error::AppError;

use super::{ChapterPipelineResult, PipelineRunner};
use super::{current_iso, sanitize_filename};

impl PipelineRunner {
    // ── 9. write_next_chapter（完整 8-agent cycle）──

    /// 写下一章完整流程：plan → compose → write → review cycle → truth validation → persist。
    pub async fn write_next_chapter(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<ChapterPipelineResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "write_next_chapter", book_id, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();
            StateManager::ensure_control_documents(&book_dir, language, None)?;

            let chapter_number = self.get_next_chapter_number(&book_dir)?;
            let story_dir = book_dir.join("story");

            self.log_stage(language, &format!("规划第{}章", chapter_number));
            let plan_result = self.plan_chapter(engine, book_id).await?;
            let chapter_memo = plan_result.memo_markdown;

            self.log_stage(language, &format!("组装第{}章上下文", chapter_number));
            let _compose_result = self.compose_chapter(engine, book_id).await?;

            let ctrl = StateManager::load_control_documents(&book_dir, language)?;
            let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();
            let read_outline = |name: &str| {
                let p = story_dir.join("outline").join(name);
                if p.exists() { std::fs::read_to_string(&p).unwrap_or_default() }
                else { std::fs::read_to_string(story_dir.join(name)).unwrap_or_default() }
            };

            let writer_ctx = writer::WriterContext {
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

            let length_spec = build_length_spec(
                word_count_override.unwrap_or(book.chapter_word_count),
                language,
            );

            self.log_stage(language, &format!("撰写第{}章草稿", chapter_number));
            let writer_output = writer::write_chapter(engine, &book, chapter_number, &writer_ctx).await?;
            let writer_count = writer_output.word_count;

            let review_mode = resolve_chapter_review_mode(&book, self.config.default_review_mode);

            let (final_content, final_word_count, revised, audit_result) = if review_mode == ChapterReviewMode::Manual {
                self.log_stage(language, "写完即停（手动审查模式）");
                (writer_output.content.clone(), writer_count, false, continuity::AuditResult {
                    passed: false,
                    overall_score: None,
                    issues: vec![],
                    summary: "尚未审查（手动模式：写完即停）".to_string(),
                    parse_failed: false,
                })
            } else {
                self.log_stage(language, "运行审稿循环");
                let governed = input::create_governed_artifacts(&book, &book_dir, chapter_number, self.config.external_context.as_deref()).ok();
                let cycle_result = chapter_review_cycle::run_chapter_review_cycle(
                    engine,
                    &book,
                    &book_dir,
                    chapter_number,
                    &writer_output.content,
                    writer_count,
                    &length_spec,
                    CycleUsage::default(),
                    governed.as_ref(),
                    Some(self.config.writing_review_retries as usize),
                ).await?;
                (cycle_result.final_content, cycle_result.final_word_count, cycle_result.revised, cycle_result.audit_result)
            };

            self.log_stage(language, "校验真相文件");
            let old_state = read_safe("current_state.md");
            let old_hooks = read_safe("pending_hooks.md");
            let old_ledger = read_safe("particle_ledger.md");
            let parsed_state = extract_section(&writer_output.post_settlement, "UPDATED_STATE");
            let parsed_hooks = extract_section(&writer_output.post_settlement, "UPDATED_HOOKS");
            let updated_state: &str = parsed_state.as_deref().unwrap_or("");
            let updated_hooks: &str = parsed_hooks.as_deref().unwrap_or("");

            let truth_validation = chapter_truth_validation::validate_chapter_truth_persistence(
                engine,
                &book,
                &book_dir,
                chapter_number,
                &writer_output.title,
                &final_content,
                updated_state,
                updated_hooks,
                &PreviousTruth {
                    old_state: old_state.clone(),
                    old_hooks: old_hooks.clone(),
                    old_ledger: old_ledger.clone(),
                },
                language,
            ).await?;

            let chapter_status = truth_validation.chapter_status.clone();

            self.log_stage(language, "落盘最终章节");
            let chapters_dir = book_dir.join("chapters");
            std::fs::create_dir_all(&chapters_dir)?;
            let padded = format!("{:04}", chapter_number);
            let chapter_filename = format!("{}-{}.md", padded, sanitize_filename(&writer_output.title));
            let chapter_path = chapters_dir.join(&chapter_filename);

            let index_path = book_dir.join("chapters.json");
            let index_backup: Option<String> = match std::fs::read_to_string(&index_path) {
                Ok(content) => Some(content),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(AppError::internal(format!("读取 chapters.json 备份失败: {}", e))),
            };

            let persist_result: Result<(), AppError> = async {
                let heading = match language {
                    Language::Zh => format!("# 第{}章 {}\n\n{}", chapter_number, writer_output.title, final_content),
                    Language::En => format!("# Chapter {}: {}\n\n{}", chapter_number, writer_output.title, final_content),
                };
                std::fs::write(&chapter_path, &heading)?;

                if chapter_status.is_none() {
                    match (&truth_validation.recovered_state, &truth_validation.recovered_hooks) {
                        (Some(state), Some(hooks)) => {
                            std::fs::write(story_dir.join("current_state.md"), state)?;
                            std::fs::write(story_dir.join("pending_hooks.md"), hooks)?;
                        }
                        _ => {
                            self.persist_truth_files(&story_dir, &writer_output)?;
                        }
                    }
                }

                let now = current_iso();
                let status = match chapter_status.as_deref() {
                    Some("state-degraded") => ChapterStatus::StateDegraded,
                    None if audit_result.passed => ChapterStatus::ReadyForReview,
                    _ => ChapterStatus::AuditFailed,
                };
                let entry = ChapterMeta {
                    number: chapter_number,
                    title: writer_output.title.clone(),
                    status,
                    word_count: final_word_count,
                    created_at: now.clone(),
                    updated_at: now,
                    audit_issues: audit_result.issues.iter().map(|i| format!("[{:?}] {}", i.severity, i.description)).collect(),
                    length_warnings: vec![],
                    review_note: if chapter_status.is_some() {
                        Some(chapter_state_recovery::build_state_degraded_review_note(
                            if audit_result.passed { "ready-for-review" } else { "audit-failed" },
                            &truth_validation.degraded_issues,
                        ))
                    } else {
                        None
                    },
                    detection_score: None,
                    detection_provider: None,
                    detected_at: None,
                    token_usage: None,
                };
                let mut index = self.load_chapter_index(&book_dir)?;
                index.push(entry);
                self.save_chapter_index(&book_dir, &index)?;

                self.snapshot_state(&book_dir, chapter_number)?;
                Ok(())
            }
            .await;

            if let Err(e) = persist_result {
                let _ = std::fs::remove_file(&chapter_path);
                match &index_backup {
                    Some(content) => {
                        if let Err(restore_err) = std::fs::write(&index_path, content) {
                            tracing::error!(
                                error = %restore_err,
                                "事务回滚：恢复 chapters.json 失败"
                            );
                        }
                    }
                    None => {
                        let _ = std::fs::remove_file(&index_path);
                    }
                }
                tracing::error!(
                    error = %e,
                    chapter = chapter_number,
                    "事务回滚：章节持久化失败，已清理 chapter.md 并恢复 chapters.json"
                );
                return Err(e);
            }

            if chapter_number % 30 == 0 {
                self.log_stage(language, "压缩旧卷摘要");
                if let Err(e) = consolidator::consolidate(engine, &book_dir).await {
                    tracing::warn!(
                        error = %e,
                        chapter = chapter_number,
                        "Consolidation 失败（章节已落盘，不影响主流程）"
                    );
                }
            }

            let pipeline_status = match chapter_status.as_deref() {
                Some("state-degraded") => "state-degraded".to_string(),
                None if audit_result.passed => "ready-for-review".to_string(),
                _ => "audit-failed".to_string(),
            };
            Ok(ChapterPipelineResult {
                chapter_number,
                title: writer_output.title,
                word_count: final_word_count,
                audit_result,
                revised,
                status: pipeline_status,
            })
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "write_next_chapter", book_id, duration_ms, chapter_number = r.chapter_number, word_count = r.word_count, status = %r.status, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "write_next_chapter", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }
}
