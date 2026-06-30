use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use crate::shared::errors::AppError;
use super::runner::PipelineRunner;
use super::state_graph::{GraphState, CheckpointStore};
use crate::infrastructure::state_store::memory::MemoryStore;
use crate::infrastructure::state_store::feedback::{FeedbackStore, ErrorEvent, Severity};
use crate::infrastructure::ai_services::rag::VectorStore;
use crate::infrastructure::state_store::gc::SnapshotGc;

/// Configuration for all scheduled tasks.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub write_interval_secs: u64,
    pub gc_interval_secs: u64,
    pub feedback_check_interval_secs: u64,
    pub max_concurrent_books: usize,
    pub chapters_per_cycle: u32,
    pub snapshot_gc: SnapshotGcConfig,
    pub context_window: u32,
}

#[derive(Debug, Clone)]
pub struct SnapshotGcConfig {
    pub max_snapshots: usize,
    pub max_age_days: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            write_interval_secs: 3600,
            gc_interval_secs: 43200, // 12 hours
            feedback_check_interval_secs: 600, // 10 minutes
            max_concurrent_books: 1,
            chapters_per_cycle: 1,
            snapshot_gc: SnapshotGcConfig { max_snapshots: 100, max_age_days: 90 },
            context_window: 128_000,
        }
    }
}

/// State of the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerStatus {
    Idle,
    Running,
    Paused,
    Error(String),
}

/// The Scheduler orchestrates all modules:
/// - Write cycles (full pipeline)
/// - Memory archiving (after observe)
/// - Feedback loop (after audit)
/// - Snapshot GC (periodic)
/// - RAG indexing (after chapter write)
/// - State graph checkpoints (periodic)
pub struct Scheduler {
    pipeline: PipelineRunner,
    config: SchedulerConfig,
    status: Arc<RwLock<SchedulerStatus>>,
    // Shared stores
    memory_store: Arc<MemoryStore>,
    feedback_store: Arc<Mutex<FeedbackStore>>,
    rag_store: Arc<Mutex<VectorStore>>,
    checkpoint_store: Arc<Mutex<CheckpointStore>>,
    // Control
    cancel_flag: Arc<RwLock<bool>>,
}

impl Scheduler {
    pub fn new(
        pipeline: PipelineRunner,
        config: SchedulerConfig,
        memory_store: Arc<MemoryStore>,
        feedback_store: FeedbackStore,
    ) -> Self {
        Self {
            pipeline,
            config,
            status: Arc::new(RwLock::new(SchedulerStatus::Idle)),
            memory_store,
            feedback_store: Arc::new(Mutex::new(feedback_store)),
            rag_store: Arc::new(Mutex::new(VectorStore::new())),
            checkpoint_store: Arc::new(Mutex::new(CheckpointStore::new())),
            cancel_flag: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the scheduler with periodic task loops.
    pub async fn start(&self) -> Result<(), AppError> {
        let mut status = self.status.write().await;
        if *status == SchedulerStatus::Running {
            return Ok(());
        }
        *status = SchedulerStatus::Running;
        drop(status);

        tracing::info!("Scheduler started");

        // Spawn background loops
        self.spawn_gc_loop().await;
        self.spawn_feedback_loop().await;
        self.spawn_checkpoint_loop().await;

        Ok(())
    }

    pub async fn stop(&self) {
        *self.cancel_flag.write().await = true;
        *self.status.write().await = SchedulerStatus::Idle;
        tracing::info!("Scheduler stopped");
    }

    pub async fn pause(&self) {
        *self.status.write().await = SchedulerStatus::Paused;
        tracing::info!("Scheduler paused");
    }

    pub async fn resume(&self) {
        *self.status.write().await = SchedulerStatus::Running;
        tracing::info!("Scheduler resumed");
    }

    pub async fn status(&self) -> SchedulerStatus {
        self.status.read().await.clone()
    }

    // ── Write cycle execution ──────────────────────────────────

    /// Execute a full write cycle for a book.
    /// Pipeline: write → observe → archive memory → index RAG → feedback
    pub async fn execute_write_cycle(&self, book_id: &str) -> Result<WriteCycleResult, AppError> {
        tracing::info!(book_id, "Starting write cycle");
        let start = std::time::Instant::now();

        // 1. Run the full pipeline (plan → compose → write → audit → revise)
        let write_result = self.pipeline.write_next_chapter(book_id, None).await?;
        let chapter = write_result.chapter_number;

        // 2. Observe: extract facts from the chapter
        let observation = self.run_observe(book_id, chapter).await?;

        // 3. Archive to memory
        self.archive_observation(book_id, chapter, &observation).await;

        // 4. Index to RAG
        self.index_chapter_to_rag(book_id, chapter, &write_result.content).await;

        // 5. Record feedback from audit
        self.record_audit_feedback(book_id, chapter, &write_result).await;

        // 6. Save state checkpoint
        self.save_state_checkpoint(book_id, chapter).await;

        // 7. GC old snapshots
        self.gc_snapshots(book_id).await;

        let elapsed = start.elapsed().as_secs();
        tracing::info!(
            book_id,
            chapter,
            word_count = write_result.word_count,
            elapsed_secs = elapsed,
            "Write cycle completed"
        );

        Ok(WriteCycleResult {
            chapter,
            word_count: write_result.word_count,
            audit_passed: write_result.audit.passed,
            elapsed_secs: elapsed,
        })
    }

    // ── Observe & Archive ──────────────────────────────────────

    /// Run observation on a chapter and archive to memory.
    async fn run_observe(&self, book_id: &str, chapter: u32) -> Result<serde_json::Value, AppError> {
        let book_dir = self.pipeline.config.project_root.join("books").join(book_id);
        let chapters_dir = book_dir.join("chapters");
        let prefix = format!("{:04}_", chapter);

        let mut content = String::new();
        let mut title = String::new();
        if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(&prefix) {
                    content = std::fs::read_to_string(entry.path())
                        .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?;
                    title = content.lines().next()
                        .and_then(|l| l.strip_prefix("# ").or_else(|| l.strip_prefix("## ")))
                        .unwrap_or("").to_string();
                    break;
                }
            }
        }

        if content.is_empty() {
            return Ok(serde_json::json!({ "chapter": chapter, "facts": [], "hooks_new": [] }));
        }

        let observer = crate::core::agent::observer::ObserverAgent::new();
        let ctx = self.pipeline.agent_ctx_for("observer", Some(book_id)).await;
        let language = "zh";

        match observer.observe_chapter(&ctx, chapter, &title, &content, language, &self.pipeline.config.data_dir).await {
            Ok(output) => {
                let facts_json: Vec<serde_json::Value> = output.facts.iter().map(|f| {
                    serde_json::json!({ "subject": f.subject, "predicate": f.predicate, "object": f.object, "category": f.category })
                }).collect();
                let hooks_json: Vec<serde_json::Value> = output.hooks_new.iter().chain(output.hooks_advanced.iter()).map(|h| {
                    serde_json::json!({ "name": h.name, "type": h.hook_type, "status": h.status, "description": h.description })
                }).collect();
                Ok(serde_json::json!({ "chapter": chapter, "facts": facts_json, "hooks_new": hooks_json }))
            }
            Err(e) => {
                tracing::warn!(error = %e, chapter, "Observation failed");
                Ok(serde_json::json!({ "chapter": chapter, "facts": [], "hooks_new": [] }))
            }
        }
    }

    /// Archive observation data into the shared memory store.
    async fn archive_observation(&self, book_id: &str, chapter: u32, observation: &serde_json::Value) {
        let facts = observation.get("facts").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let hooks = observation.get("hooks_new").and_then(|v| v.as_array()).cloned().unwrap_or_default();

        for fact in &facts {
            let subject = fact.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            let predicate = fact.get("predicate").and_then(|v| v.as_str()).unwrap_or("");
            let object = fact.get("object").and_then(|v| v.as_str()).unwrap_or("");
            let category = fact.get("category").and_then(|v| v.as_str()).unwrap_or("other");
            self.memory_store.archive_fact(book_id, chapter, subject, predicate, object, category).await;
        }

        for hook in &hooks {
            let name = hook.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let hook_type = hook.get("type").and_then(|v| v.as_str()).unwrap_or("foreshadowing");
            let status = hook.get("status").and_then(|v| v.as_str()).unwrap_or("Open");
            let description = hook.get("description").and_then(|v| v.as_str()).unwrap_or("");
            self.memory_store.archive_hook(book_id, chapter, name, hook_type, status, description).await;
        }

        tracing::info!(book_id, chapter, facts = facts.len(), hooks = hooks.len(), "Archived to memory");
    }

    // ── RAG Indexing ───────────────────────────────────────────

    async fn index_chapter_to_rag(&self, book_id: &str, chapter: u32, content: &str) {
        let source = format!("{}/chapter_{:04}", book_id, chapter);
        let mut rag = self.rag_store.lock().await;
        rag.index_document(content, &source, &crate::infrastructure::ai_services::rag::ChunkConfig::default());
        tracing::info!(book_id, chapter, total_chunks = rag.count(), "Indexed to RAG");
    }

    // ── Feedback Loop ──────────────────────────────────────────

    /// Record feedback events based on audit results.
    async fn record_audit_feedback(&self, book_id: &str, chapter: u32, write_result: &crate::features::story::WriteResult) {
        let mut feedback = self.feedback_store.lock().await;

        // Record issues from audit as error events
        for issue in &write_result.audit.issues {
            let severity = match issue.severity {
                crate::features::story::AuditSeverity::Critical => Severity::Critical,
                crate::features::story::AuditSeverity::Warning => Severity::Warning,
                crate::features::story::AuditSeverity::Info => Severity::Warning,
            };

            feedback.record_event(ErrorEvent {
                id: uuid::Uuid::new_v4().to_string(),
                agent: "auditor".to_string(),
                error_type: issue.category.clone(),
                message: issue.description.clone(),
                chapter: Some(chapter),
                book_id: Some(book_id.to_string()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                severity,
            });
        }

        // Prune old events
        feedback.prune_events(500);

        let lessons = feedback.active_lessons().len();
        tracing::info!(book_id, chapter, lessons, "Feedback recorded");
    }

    // ── State Checkpoints ──────────────────────────────────────

    async fn save_state_checkpoint(&self, book_id: &str, chapter: u32) {
        let mut state = GraphState::default();
        state.set("book_id", serde_json::json!(book_id));
        state.set("chapter", serde_json::json!(chapter));
        state.set("timestamp", serde_json::json!(chrono::Utc::now().to_rfc3339()));

        let checkpoint_id = format!("{}_ch{}", book_id, chapter);
        let mut store = self.checkpoint_store.lock().await;
        if let Err(e) = store.save(&checkpoint_id, &state) {
            tracing::warn!(error = %e, "Failed to save checkpoint");
        } else {
            tracing::debug!(checkpoint_id, "State checkpoint saved");
        }
    }

    /// Restore from the latest checkpoint for a book.
    pub async fn restore_checkpoint(&self, book_id: &str) -> Result<Option<GraphState>, AppError> {
        let store = self.checkpoint_store.lock().await;
        let checkpoints = store.list();
        let latest = checkpoints.iter()
            .filter(|c| c.starts_with(book_id))
            .max_by_key(|c| c.as_str())
            .cloned();

        match latest {
            Some(id) => Ok(store.load(&id)?),
            None => Ok(None),
        }
    }

    // ── Snapshot GC ────────────────────────────────────────────

    async fn gc_snapshots(&self, book_id: &str) {
        let snapshots_dir = self.pipeline.config.project_root
            .join("books").join(book_id).join("story").join("snapshots");

        let gc = SnapshotGc::new(
            self.config.snapshot_gc.max_snapshots,
            self.config.snapshot_gc.max_age_days,
        );

        match gc.run(&snapshots_dir) {
            Ok(removed) if removed > 0 => {
                tracing::info!(book_id, removed, "Cleaned up old snapshots");
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = %e, "Snapshot GC failed");
            }
        }
    }

    // ── Background Loops ───────────────────────────────────────

    async fn spawn_gc_loop(&self) {
        let cancel = self.cancel_flag.clone();
        let status = self.status.clone();
        let pipeline_project_root = self.pipeline.config.project_root.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(config.gc_interval_secs)
            );
            loop {
                interval.tick().await;
                if *cancel.read().await || *status.read().await != SchedulerStatus::Running {
                    continue;
                }

                // GC all books
                let books_dir = pipeline_project_root.join("books");
                if let Ok(entries) = std::fs::read_dir(&books_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let snapshots_dir = entry.path().join("story").join("snapshots");
                            let gc = SnapshotGc::new(
                                config.snapshot_gc.max_snapshots,
                                config.snapshot_gc.max_age_days,
                            );
                            let _ = gc.run(&snapshots_dir);
                        }
                    }
                }
                tracing::debug!("Snapshot GC completed");
            }
        });
    }

    async fn spawn_feedback_loop(&self) {
        let cancel = self.cancel_flag.clone();
        let status = self.status.clone();
        let feedback_store = self.feedback_store.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(600) // 10 minutes
            );
            loop {
                interval.tick().await;
                if *cancel.read().await || *status.read().await != SchedulerStatus::Running {
                    continue;
                }

                let feedback = feedback_store.lock().await;
                let lessons = feedback.active_lessons();
                if !lessons.is_empty() {
                    tracing::info!(count = lessons.len(), "Active feedback lessons");
                    for lesson in &lessons {
                        tracing::debug!(rule = %lesson.rule, "Lesson: {}", lesson.reason);
                    }
                }
            }
        });
    }

    async fn spawn_checkpoint_loop(&self) {
        let cancel = self.cancel_flag.clone();
        let status = self.status.clone();
        let checkpoint_store = self.checkpoint_store.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(300) // 5 minutes
            );
            loop {
                interval.tick().await;
                if *cancel.read().await || *status.read().await != SchedulerStatus::Running {
                    continue;
                }

                let store = checkpoint_store.lock().await;
                let count = store.list().len();
                tracing::debug!(checkpoints = count, "Checkpoint store status");
            }
        });
    }

    // ── Accessors for external modules ─────────────────────────

    pub fn memory_store(&self) -> &Arc<MemoryStore> {
        &self.memory_store
    }

    pub fn feedback_store(&self) -> &Arc<Mutex<FeedbackStore>> {
        &self.feedback_store
    }

    pub fn rag_store(&self) -> &Arc<Mutex<VectorStore>> {
        &self.rag_store
    }

    /// Get feedback lessons formatted for prompt injection.
    pub async fn get_lessons_for_prompt(&self) -> String {
        let feedback = self.feedback_store.lock().await;
        feedback.format_lessons_for_prompt()
    }

    /// Search RAG for relevant content.
    pub async fn search_rag(&self, query: &str, top_k: usize) -> Vec<crate::infrastructure::ai_services::rag::SearchResult> {
        let rag = self.rag_store.lock().await;
        rag.search_hybrid(query, top_k)
    }

    /// Search memory for relevant facts.
    pub async fn search_memory(&self, book_id: &str, query: &str, top_k: usize) -> Vec<crate::core::agent::base::MemoryEntry> {
        self.memory_store.search(book_id, query, top_k).await
    }
}

/// Result of a write cycle execution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WriteCycleResult {
    pub chapter: u32,
    pub word_count: u32,
    pub audit_passed: bool,
    pub elapsed_secs: u64,
}
