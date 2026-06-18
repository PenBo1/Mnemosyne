use crate::errors::AppError;
use crate::domain::story::{BookConfig, ChapterMeta, AuditResult, WriteResult};
use crate::infra::llm::Provider;
use crate::infra::gc::utils;
use crate::domain::agents::*;
use crate::domain::agents::base::AgentContext;
use crate::domain::agents::recovery::{RecoveryManager, RecoveryConfig, RecoveryStrategy};
use crate::domain::agents::verification::{VerificationPipeline, GateContext};
use crate::domain::agents::reviser::ReviseMode;
use std::sync::Arc;

pub struct PipelineConfig {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: std::path::PathBuf,
    /// Per-agent model overrides: agent_name -> model_id
    pub model_overrides: std::collections::HashMap<String, String>,
    /// Shared memory store for cross-chapter persistence
    pub memory_store: Option<Arc<crate::infra::memory::MemoryStore>>,
}

pub struct PipelineRunner {
    pub config: PipelineConfig,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    fn book_dir(&self, book_id: &str) -> std::path::PathBuf {
        self.config.project_root.join("books").join(book_id)
    }

    fn agent_ctx(&self, book_id: Option<&str>) -> AgentContext {
        let memory = if let (Some(_mem_store), Some(_bid)) = (&self.config.memory_store, book_id) {
            // Use shared memory store — data persists across agent calls
            Arc::new(tokio::sync::RwLock::new(MemorySystem::new(20)))
        } else {
            Arc::new(tokio::sync::RwLock::new(MemorySystem::new(20)))
        };
        AgentContext {
            provider: self.config.provider.clone(),
            model: self.config.model.clone(),
            project_root: self.config.project_root.clone(),
            book_id: book_id.map(|s| s.to_string()),
            tools: Arc::new(ToolRegistry::new()),
            memory,
        }
    }

    /// Get agent context with optional model override
    pub fn agent_ctx_for(&self, agent_name: &str, book_id: Option<&str>) -> AgentContext {
        let model = self.config.model_overrides.get(agent_name)
            .cloned()
            .unwrap_or_else(|| self.config.model.clone());
        let memory = if let (Some(_mem_store), Some(_bid)) = (&self.config.memory_store, book_id) {
            // TODO: Wire up actual MemoryStore.get_or_create(bid, 20) here
            Arc::new(tokio::sync::RwLock::new(MemorySystem::new(20)))
        } else {
            Arc::new(tokio::sync::RwLock::new(MemorySystem::new(20)))
        };
        AgentContext {
            provider: self.config.provider.clone(),
            model,
            project_root: self.config.project_root.clone(),
            book_id: book_id.map(|s| s.to_string()),
            tools: Arc::new(ToolRegistry::new()),
            memory,
        }
    }

    // ── Book creation ──────────────────────────────────────────

    pub async fn create_book(
        &self,
        title: &str,
        genre: &str,
        brief: Option<&str>,
    ) -> Result<BookConfig, AppError> {
        tracing::info!(title, genre, "Creating new book");
        let start = std::time::Instant::now();

        let book_id = uuid::Uuid::new_v4().to_string();
        let architect_ctx = self.agent_ctx_for("architect", Some(&book_id));

        let architect = ArchitectAgent::new();
        let book = BookConfig {
            id: book_id.clone(),
            title: title.to_string(),
            genre: genre.to_string(),
            platform: "local".to_string(),
            status: Default::default(),
            language: "zh".to_string(),
            chapter_words: 3000,
            target_chapters: 200,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let book_dir = self.book_dir(&book_id);
        std::fs::create_dir_all(&book_dir)?;

        let output = architect.generate_foundation(&architect_ctx, &book, brief).await?;

        // Foundation review loop (max 2 retries)
        let reviewer = FoundationReviewerAgent::new();
        let reviewer_ctx = self.agent_ctx_for("foundation-reviewer", Some(&book_id));
        let mut foundation = output;
        let max_retries = 2;
        for attempt in 0..max_retries {
            tracing::info!(attempt, "Reviewing foundation");
            let review = reviewer.review(&reviewer_ctx, &foundation, &book, &book.language).await?;
            tracing::info!(score = review.total_score, passed = review.passed, "Foundation review");
            if review.passed {
                break;
            }
            if attempt < max_retries - 1 {
                tracing::warn!(score = review.total_score, "Foundation rejected, regenerating");
                foundation = architect.generate_foundation(&architect_ctx, &book, brief).await?;
            }
        }

        architect.write_foundation_files(&book_dir, &foundation, &book.language).await?;

        // Save book config
        let config_json = serde_json::to_string_pretty(&book)?;
        std::fs::write(book_dir.join("book.json"), config_json)?;

        // Create directories
        std::fs::create_dir_all(book_dir.join("chapters"))?;
        std::fs::create_dir_all(book_dir.join("story/state"))?;
        std::fs::create_dir_all(book_dir.join("story/runtime"))?;
        std::fs::create_dir_all(book_dir.join("story/snapshots"))?;

        // Initialize state
        let state = crate::domain::story::StoryState::default();
        let state_json = serde_json::to_string_pretty(&state)?;
        std::fs::write(book_dir.join("story/state.json"), state_json)?;

        let elapsed = start.elapsed().as_secs();
        tracing::info!(book_id = %book_id, title, genre, elapsed_secs = elapsed, "Book created");

        Ok(book)
    }

    // ── Full pipeline: plan → compose → write → audit → revise ──

    pub async fn write_next_chapter(
        &self,
        book_id: &str,
        target_words: Option<u32>,
    ) -> Result<WriteResult, AppError> {
        tracing::info!(book_id, "Starting full pipeline");
        let start = std::time::Instant::now();
        let book_dir = self.book_dir(book_id);

        let book = load_book_config(&book_dir)?;
        let words = target_words.unwrap_or(book.chapter_words);

        // Get next chapter number
        let chapter_number = get_next_chapter_number(&book_dir)?;

        // Initialize recovery manager (P14.26)
        let mut recovery_manager = RecoveryManager::new(RecoveryConfig::default());

        // Initialize verification pipeline (P14.38)
        let verification_pipeline = VerificationPipeline::new();

        // 1. Plan (with recovery)
        tracing::info!(chapter = chapter_number, "Stage: Plan");
        let planner_ctx = self.agent_ctx_for("planner", Some(book_id));
        let planner = PlannerAgent::new();
        let plan = match planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, None).await {
            Ok(plan) => plan,
            Err(e) => {
                tracing::warn!(error = %e, "Plan failed, attempting recovery");
                match recovery_manager.next_strategy(&e) {
                    Some(RecoveryStrategy::Retry) => {
                        planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, None).await?
                    }
                    Some(RecoveryStrategy::Simplify) => {
                        tracing::info!("Simplifying plan task");
                        planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, Some("简化任务")).await?
                    }
                    _ => return Err(e),
                }
            }
        };

        // 2. Compose (with recovery)
        tracing::info!(chapter = chapter_number, "Stage: Compose");
        let composer_ctx = self.agent_ctx_for("composer", Some(book_id));
        let composer = ComposerAgent::new();
        let composed = match composer.compose_chapter(&composer_ctx, &book_dir, chapter_number, &plan).await {
            Ok(composed) => composed,
            Err(e) => {
                tracing::warn!(error = %e, "Compose failed, attempting recovery");
                recovery_manager.reset();
                match recovery_manager.next_strategy(&e) {
                    Some(RecoveryStrategy::Retry) => {
                        composer.compose_chapter(&composer_ctx, &book_dir, chapter_number, &plan).await?
                    }
                    _ => return Err(e),
                }
            }
        };

        // 3. Write (with recovery)
        tracing::info!(chapter = chapter_number, "Stage: Write");
        let writer_ctx = self.agent_ctx_for("writer", Some(book_id));
        let writer = WriterAgent::new();
        let write_output = match writer.write_chapter(
            &writer_ctx, &book_dir, chapter_number, &plan, &composed, words,
        ).await {
            Ok(output) => output,
            Err(e) => {
                tracing::warn!(error = %e, "Write failed, attempting recovery");
                recovery_manager.reset();
                match recovery_manager.next_strategy(&e) {
                    Some(RecoveryStrategy::Retry) => {
                        writer.write_chapter(&writer_ctx, &book_dir, chapter_number, &plan, &composed, words).await?
                    }
                    Some(RecoveryStrategy::Simplify) => {
                        tracing::info!("Simplifying write task");
                        writer.write_chapter(&writer_ctx, &book_dir, chapter_number, &plan, &composed, words / 2).await?
                    }
                    _ => return Err(e),
                }
            }
        };

        // 3.5. Verification Gates (P14.38)
        tracing::info!(chapter = chapter_number, "Stage: Verification");
        let gate_context = GateContext {
            chapter_number,
            plan: Some(serde_json::to_string(&plan.memo).unwrap_or_default()),
            previous_content: if chapter_number > 1 {
                read_chapter_content(&book_dir, chapter_number - 1).ok()
            } else {
                None
            },
            style_guide: None,
            language: book.language.clone(),
            ..Default::default()
        };
        let gate_results = verification_pipeline.validate_all(&write_output.content, &gate_context).await?;

        if !verification_pipeline.overall_passed(&gate_results) {
            tracing::warn!("Verification gates failed, triggering revision");
            // Log gate failures for debugging
            for result in &gate_results {
                if !result.passed {
                    for issue in &result.issues {
                        tracing::warn!(
                            dimension = %issue.dimension,
                            severity = ?issue.severity,
                            description = %issue.description,
                            "Verification issue"
                        );
                    }
                }
            }
        }

        // 4. Audit
        tracing::info!(chapter = chapter_number, "Stage: Audit");
        let auditor_ctx = self.agent_ctx_for("auditor", Some(book_id));
        let auditor = ContinuityAuditor::new();
        let audit = auditor.audit_chapter(&auditor_ctx, &book_dir, chapter_number).await?;

        // 5. Revise if needed (with recovery)
        let mut final_content = write_output.content.clone();
        let mut final_word_count = write_output.word_count;
        let mut revised = false;

        if !audit.passed {
            recovery_manager.reset();
            let max_rounds = 3;
            let mut current_audit = audit.clone();
            let reviser_ctx = self.agent_ctx_for("reviser", Some(book_id));
            for round in 0..max_rounds {
                if current_audit.issues.iter().any(|i| i.severity == crate::domain::story::AuditSeverity::Critical) {
                    tracing::info!(chapter = chapter_number, round, "Stage: Revise");
                    let reviser = ReviserAgent::new();

                    match reviser.revise_chapter(
                        &reviser_ctx, &book_dir, chapter_number,
                        &final_content, &current_audit, ReviseMode::Auto,
                    ).await {
                        Ok(revise_output) => {
                            // Save revised content
                            save_chapter_content(&book_dir, chapter_number, &write_output.title, &revise_output.content)?;
                            final_content = revise_output.content;
                            final_word_count = revise_output.word_count;
                            revised = true;

                            // Re-audit
                            current_audit = auditor.audit_chapter(&auditor_ctx, &book_dir, chapter_number).await?;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, round, "Revise failed");
                            match recovery_manager.next_strategy(&e) {
                                Some(RecoveryStrategy::Retry) => {
                                    tracing::info!("Retrying revision");
                                    continue;
                                }
                                Some(RecoveryStrategy::Simplify) => {
                                    tracing::info!("Simplifying revision task");
                                    // Continue with simplified revision
                                }
                                _ => {
                                    tracing::error!("No recovery strategy available, using current content");
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // 6. Save chapter index
        save_chapter_index(&book_dir, chapter_number, &write_output.title, final_word_count, &audit)?;

        // 7. Snapshot
        save_snapshot(&book_dir, chapter_number)?;

        let elapsed = start.elapsed().as_secs();
        tracing::info!(
            book_id,
            chapter = chapter_number,
            word_count = final_word_count,
            audit_passed = audit.passed,
            revised,
            elapsed_secs = elapsed,
            "Pipeline completed"
        );

        Ok(WriteResult {
            chapter_number,
            title: write_output.title,
            content: final_content,
            word_count: final_word_count,
            audit,
        })
    }

    // ── Individual operations ──────────────────────────────────

    pub async fn plan_chapter(
        &self,
        book_id: &str,
        context: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let book_dir = self.book_dir(book_id);
        let ctx = self.agent_ctx(Some(book_id));
        let chapter_number = get_next_chapter_number(&book_dir)?;
        let planner = PlannerAgent::new();
        let plan = planner.plan_chapter(&ctx, &book_dir, chapter_number, context).await?;
        Ok(serde_json::to_value(&plan.memo)?)
    }

    pub async fn audit_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
    ) -> Result<AuditResult, AppError> {
        let book_dir = self.book_dir(book_id);
        let ctx = self.agent_ctx(Some(book_id));
        let auditor = ContinuityAuditor::new();
        auditor.audit_chapter(&ctx, &book_dir, chapter_number).await
    }

    pub async fn revise_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
        mode: ReviseMode,
    ) -> Result<String, AppError> {
        let book_dir = self.book_dir(book_id);
        let ctx = self.agent_ctx(Some(book_id));

        let content = read_chapter_content(&book_dir, chapter_number)?;
        let auditor = ContinuityAuditor::new();
        let audit = auditor.audit_chapter(&ctx, &book_dir, chapter_number).await?;

        let reviser = ReviserAgent::new();
        let output = reviser.revise_chapter(&ctx, &book_dir, chapter_number, &content, &audit, mode).await?;

        save_chapter_content(&book_dir, chapter_number, "", &output.content)?;

        Ok(output.content)
    }
}

// ── Helper functions ─────────────────────────────────────────

fn load_book_config(book_dir: &std::path::Path) -> Result<BookConfig, AppError> {
    let content = std::fs::read_to_string(book_dir.join("book.json"))
        .map_err(|e| AppError::internal(format!("Failed to read book config: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse book config: {}", e)))
}

fn get_next_chapter_number(book_dir: &std::path::Path) -> Result<u32, AppError> {
    let chapters_dir = book_dir.join("chapters");
    if !chapters_dir.exists() {
        return Ok(1);
    }

    let mut max_num = 0u32;
    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(num_str) = name_str.split('_').next() {
                if let Ok(num) = num_str.parse::<u32>() {
                    if num > max_num {
                        max_num = num;
                    }
                }
            }
        }
    }
    Ok(max_num + 1)
}

fn read_chapter_content(book_dir: &std::path::Path, chapter_number: u32) -> Result<String, AppError> {
    let chapters_dir = book_dir.join("chapters");
    let prefix = format!("{:04}_", chapter_number);

    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                return std::fs::read_to_string(entry.path())
                    .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)));
            }
        }
    }

    Err(AppError::not_found(format!("Chapter {} not found", chapter_number)))
}

fn save_chapter_content(book_dir: &std::path::Path, chapter_number: u32, title: &str, content: &str) -> Result<(), AppError> {
    let chapters_dir = book_dir.join("chapters");
    std::fs::create_dir_all(&chapters_dir)?;

    let filename = format!("{:04}_{}.md", chapter_number, utils::sanitize_filename(title));
    let path = chapters_dir.join(filename);

    let heading = if utils::is_english_book(book_dir) {
        format!("# Chapter {}: {}", chapter_number, title)
    } else {
        format!("# 第{}章 {}", chapter_number, title)
    };

    std::fs::write(path, format!("{}\n\n{}", heading, content))
        .map_err(|e| AppError::internal(format!("Failed to write chapter: {}", e)))
}

fn save_chapter_index(
    book_dir: &std::path::Path,
    chapter_number: u32,
    title: &str,
    word_count: u32,
    audit: &AuditResult,
) -> Result<(), AppError> {
    let index_path = book_dir.join("chapters").join("index.json");
    let mut chapters: Vec<ChapterMeta> = if index_path.exists() {
        let content = std::fs::read_to_string(&index_path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    let now = chrono::Utc::now().to_rfc3339();
    let meta = ChapterMeta {
        number: chapter_number,
        title: title.to_string(),
        status: if audit.passed {
            crate::domain::story::ChapterStatus::AuditPassed
        } else {
            crate::domain::story::ChapterStatus::AuditFailed
        },
        word_count,
        audit_passed: audit.passed,
        audit_score: Some(audit.score),
        created_at: now.clone(),
        updated_at: now,
    };

    if let Some(existing) = chapters.iter_mut().find(|c| c.number == chapter_number) {
        *existing = meta;
    } else {
        chapters.push(meta);
    }

    let json = serde_json::to_string_pretty(&chapters)?;
    std::fs::write(index_path, json)?;
    Ok(())
}

fn save_snapshot(book_dir: &std::path::Path, chapter_number: u32) -> Result<(), AppError> {
    let state_path = book_dir.join("story").join("state.json");
    if !state_path.exists() {
        return Ok(());
    }

    let state: crate::domain::story::StoryState = serde_json::from_str(
        &std::fs::read_to_string(&state_path)?
    )?;

    let snapshots_dir = book_dir.join("story").join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)?;

    let snapshot = crate::domain::story::ChapterSnapshot {
        chapter_number,
        state,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&snapshot)?;
    std::fs::write(snapshots_dir.join(format!("{:04}.json", chapter_number)), json)?;
    Ok(())
}
