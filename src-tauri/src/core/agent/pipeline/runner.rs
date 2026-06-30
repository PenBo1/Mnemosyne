use crate::shared::errors::AppError;
use crate::features::story::{BookConfig, ChapterMeta, AuditResult, WriteResult};
use crate::infrastructure::llm_client::Provider;
use crate::infrastructure::state_store::gc::utils;
use crate::core::agent::*;
use crate::core::agent::base::AgentContext;
use crate::core::agent::recovery::{RecoveryManager, RecoveryConfig, RecoveryStrategy};
use crate::core::agent::verification::{VerificationPipeline, GateContext};
use crate::core::agent::reviser::ReviseMode;
use crate::core::agent::iteration_budget::IterationBudget;
use crate::core::agent::tool_guardrails::{ToolCallGuardrailController, ToolGuardrailConfig};
use crate::core::agent::context_compressor::{ContextCompressor, CompressorConfig};
use crate::core::agent::error_classifier::classify_api_error;
use crate::core::agent::lesson_tracker::{LessonTracker, append_lessons_to_memory, load_lessons_from_memory};
use crate::core::agent::task_lifecycle::TaskManager;
use crate::core::agent::tools::{ReadFileTool, WriteFileTool, ListFilesTool, BashTool, SearchMemoryTool, ArchiveMemoryTool};
use crate::infrastructure::sandbox::SandboxEnforcer;
use crate::infrastructure::sandbox::policy::SandboxPolicy;
use std::sync::Arc;

pub struct PipelineConfig {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: std::path::PathBuf,
    /// Per-agent model overrides: agent_name -> model_id
    pub model_overrides: std::collections::HashMap<String, String>,
    /// Shared memory store for cross-chapter persistence
    pub memory_store: Option<Arc<crate::infrastructure::state_store::memory::MemoryStore>>,
    /// App data directory for loading agent identity files (SOUL.md, CONTEXT.md, MEMORY.md)
    pub data_dir: crate::infrastructure::file_storage::data_dir::DataDir,
    /// User profile for tailoring agent output
    pub user_profile: Option<Arc<tokio::sync::Mutex<crate::features::user_profile::UserProfileStore>>>,
    /// Fallback model for RecoveryStrategy::FallbackModel (None = no fallback, fail explicitly)
    pub fallback_model: Option<String>,
}

pub struct PipelineRunner {
    pub config: PipelineConfig,
    pub task_manager: std::sync::Mutex<TaskManager>,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            task_manager: std::sync::Mutex::new(TaskManager::new()),
        }
    }

    fn book_dir(&self, book_id: &str) -> std::path::PathBuf {
        self.config.project_root.join("books").join(book_id)
    }

    async fn agent_ctx(&self, book_id: Option<&str>) -> AgentContext {
        let memory = if let (Some(mem_store), Some(bid)) = (&self.config.memory_store, book_id) {
            mem_store.get_or_create(bid, 20).await
        } else {
            Arc::new(tokio::sync::RwLock::new(MemorySystem::new(20)))
        };
        let mut tools = ToolRegistry::new();
        let work_dir = self.config.project_root.clone();

        tools.register("read_file", Box::new(ReadFileTool::new(work_dir.clone())));
        tools.register("write_file", Box::new(WriteFileTool::new(work_dir.clone())));
        tools.register("list_files", Box::new(ListFilesTool::new(work_dir.clone())));

        tools.register("search_memory", Box::new(SearchMemoryTool::new(memory.clone())));
        tools.register("archive_memory", Box::new(ArchiveMemoryTool::new(memory.clone())));

        let sandbox = Arc::new(SandboxEnforcer::new(SandboxPolicy::restricted(), work_dir.clone()));
        tools.register("bash", Box::new(BashTool::new(work_dir.clone(), Some(sandbox))));

        AgentContext {
            provider: self.config.provider.clone(),
            model: self.config.model.clone(),
            project_root: self.config.project_root.clone(),
            book_id: book_id.map(|s| s.to_string()),
            tools: Arc::new(tools),
            memory,
            iteration_budget: Arc::new(IterationBudget::new(90)),
            tool_guardrails: Arc::new(tokio::sync::Mutex::new(
                ToolCallGuardrailController::new(ToolGuardrailConfig::default())
            )),
            context_compressor: Arc::new(tokio::sync::Mutex::new(
                ContextCompressor::new(CompressorConfig::default())
            )),
            skill_manager: None,
            user_profile: self.config.user_profile.clone(),
        }
    }

    /// Get agent context with optional model override.
    /// All agents receive the standard tool set; per-agent tool filtering is not yet supported.
    pub async fn agent_ctx_for(&self, agent_name: &str, book_id: Option<&str>) -> AgentContext {
        let model = self.config.model_overrides.get(agent_name)
            .cloned()
            .unwrap_or_else(|| self.config.model.clone());
        let memory = if let (Some(mem_store), Some(bid)) = (&self.config.memory_store, book_id) {
            mem_store.get_or_create(bid, 20).await
        } else {
            Arc::new(tokio::sync::RwLock::new(MemorySystem::new(20)))
        };

        let mut tools = ToolRegistry::new();
        let work_dir = self.config.project_root.clone();

        tools.register("read_file", Box::new(ReadFileTool::new(work_dir.clone())));
        tools.register("write_file", Box::new(WriteFileTool::new(work_dir.clone())));
        tools.register("list_files", Box::new(ListFilesTool::new(work_dir.clone())));
        tools.register("search_memory", Box::new(SearchMemoryTool::new(memory.clone())));
        tools.register("archive_memory", Box::new(ArchiveMemoryTool::new(memory.clone())));

        let sandbox = Arc::new(SandboxEnforcer::new(SandboxPolicy::restricted(), work_dir.clone()));
        tools.register("bash", Box::new(BashTool::new(work_dir.clone(), Some(sandbox))));

        AgentContext {
            provider: self.config.provider.clone(),
            model,
            project_root: self.config.project_root.clone(),
            book_id: book_id.map(|s| s.to_string()),
            tools: Arc::new(tools),
            memory,
            iteration_budget: Arc::new(IterationBudget::new(50)),
            tool_guardrails: Arc::new(tokio::sync::Mutex::new(
                ToolCallGuardrailController::new(ToolGuardrailConfig::default())
            )),
            context_compressor: Arc::new(tokio::sync::Mutex::new(
                ContextCompressor::new(CompressorConfig::default())
            )),
            skill_manager: None,
            user_profile: self.config.user_profile.clone(),
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
        let architect_ctx = self.agent_ctx_for("architect", Some(&book_id)).await;

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

        let output = architect.generate_foundation(&architect_ctx, &book, brief, &self.config.data_dir).await?;

        // Foundation review loop (max 2 retries)
        let reviewer = FoundationReviewerAgent::new();
        let reviewer_ctx = self.agent_ctx_for("foundation-reviewer", Some(&book_id)).await;
        let mut foundation = output;
        let max_retries = 2;
        for attempt in 0..max_retries {
            tracing::info!(attempt, "Reviewing foundation");
            let review = reviewer.review(&reviewer_ctx, &foundation, &book, &book.language, &self.config.data_dir).await?;
            tracing::info!(score = review.total_score, passed = review.passed, "Foundation review");
            if review.passed {
                break;
            }
            if attempt < max_retries - 1 {
                tracing::warn!(score = review.total_score, "Foundation rejected, regenerating");
                foundation = architect.generate_foundation(&architect_ctx, &book, brief, &self.config.data_dir).await?;
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
        let state = crate::features::story::StoryState::default();
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

        // Initialize recovery manager (P14.26) — now uses ErrorClassifier
        let recovery_config = RecoveryConfig {
            fallback_model: self.config.fallback_model.clone(),
            ..RecoveryConfig::default()
        };
        let mut recovery_manager = RecoveryManager::new(recovery_config);

        // Initialize verification pipeline (P14.38)
        let verification_pipeline = VerificationPipeline::new();

        // ── 创建共享的 Agent 上下文（包含 IterationBudget + ToolGuardrails + ContextCompressor）──
        let shared_ctx = self.agent_ctx_for("pipeline", Some(book_id)).await;

        // 1. Plan (with recovery + budget + guardrails)
        tracing::info!(chapter = chapter_number, "Stage: Plan");
        let planner_ctx = self.agent_ctx_for("planner", Some(book_id)).await;

        // 检查迭代预算
        if !planner_ctx.iteration_budget.consume() {
            tracing::error!(
                budget_used = planner_ctx.iteration_budget.used(),
                budget_max = planner_ctx.iteration_budget.max_total(),
                "Iteration budget exhausted before Plan stage"
            );
            return Err(AppError::internal("迭代预算已耗尽，无法执行 Plan 阶段"));
        }

        // 重置工具守卫
        planner_ctx.tool_guardrails.lock().await.reset_for_turn();

        let planner = PlannerAgent::new();
        let plan = match planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, None, &self.config.data_dir).await {
            Ok(plan) => plan,
            Err(e) => {
                let error_msg = e.to_string();
                let classified = classify_api_error(&error_msg, None, "", "");
                tracing::warn!(
                    error = %e,
                    reason = ?classified.reason,
                    should_compress = classified.should_compress,
                    "Plan failed, attempting recovery"
                );

                // 注意：Plan 阶段无长对话历史可压缩（planner 无状态），上下文溢出
                // 由 RecoveryStrategy::CompressContext 分支处理（重试）。真正的压缩
                // 适用于有状态长对话场景（见 ContextCompressor::compress）。

                match recovery_manager.next_strategy(&e) {
                    Some(RecoveryStrategy::Retry) => {
                        planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, None, &self.config.data_dir).await?
                    }
                    Some(RecoveryStrategy::Simplify) => {
                        tracing::info!("Simplifying plan task");
                        planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, Some("简化任务"), &self.config.data_dir).await?
                    }
                    Some(RecoveryStrategy::CompressContext) => {
                        tracing::info!("Retrying after context compression");
                        planner.plan_chapter(&planner_ctx, &book_dir, chapter_number, None, &self.config.data_dir).await?
                    }
                    Some(RecoveryStrategy::FallbackModel) => {
                        let fallback = self.config.fallback_model.as_ref()
                            .ok_or_else(|| AppError::internal(
                                "Fallback model requested by recovery manager but none configured"
                            ))?;
                        tracing::info!(fallback_model = %fallback, "Switching to fallback model");
                        let mut fallback_ctx = planner_ctx.clone();
                        fallback_ctx.model = fallback.clone();
                        planner.plan_chapter(&fallback_ctx, &book_dir, chapter_number, None, &self.config.data_dir).await?
                    }
                    _ => return Err(e),
                }
            }
        };

        // 2. Compose (with recovery + budget + guardrails)
        tracing::info!(chapter = chapter_number, "Stage: Compose");
        let composer_ctx = self.agent_ctx_for("composer", Some(book_id)).await;

        if !composer_ctx.iteration_budget.consume() {
            tracing::error!("Iteration budget exhausted before Compose stage");
            return Err(AppError::internal("迭代预算已耗尽，无法执行 Compose 阶段"));
        }
        composer_ctx.tool_guardrails.lock().await.reset_for_turn();

        let composer = ComposerAgent::new();
        let composed = match composer.compose_chapter(&composer_ctx, &book_dir, chapter_number, &plan, &self.config.data_dir).await {
            Ok(composed) => composed,
            Err(e) => {
                let classified = classify_api_error(&e.to_string(), None, "", "");
                tracing::warn!(error = %e, reason = ?classified.reason, "Compose failed, attempting recovery");
                recovery_manager.reset();
                match recovery_manager.next_strategy(&e) {
                    Some(RecoveryStrategy::Retry) => {
                        composer.compose_chapter(&composer_ctx, &book_dir, chapter_number, &plan, &self.config.data_dir).await?
                    }
                    Some(RecoveryStrategy::CompressContext) => {
                        tracing::info!("Retrying compose after context compression");
                        composer.compose_chapter(&composer_ctx, &book_dir, chapter_number, &plan, &self.config.data_dir).await?
                    }
                    _ => return Err(e),
                }
            }
        };

        // 3. Write (with recovery + budget + guardrails)
        tracing::info!(chapter = chapter_number, "Stage: Write");
        let writer_ctx = self.agent_ctx_for("writer", Some(book_id)).await;

        if !writer_ctx.iteration_budget.consume() {
            tracing::error!("Iteration budget exhausted before Write stage");
            return Err(AppError::internal("迭代预算已耗尽，无法执行 Write 阶段"));
        }
        writer_ctx.tool_guardrails.lock().await.reset_for_turn();

        let writer = WriterAgent::new();
        let write_output = match writer.write_chapter(
            &writer_ctx, &book_dir, chapter_number, &plan, &composed, words,
            &self.config.data_dir,
        ).await {
            Ok(output) => output,
            Err(e) => {
                let classified = classify_api_error(&e.to_string(), None, "", "");
                tracing::warn!(error = %e, reason = ?classified.reason, "Write failed, attempting recovery");
                recovery_manager.reset();
                match recovery_manager.next_strategy(&e) {
                    Some(RecoveryStrategy::Retry) => {
                        writer.write_chapter(&writer_ctx, &book_dir, chapter_number, &plan, &composed, words, &self.config.data_dir).await?
                    }
                    Some(RecoveryStrategy::Simplify) => {
                        tracing::info!("Simplifying write task");
                        writer.write_chapter(&writer_ctx, &book_dir, chapter_number, &plan, &composed, words / 2, &self.config.data_dir).await?
                    }
                    Some(RecoveryStrategy::CompressContext) => {
                        tracing::info!("Retrying write after context compression");
                        writer.write_chapter(&writer_ctx, &book_dir, chapter_number, &plan, &composed, words, &self.config.data_dir).await?
                    }
                    _ => return Err(e),
                }
            }
        };

        // ── 上下文压缩检查（写入完成后检查是否需要压缩）──
        if let Ok(token_estimate) = estimate_content_tokens(&write_output.content) {
            let compressor = shared_ctx.context_compressor.lock().await;
            if compressor.should_compress(token_estimate, 200000) {
                tracing::info!(
                    estimated_tokens = token_estimate,
                    "Content approaching context limit, compression recommended for next chapter"
                );
            }
        }

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

        // 4. Audit (with budget check)
        tracing::info!(chapter = chapter_number, "Stage: Audit");
        let auditor_ctx = self.agent_ctx_for("auditor", Some(book_id)).await;

        if !auditor_ctx.iteration_budget.consume() {
            tracing::warn!("Iteration budget exhausted before Audit stage, skipping audit");
            // 审计失败不阻塞流程，跳过
        } else {
            auditor_ctx.tool_guardrails.lock().await.reset_for_turn();
        }

        let auditor = ContinuityAuditor::new();
        let audit = if auditor_ctx.iteration_budget.used() > 0 {
            auditor.audit_chapter(&auditor_ctx, &book_dir, chapter_number, &self.config.data_dir).await?
        } else {
            // 预算耗尽，跳过审计
            AuditResult {
                passed: true,
                score: 0.0,
                issues: vec![],
                summary: "审计已跳过（迭代预算耗尽）".to_string(),
            }
        };

        // 5. Revise if needed (with recovery + budget + guardrails)
        let mut final_content = write_output.content.clone();
        let mut final_word_count = write_output.word_count;
        let mut revised = false;

        if !audit.passed {
            recovery_manager.reset();
            let max_rounds = 3;
            let mut current_audit = audit.clone();
            let reviser_ctx = self.agent_ctx_for("reviser", Some(book_id)).await;

            for round in 0..max_rounds {
                // 检查迭代预算
                if !reviser_ctx.iteration_budget.consume() {
                    tracing::warn!(
                        round,
                        budget_used = reviser_ctx.iteration_budget.used(),
                        "Iteration budget exhausted during revision, stopping"
                    );
                    break;
                }

                // 重置工具守卫
                reviser_ctx.tool_guardrails.lock().await.reset_for_turn();

                if current_audit.issues.iter().any(|i| i.severity == crate::features::story::AuditSeverity::Critical) {
                    tracing::info!(chapter = chapter_number, round, "Stage: Revise");
                    let reviser = ReviserAgent::new();

                    match reviser.revise_chapter(
                        &reviser_ctx, &book_dir, chapter_number,
                        &final_content, &current_audit, ReviseMode::Auto,
                        &self.config.data_dir,
                    ).await {
                        Ok(revise_output) => {
                            save_chapter_content(&book_dir, chapter_number, &write_output.title, &revise_output.content)?;
                            final_content = revise_output.content;
                            final_word_count = revise_output.word_count;
                            revised = true;

                            current_audit = auditor.audit_chapter(&auditor_ctx, &book_dir, chapter_number, &self.config.data_dir).await?;
                        }
                        Err(e) => {
                            let classified = classify_api_error(&e.to_string(), None, "", "");
                            tracing::warn!(error = %e, round, reason = ?classified.reason, "Revise failed");
                            match recovery_manager.next_strategy(&e) {
                                Some(RecoveryStrategy::Retry) => {
                                    tracing::info!("Retrying revision");
                                    continue;
                                }
                                Some(RecoveryStrategy::Simplify) => {
                                    tracing::info!("Simplifying revision task");
                                }
                                Some(RecoveryStrategy::CompressContext) => {
                                    tracing::info!("Retrying revision after context compression");
                                    continue;
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

        // 7. Self-evolution: record audit issues and generate constraint lessons
        //    Aligned with Hermes Agent's feedback loop: error events → lessons → prompt injection
        tracing::info!(chapter = chapter_number, "Stage: Self-evolution (lesson tracking)");
        let existing_lessons = load_lessons_from_memory(&self.config.data_dir, "writer");
        let mut lesson_tracker = LessonTracker::default_config();
        lesson_tracker.load_lessons(existing_lessons);

        let new_lessons = lesson_tracker.record_audit("writer", &audit);
        if !new_lessons.is_empty() {
            tracing::info!(
                count = new_lessons.len(),
                "Generated new constraint lessons for writer"
            );
            if let Err(e) = append_lessons_to_memory(&self.config.data_dir, "writer", &new_lessons) {
                tracing::warn!(error = %e, "Failed to write lessons to writer MEMORY.md");
            }
        }

        // Also track planner lessons if plan had issues
        // (plan issues manifest as writer issues, but we track them separately)
        if !audit.passed {
            let existing_planner_lessons = load_lessons_from_memory(&self.config.data_dir, "planner");
            let mut planner_tracker = LessonTracker::default_config();
            planner_tracker.load_lessons(existing_planner_lessons);

            // Record high-level audit failures as planner lessons
            let planner_audit = AuditResult {
                passed: audit.passed,
                score: audit.score,
                issues: audit.issues.iter().filter(|i| {
                    matches!(i.severity, crate::features::story::AuditSeverity::Critical)
                }).cloned().collect(),
                summary: audit.summary.clone(),
            };
            let new_planner_lessons = planner_tracker.record_audit("planner", &planner_audit);
            if !new_planner_lessons.is_empty() {
                if let Err(e) = append_lessons_to_memory(&self.config.data_dir, "planner", &new_planner_lessons) {
                    tracing::warn!(error = %e, "Failed to write lessons to planner MEMORY.md");
                }
            }
        }

        // 8. Snapshot
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

    /// Write a chapter with goal-oriented decomposition
    pub async fn write_chapter_with_goal(
        &self,
        book_id: &str,
        goal: &str,
    ) -> Result<WriteResult, AppError> {
        // Decompose goal into subtasks
        let ctx = self.agent_ctx(Some(book_id)).await;
        let task_ids = {
            let mut tm = self.task_manager.lock().map_err(|_| AppError::internal("Lock poisoned"))?;
            GoalDecomposer::decompose(&ctx, goal, &mut *tm).await?
        };

        tracing::info!(
            book_id,
            goal,
            tasks = ?task_ids,
            "Goal decomposed into subtasks"
        );

        // Execute the standard pipeline
        self.write_next_chapter(book_id, None).await
    }

    // ── Individual operations ──────────────────────────────────

    pub async fn plan_chapter(
        &self,
        book_id: &str,
        context: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let book_dir = self.book_dir(book_id);
        let ctx = self.agent_ctx(Some(book_id)).await;
        let chapter_number = get_next_chapter_number(&book_dir)?;
        let planner = PlannerAgent::new();
        let plan = planner.plan_chapter(&ctx, &book_dir, chapter_number, context, &self.config.data_dir).await?;
        Ok(serde_json::to_value(&plan.memo)?)
    }

    pub async fn audit_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
    ) -> Result<AuditResult, AppError> {
        let book_dir = self.book_dir(book_id);
        let ctx = self.agent_ctx(Some(book_id)).await;
        let auditor = ContinuityAuditor::new();
        auditor.audit_chapter(&ctx, &book_dir, chapter_number, &self.config.data_dir).await
    }

    pub async fn revise_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
        mode: ReviseMode,
    ) -> Result<String, AppError> {
        let book_dir = self.book_dir(book_id);
        let ctx = self.agent_ctx(Some(book_id)).await;

        let content = read_chapter_content(&book_dir, chapter_number)?;
        let auditor = ContinuityAuditor::new();
        let audit = auditor.audit_chapter(&ctx, &book_dir, chapter_number, &self.config.data_dir).await?;

        let reviser = ReviserAgent::new();
        let output = reviser.revise_chapter(&ctx, &book_dir, chapter_number, &content, &audit, mode, &self.config.data_dir).await?;

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
            crate::features::story::ChapterStatus::AuditPassed
        } else {
            crate::features::story::ChapterStatus::AuditFailed
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

    let state: crate::features::story::StoryState = serde_json::from_str(
        &std::fs::read_to_string(&state_path)?
    )?;

    let snapshots_dir = book_dir.join("story").join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)?;

    let snapshot = crate::features::story::ChapterSnapshot {
        chapter_number,
        state,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&snapshot)?;
    std::fs::write(snapshots_dir.join(format!("{:04}.json", chapter_number)), json)?;
    Ok(())
}

/// 估算文本内容的大致 token 数（中文约 1.5 字符/token，英文约 4 字符/token）
fn estimate_content_tokens(content: &str) -> Result<usize, AppError> {
    // 粗略估算：混合中英文场景，取中间值
    let chars = content.len();
    Ok(chars / 3)
}
