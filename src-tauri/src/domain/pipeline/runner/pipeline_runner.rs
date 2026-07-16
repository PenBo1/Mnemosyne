// PipelineRunner —— pipeline 编排器。
//
// 8-agent 编排核心：initBook / planChapter / composeChapter / writeDraft / auditDraft /
// reviseDraft / writeNextChapter / reviseFoundation。
//
// Rust 版拆为 PipelineRunner struct + 关联函数。
// 文件系统操作用 std::fs 同步 API（与 state/agents 模块一致），LLM 调用走 AgentEngine。

use std::path::{Path, PathBuf};
use crate::core::agent::engine::AgentEngine;
use crate::infrastructure::telemetry::SpanKind;
use crate::shared::error::AppError;

use super::super::agents::{architect, planner, writer, continuity, reviser, foundation_reviewer, length_normalizer, consolidator};
use super::super::governance::input::{self, compile_context_package, compile_rule_stack};
use super::super::governance::length::{count_chapter_length, is_outside_hard_range, build_length_spec};
use super::super::state::manager::StateManager;
use super::super::state::store;
use super::super::types::{BookConfig, ChapterMeta, ChapterStatus, Language, ChapterReviewMode, resolve_chapter_review_mode};
use super::super::utils::text_parse::extract_section;
use super::chapter_review_cycle::{self, CycleUsage};
use super::chapter_truth_validation::{self, PreviousTruth};
use super::chapter_state_recovery;

// ── PipelineConfig ───────────────────────────────────────────

/// Pipeline 配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub books_dir: PathBuf,
    pub model: String,
    pub default_review_mode: Option<ChapterReviewMode>,
    pub default_revision_gate: Option<super::super::types::RevisionGate>,
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

// ── PipelineRunner ───────────────────────────────────────────

/// Pipeline 编排器
pub struct PipelineRunner {
    config: PipelineConfig,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    fn book_dir(&self, book_id: &str) -> PathBuf {
        self.config.books_dir.join(book_id)
    }

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
        // Trace span: 记录 pipeline.init_book 阶段的耗时与上下文（RAII Drop 自动写入 DB）
        let tracer = engine.tracer();
        let mut span = tracer.start_span("pipeline.init_book", SpanKind::Internal);
        span.set_workspace(&book.id);

        let book_dir = self.book_dir(&book.id);
        let staging_dir = self.book_dir(&format!(".tmp-book-create-{}", book.id));
        let language = book.language.unwrap_or_default();

        // 1. 生成基础设定（含审核循环）
        self.log_stage(language, r###"生成基础设定"###);
        let foundation = self.generate_and_review_foundation(engine, book, genre_name, genre_body, external_context).await?;

        // 2. 写入 staging 目录（先建目录，save_book_config 不负责建父目录）
        std::fs::create_dir_all(&staging_dir)?;
        self.log_stage(language, r###"保存书籍配置"###);
        StateManager::save_book_config(&staging_dir, book)?;

        self.log_stage(language, r###"写入基础设定文件"###);
        architect::persist_output(&staging_dir, &foundation)?;

        // brief.md（外部上下文）
        if let Some(ctx) = external_context {
            if !ctx.trim().is_empty() {
                let story_dir = staging_dir.join("story");
                std::fs::create_dir_all(&story_dir)?;
                std::fs::write(story_dir.join("brief.md"), ctx)?;
            }
        }

        // 3. 初始化控制文档
        self.log_stage(language, r###"初始化控制文档"###);
        StateManager::ensure_control_documents(&staging_dir, language, author_intent.or(external_context))?;

        // 4. 章节索引 + 初始快照
        std::fs::create_dir_all(staging_dir.join("chapters"))?;
        self.save_chapter_index(&staging_dir, &[])?;
        self.snapshot_state(&staging_dir, 0)?;

        // 5. 原子 rename
        if book_dir.exists() {
            return Err(AppError::invalid_input(format!("Book {} already exists", book.id)));
        }
        std::fs::rename(&staging_dir, &book_dir)?;

        Ok(())
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
        let book_dir = self.book_dir(book_id);
        let story_dir = book_dir.join("story");
        let book = StateManager::load_book_config(&book_dir)?;
        let language = book.language.unwrap_or_default();

        // 备份现有基础设定
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
        // 备份 outline/ 和 roles/
        self.copy_dir_recursive(&story_dir.join("outline"), &backup_dir.join("outline"))?;
        self.copy_dir_recursive(&story_dir.join("roles"), &backup_dir.join("roles"))?;

        // 重新生成基础设定
        self.log_stage(language, r###"重新生成基础设定"###);
        let foundation = architect::generate_foundation(engine, &book, genre_name, genre_body, None).await?;

        // 审核（失败不阻塞）
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

        // 写入新基础设定
        std::fs::create_dir_all(story_dir.join("outline"))?;
        std::fs::create_dir_all(story_dir.join("roles").join("主要角色"))?;
        std::fs::create_dir_all(story_dir.join("roles").join("次要角色"))?;
        architect::persist_output(&book_dir, &foundation)?;

        Ok(())
    }

    // ── 4. plan_chapter ──

    /// 为下一章生成 chapter memo。
    pub async fn plan_chapter(
        &self,
        engine: &AgentEngine,
        book_id: &str,
    ) -> Result<PlanChapterResult, AppError> {
        let book_dir = self.book_dir(book_id);
        let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
        let book = StateManager::load_book_config(&book_dir)?;
        let language = book.language.unwrap_or_default();
        StateManager::ensure_control_documents(&book_dir, language, None)?;

        let chapter_number = self.get_next_chapter_number(&book_dir)?;
        let story_dir = book_dir.join("story");

        // 组装 planner 上下文
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

        // 落盘 intent.md
        let runtime_dir = story_dir.join("runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        let intent_path = runtime_dir.join(format!("ch{:04}_intent.md", chapter_number));
        std::fs::write(&intent_path, &output.memo_markdown)?;

        Ok(PlanChapterResult {
            chapter_number,
            intent_path,
            memo_markdown: output.memo_markdown,
        })
    }

    // ── 5. compose_chapter ──

    /// 组装章节上下文（context package + rule stack）。
    pub async fn compose_chapter(
        &self,
        _engine: &AgentEngine,
        book_id: &str,
    ) -> Result<ComposeChapterResult, AppError> {
        let book_dir = self.book_dir(book_id);
        let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
        let book = StateManager::load_book_config(&book_dir)?;
        let language = book.language.unwrap_or_default();
        let chapter_number = self.get_next_chapter_number(&book_dir)?;

        self.log_stage(language, &format!("组装第{}章上下文", chapter_number));
        let context_package = compile_context_package(&book_dir, chapter_number, language)?;
        let rule_stack = compile_rule_stack(&book_dir, language)?;

        // 落盘 context + rule stack
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
    }

    // ── 6. write_draft ──

    /// 写一章草稿（不含 audit/revise）。
    pub async fn write_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<DraftResult, AppError> {
        let book_dir = self.book_dir(book_id);
        let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
        let book = StateManager::load_book_config(&book_dir)?;
        let language = book.language.unwrap_or_default();
        StateManager::ensure_control_documents(&book_dir, language, None)?;

        let chapter_number = self.get_next_chapter_number(&book_dir)?;
        let story_dir = book_dir.join("story");

        // 组装 writer 上下文
        let ctrl = StateManager::load_control_documents(&book_dir, language)?;
        let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();
        let read_outline = |name: &str| {
            let p = story_dir.join("outline").join(name);
            if p.exists() { std::fs::read_to_string(&p).unwrap_or_default() }
            else { std::fs::read_to_string(story_dir.join(name)).unwrap_or_default() }
        };

        // 读取 chapter memo（来自 plan_chapter）
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

        // 落盘草稿
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

        // 落盘 truth files（settler 输出）
        self.persist_truth_files(&story_dir, &output)?;

        // 更新章节索引
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

        // 快照
        self.snapshot_state(&book_dir, chapter_number)?;

        Ok(DraftResult {
            chapter_number,
            title: output.title,
            word_count: output.word_count,
            file_path: chapter_path,
        })
    }

    // ── 7. audit_draft ──

    /// 审计指定章节（或最新章）。
    pub async fn audit_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        chapter_number: Option<u32>,
    ) -> Result<continuity::AuditResult, AppError> {
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

        // 更新索引
        let mut index = self.load_chapter_index(&book_dir)?;
        if let Some(entry) = index.iter_mut().find(|c| c.number == target) {
            entry.status = if result.passed { ChapterStatus::AuditPassed } else { ChapterStatus::AuditFailed };
            entry.updated_at = current_iso();
            entry.audit_issues = result.issues.iter().map(|i| format!("[{:?}] {}", i.severity, i.description)).collect();
        }
        self.save_chapter_index(&book_dir, &index)?;

        Ok(result)
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

        // 1. 重新审计获取结构化问题
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

        // 2. 修订
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

        // 3. 字数归一化
        let length_spec = build_length_spec(book.chapter_word_count, language);
        let normalized = if is_outside_hard_range(revise_output.word_count, &length_spec) {
            let n = length_normalizer::normalize_chapter(engine, &revise_output.revised_content, length_spec.target, length_spec.soft_min, length_spec.soft_max).await?;
            (n.normalized_content, n.final_count)
        } else {
            (revise_output.revised_content.clone(), revise_output.word_count)
        };

        // 4. 重新审计
        let post_audit = continuity::audit_chapter(engine, &book, target, "", &normalized.0, &auditor_ctx).await?;

        // 5. 应用修订（简化版门控：只要不恶化就应用）
        let should_apply = post_audit.issues.len() <= pre_audit.issues.len();
        if should_apply {
            // 落盘修订后的章节
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

            // 更新 truth files
            if !revise_output.updated_state.is_empty() {
                std::fs::write(story_dir.join("current_state.md"), &revise_output.updated_state)?;
            }
            if !revise_output.updated_hooks.is_empty() {
                std::fs::write(story_dir.join("pending_hooks.md"), &revise_output.updated_hooks)?;
            }

            // 更新索引
            let mut index = self.load_chapter_index(&book_dir)?;
            if let Some(entry) = index.iter_mut().find(|c| c.number == target) {
                entry.status = if post_audit.passed { ChapterStatus::ReadyForReview } else { ChapterStatus::AuditFailed };
                entry.word_count = normalized.1;
                entry.updated_at = current_iso();
                entry.audit_issues = post_audit.issues.iter().map(|i| format!("[{:?}] {}", i.severity, i.description)).collect();
            }
            self.save_chapter_index(&book_dir, &index)?;

            // 快照
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
    }

    // ── 9. write_next_chapter（完整 8-agent cycle）──

    /// 写下一章完整流程：plan → compose → write → review cycle → truth validation → persist。
    pub async fn write_next_chapter(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<ChapterPipelineResult, AppError> {
        let book_dir = self.book_dir(book_id);
        let _guard = StateManager::acquire_book_lock(book_id, &book_dir)?;
        let book = StateManager::load_book_config(&book_dir)?;
        let language = book.language.unwrap_or_default();
        StateManager::ensure_control_documents(&book_dir, language, None)?;

        let chapter_number = self.get_next_chapter_number(&book_dir)?;
        let story_dir = book_dir.join("story");

        // ── 1. Plan ──
        self.log_stage(language, &format!("规划第{}章", chapter_number));
        let plan_result = self.plan_chapter(engine, book_id).await?;
        let chapter_memo = plan_result.memo_markdown;

        // ── 2. Compose ──
        self.log_stage(language, &format!("组装第{}章上下文", chapter_number));
        let _compose_result = self.compose_chapter(engine, book_id).await?;

        // ── 3. Write ──
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

        // ── 4. Review Cycle (Audit↔Revise) ──
        let review_mode = resolve_chapter_review_mode(&book, self.config.default_review_mode);

        let (final_content, final_word_count, revised, audit_result) = if review_mode == ChapterReviewMode::Manual {
            // 手动模式：写完即停
            self.log_stage(language, "写完即停（手动审查模式）");
            (writer_output.content.clone(), writer_count, false, continuity::AuditResult {
                passed: false,
                overall_score: None,
                issues: vec![],
                summary: "尚未审查（手动模式：写完即停）".to_string(),
                parse_failed: false,
            })
        } else {
            // 自动模式：运行 review cycle
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

        // ── 5. Truth Validation ──
        self.log_stage(language, "校验真相文件");
        let old_state = read_safe("current_state.md");
        let old_hooks = read_safe("pending_hooks.md");
        let old_ledger = read_safe("particle_ledger.md");
        // 从 post_settlement 中解析出独立的 UPDATED_STATE 与 UPDATED_HOOKS 区块。
        // 若区块缺失则回退到空串（state_validator 会将其标记为矛盾并触发 retry/degraded）。
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

        // ── 6. Persist（事务式：失败时回滚 chapters.json + 清理 chapter.md）──
        self.log_stage(language, "落盘最终章节");
        let chapters_dir = book_dir.join("chapters");
        std::fs::create_dir_all(&chapters_dir)?;
        let padded = format!("{:04}", chapter_number);
        let chapter_filename = format!("{}-{}.md", padded, sanitize_filename(&writer_output.title));
        let chapter_path = chapters_dir.join(&chapter_filename);

        // 备份 chapters.json 以便失败时回滚（不存在则记 None）
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

            // 落盘 truth files（除非 state-degraded）
            if chapter_status.is_none() {
                // 优先使用 retry_settlement 成功后返回的 truth 文件；
                // 否则从 writer_output.post_settlement 解析 UPDATED_STATE/UPDATED_HOOKS。
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

            // 更新章节索引
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

            // 快照
            self.snapshot_state(&book_dir, chapter_number)?;
            Ok(())
        }
        .await;

        if let Err(e) = persist_result {
            // 事务回滚：删除已写的 chapter.md，恢复 chapters.json 备份
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
                    // 原本不存在则删除当前文件（避免残留空/损坏索引）
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

        // ── 7. Consolidation（定期压缩旧卷摘要）──
        if chapter_number % 30 == 0 {
            self.log_stage(language, "压缩旧卷摘要");
            if let Err(e) = consolidator::consolidate(engine, &book_dir).await {
                // 压缩失败不阻塞主流程（章节已落盘），仅记录警告
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
    }

    // ── 辅助方法 ─────────────────────────────────────────────

    fn log_stage(&self, language: Language, message: &str) {
        let prefix = match language { Language::Zh => "阶段：", Language::En => "Stage: " };
        tracing::info!("{}{}", prefix, message);
    }

    fn get_next_chapter_number(&self, book_dir: &Path) -> Result<u32, AppError> {
        let index = self.load_chapter_index(book_dir)?;
        Ok(index.iter().map(|c| c.number).max().unwrap_or(0) + 1)
    }

    fn load_chapter_index(&self, book_dir: &Path) -> Result<Vec<ChapterMeta>, AppError> {
        let path = book_dir.join("chapters.json");
        if !path.exists() { return Ok(vec![]); }
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(|e| AppError::invalid_format(format!("chapters.json parse: {}", e)))
    }

    fn save_chapter_index(&self, book_dir: &Path, index: &[ChapterMeta]) -> Result<(), AppError> {
        let path = book_dir.join("chapters.json");
        let content = serde_json::to_string_pretty(index)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn snapshot_state(&self, book_dir: &Path, chapter_number: u32) -> Result<(), AppError> {
        let mut snapshot = store::load_runtime_state_snapshot(book_dir)?;
        // 更新 manifest.last_applied_chapter
        snapshot.manifest.last_applied_chapter = chapter_number;
        store::save_runtime_state_snapshot(book_dir, &snapshot)?;
        Ok(())
    }

    fn read_chapter_content(&self, book_dir: &Path, chapter_number: u32) -> Result<Option<String>, AppError> {
        let chapters_dir = book_dir.join("chapters");
        let padded = format!("{:04}", chapter_number);
        if !chapters_dir.exists() { return Ok(None); }
        for entry in std::fs::read_dir(&chapters_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&padded) && name.ends_with(".md") {
                let content = std::fs::read_to_string(entry.path())?;
                // 去除首行标题（仅当首行以 "# " 开头时），否则保留全部正文
                let without_heading: String = if content.lines().next().map_or(false, |l| l.starts_with("# ")) {
                    content.lines().skip(1).collect::<Vec<_>>().join("\n")
                } else {
                    content
                };
                return Ok(Some(without_heading.trim().to_string()));
            }
        }
        Ok(None)
    }

    fn read_recent_chapters(&self, book_dir: &Path, current_chapter: u32, count: u32) -> String {
        let mut chapters = Vec::new();
        for i in (1..current_chapter).rev().take(count as usize) {
            if let Ok(Some(content)) = self.read_chapter_content(book_dir, i) {
                chapters.push(format!("## 第{}章\n\n{}", i, content));
            }
        }
        chapters.join("\n\n---\n\n")
    }

    fn get_chapter_title(&self, book_dir: &Path, chapter_number: u32) -> String {
        let index = self.load_chapter_index(book_dir).unwrap_or_default();
        index.iter()
            .find(|c| c.number == chapter_number)
            .map(|c| c.title.clone())
            .unwrap_or_default()
    }

    fn trim_recent_summaries(&self, content: &str, n: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut header_lines: Vec<&str> = Vec::new();
        let mut data_lines: Vec<&str> = Vec::new();
        for line in &lines {
            if line.starts_with('|') {
                if header_lines.is_empty() || line.contains("---") {
                    header_lines.push(line);
                } else {
                    data_lines.push(line);
                }
            }
        }
        if data_lines.is_empty() { return String::new(); }
        let start = data_lines.len().saturating_sub(n);
        let recent = &data_lines[start..];
        let mut result = header_lines.join("\n");
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&recent.join("\n"));
        result
    }

    fn persist_truth_files(&self, story_dir: &Path, output: &writer::WriterOutput) -> Result<(), AppError> {
        // post_settlement 包含 settler 的 UPDATED_STATE + UPDATED_HOOKS 等
        // 简化版：直接写入 post_settlement 到 current_state.md
        if !output.post_settlement.is_empty() {
            // 解析 post_settlement 中的区块
            let state = extract_section(&output.post_settlement, "UPDATED_STATE");
            let hooks = extract_section(&output.post_settlement, "UPDATED_HOOKS");
            if let Some(s) = state { std::fs::write(story_dir.join("current_state.md"), s)?; }
            if let Some(h) = hooks { std::fs::write(story_dir.join("pending_hooks.md"), h)?; }
        }
        Ok(())
    }

    fn copy_dir_recursive(&self, src: &Path, dest: &Path) -> Result<(), AppError> {
        if !src.exists() { return Ok(()); }
        std::fs::create_dir_all(dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                self.copy_dir_recursive(&src_path, &dest_path)?;
            } else if file_type.is_file() {
                if let Ok(content) = std::fs::read_to_string(&src_path) {
                    std::fs::write(&dest_path, content)?;
                }
            }
        }
        Ok(())
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

// ── 工具函数（模块级）──

/// 当前 ISO 时间戳
fn current_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

/// 文件名安全化：将特殊字符替换为下划线
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_replaces_special_chars() {
        // "a/b:c" → 'a','_','b','_','c' = "a_b_c"
        let result = sanitize_filename("a/b:c");
        assert_eq!(result, "a_b_c");
    }

    #[test]
    fn sanitize_filename_replaces_all_special_chars() {
        // 覆盖所有特殊字符
        let result = sanitize_filename(r#"a/b\c:d*e?f"g<h>i|j"#);
        assert_eq!(result, "a_b_c_d_e_f_g_h_i_j");
    }

    #[test]
    fn sanitize_filename_preserves_normal_chars() {
        let result = sanitize_filename("第1章 暗流");
        assert_eq!(result, "第1章 暗流");
    }

    #[test]
    fn extract_section_finds_tag() {
        let content = "前文\n=== UPDATED_STATE ===\n这是状态内容\n=== UPDATED_HOOKS ===\n这是伏笔内容";
        let state = extract_section(content, "UPDATED_STATE").expect("应找到 UPDATED_STATE");
        assert_eq!(state, "这是状态内容");

        let hooks = extract_section(content, "UPDATED_HOOKS").expect("应找到 UPDATED_HOOKS");
        assert_eq!(hooks, "这是伏笔内容");
    }

    #[test]
    fn extract_section_returns_none_when_missing() {
        let content = "无标签内容";
        assert!(extract_section(content, "UPDATED_STATE").is_none());
    }

    #[test]
    fn extract_section_returns_content_until_end() {
        // 最后一个区块：提取到文本结尾
        let content = "=== UPDATED_STATE ===\n最后一行内容\n没有后续标签";
        let state = extract_section(content, "UPDATED_STATE").expect("应找到");
        assert_eq!(state, "最后一行内容\n没有后续标签");
    }

    #[test]
    fn trim_recent_summaries_keeps_last_n() {
        // 构造 15 行数据 + 2 行表头（表头行 + 分隔行）
        let runner = PipelineRunner::new(PipelineConfig::default());
        let mut lines: Vec<String> = Vec::new();
        lines.push("| 章节 | 标题 | 摘要 |".to_string());
        lines.push("| --- | --- | --- |".to_string());
        for i in 1..=15 {
            lines.push(format!("| {} | 标题{} | 摘要{} |", i, i, i));
        }
        let content = lines.join("\n");
        let trimmed = runner.trim_recent_summaries(&content, 10);
        let trimmed_lines: Vec<&str> = trimmed.lines().collect();
        // 2 行表头 + 10 行数据
        assert_eq!(trimmed_lines.len(), 12);
        // 数据行应是第 6..=15 行（最近 10 条）
        assert!(trimmed_lines[2].contains("标题6"));
        assert!(trimmed_lines[11].contains("标题15"));
    }

    #[test]
    fn trim_recent_summaries_returns_empty_when_no_data() {
        let runner = PipelineRunner::new(PipelineConfig::default());
        let content = "| 章节 | 标题 |\n| --- | --- |";
        let trimmed = runner.trim_recent_summaries(content, 10);
        assert!(trimmed.is_empty());
    }

    #[test]
    fn current_iso_returns_valid_iso() {
        let iso = current_iso();
        // RFC3339 格式包含 'T' 分隔符和时区偏移（+08:00 或 Z）
        assert!(iso.contains('T'), "ISO 时间戳应包含 T 分隔符: {}", iso);
        // 时区偏移：以 + 或 Z 结尾特征
        assert!(
            iso.contains('+') || iso.ends_with('Z'),
            "ISO 时间戳应包含时区信息: {}", iso
        );
    }
}
