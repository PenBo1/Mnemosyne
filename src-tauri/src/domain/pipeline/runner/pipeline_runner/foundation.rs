//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner Foundation - 基础设定阶段
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：init_book / generate_and_review_foundation / revise_foundation

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::agents::{architect, foundation_reviewer};
use crate::domain::pipeline::state::manager::StateManager;
use crate::domain::pipeline::types::BookConfig;
use crate::infrastructure::telemetry::SpanKind;
use crate::shared::error::AppError;

use super::PipelineRunner;

impl PipelineRunner {
    // ── 1. init_book ──

    /// 初始化书籍：生成基础设定 + 写入文件 + 初始化控制文档 + 创建快照。
    /// 使用 staging 目录原子化：全部成功后才 rename 到目标位置。
    pub async fn init_book(
        &self,
        engine: &AgentEngine,
        book: &BookConfig,
        genre_name: &str,
        genre_body: &str,
        external_context: Option<&str>,
        author_intent: Option<&str>,
    ) -> Result<(), AppError> {
        let start = Instant::now();
        tracing::info!(function = "init_book", book_id = %book.id, genre_name, "入口");

        let tracer = engine.tracer();
        let mut span = tracer.start_span("pipeline.init_book", SpanKind::Internal);
        span.set_workspace(&book.id);

        let book_dir = self.book_dir(&book.id);
        let staging_dir = self.book_dir(&format!(".tmp-book-create-{}", book.id));
        let language = book.language.unwrap_or_default();

        let result = async {
            self.log_stage(language, r###"生成基础设定"###);
            let foundation = self.generate_and_review_foundation(engine, book, genre_name, genre_body, external_context).await?;

            std::fs::create_dir_all(&staging_dir)?;
            self.log_stage(language, r###"保存书籍配置"###);
            StateManager::save_book_config(&staging_dir, book)?;

            self.log_stage(language, r###"写入基础设定文件"###);
            architect::persist_output(&staging_dir, &foundation)?;

            if let Some(ctx) = external_context {
                if !ctx.trim().is_empty() {
                    let story_dir = staging_dir.join("story");
                    std::fs::create_dir_all(&story_dir)?;
                    std::fs::write(story_dir.join("brief.md"), ctx)?;
                }
            }

            self.log_stage(language, r###"初始化控制文档"###);
            StateManager::ensure_control_documents(&staging_dir, language, author_intent.or(external_context))?;

            std::fs::create_dir_all(staging_dir.join("chapters"))?;
            self.save_chapter_index(&staging_dir, &[])?;
            self.snapshot_state(&staging_dir, 0)?;

            if book_dir.exists() {
                return Err(AppError::invalid_input(format!("Book {} already exists", book.id)));
            }
            std::fs::rename(&staging_dir, &book_dir)?;

            Ok(())
        }.await;

        match &result {
            Ok(_) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "init_book", book_id = %book.id, duration_ms, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "init_book", book_id = %book.id, duration_ms, error = %e, "错误");
            }
        }
        result
    }

    // ── 2. generate_and_review_foundation ──

    /// 生成基础设定并审核。最多重试 foundation_review_retries 次。
    async fn generate_and_review_foundation(
        &self,
        engine: &AgentEngine,
        book: &BookConfig,
        genre_name: &str,
        genre_body: &str,
        external_context: Option<&str>,
    ) -> Result<architect::ArchitectOutput, AppError> {
        let max_retries = self.config.foundation_review_retries;
        let mut foundation = architect::generate_foundation(engine, book, genre_name, genre_body, external_context).await?;

        for attempt in 0..max_retries {
            self.log_stage(book.language.unwrap_or_default(), &format!("审核基础设定（第{}轮）", attempt + 1));
            let review = foundation_reviewer::review_foundation(engine, &foundation, book.target_chapters).await?;

            tracing::info!("Foundation review: {}/100 {}", review.total_score, if review.passed { "PASSED" } else { "REJECTED" });

            if review.passed {
                return Ok(foundation);
            }

            tracing::warn!("Foundation rejected ({}/100), regenerating...", review.total_score);
            // 重新生成（带 feedback）
            foundation = architect::generate_foundation(engine, book, genre_name, genre_body, external_context).await?;
        }

        // 最终审核
        let final_review = foundation_reviewer::review_foundation(engine, &foundation, book.target_chapters).await?;
        tracing::info!("Foundation final review: {}/100 {}", final_review.total_score, if final_review.passed { "PASSED" } else { "ACCEPTED (max retries)" });

        Ok(foundation)
    }

    // ── 3. revise_foundation ──

    /// 修订已有书籍的基础设定（不动 runtime chapter state）。
    pub async fn revise_foundation(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        _feedback: &str,
        genre_name: &str,
        genre_body: &str,
    ) -> Result<(), AppError> {
        let start = Instant::now();
        tracing::info!(function = "revise_foundation", book_id, "入口");

        let result = async {
            let book_dir = self.book_dir(book_id);
            let story_dir = book_dir.join("story");
            let book = StateManager::load_book_config(&book_dir)?;
            let language = book.language.unwrap_or_default();

            let timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S").to_string();
            let backup_dir = story_dir.join(format!(".backup-{}", timestamp));
            std::fs::create_dir_all(&backup_dir)?;
            let flat_files = ["book_rules.md", "current_state.md", "pending_hooks.md"];
            for name in &flat_files {
                let src = story_dir.join(name);
                if src.exists() {
                    let content = std::fs::read_to_string(&src)?;
                    std::fs::write(backup_dir.join(name), content)?;
                }
            }
            self.copy_dir_recursive(&story_dir.join("outline"), &backup_dir.join("outline"))?;
            self.copy_dir_recursive(&story_dir.join("roles"), &backup_dir.join("roles"))?;

            self.log_stage(language, r###"重新生成基础设定"###);
            let foundation = architect::generate_foundation(engine, &book, genre_name, genre_body, None).await?;

            match foundation_reviewer::review_foundation(engine, &foundation, book.target_chapters).await {
                Ok(review) => {
                    if !review.passed {
                        tracing::warn!("[reviseFoundation] Foundation review did not pass; accepting rewrite. Feedback: {}", review.overall_feedback);
                    }
                }
                Err(e) => {
                    tracing::warn!("[reviseFoundation] Foundation review failed and was skipped: {}", e);
                }
            }

            std::fs::create_dir_all(story_dir.join("outline"))?;
            std::fs::create_dir_all(story_dir.join("roles").join("主要角色"))?;
            std::fs::create_dir_all(story_dir.join("roles").join("次要角色"))?;
            architect::persist_output(&book_dir, &foundation)?;

            Ok(())
        }.await;

        match &result {
            Ok(_) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "revise_foundation", book_id, duration_ms, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "revise_foundation", book_id, duration_ms, error = %e, "错误");
            }
        }
        result
    }
}
