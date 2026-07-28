
//! ═══════════════════════════════════════════════════════════════════════════
//! State - 运行时状态管理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供书籍运行时状态的管理功能，包括：
//! - 钩子状态管理
//! - 事实状态管理
//! - 章节摘要管理
//! - 状态快照保存与回滚

use std::path::PathBuf;
use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;
use crate::domain::story::models::{StoryState, BookConfig, ChapterMeta, HookStatus, HookRecord, StoryFact, ChapterSummary};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeStateDelta {
    pub chapter: u32,
    pub hook_ops: Vec<HookOp>,
    pub facts_new: Vec<NewFact>,
    pub chapter_summary: Option<ChapterSummaryDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOp {
    pub op: HookOpType,
    pub name: String,
    pub hook_type: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOpType {
    Upsert,
    Mention,
    Resolve,
    Defer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummaryDelta {
    pub chapter: u32,
    pub title: String,
    pub characters: Vec<String>,
    pub events: Vec<String>,
    pub state_changes: Vec<String>,
    pub hook_activity: Vec<String>,
    pub mood: String,
    pub chapter_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStateSnapshot {
    pub chapter: u32,
    pub timestamp: String,
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
    pub hooks_state: HooksState,
    pub facts_state: FactsState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HooksState {
    pub hooks: Vec<HooksStateHook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksStateHook {
    pub hook_id: String,
    pub name: String,
    pub status: String,
    pub start_chapter: u32,
    pub last_advanced_chapter: u32,
    pub expected_payoff: String,
    pub core_hook: bool,
    pub promoted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactsState {
    pub facts: Vec<FactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactEntry {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: String,
    pub valid_from_chapter: u32,
}

pub fn apply_delta_to_snapshot(
    snapshot: &RuntimeStateSnapshot,
    delta: &RuntimeStateDelta,
) -> RuntimeStateSnapshot {
    let mut new_snapshot = snapshot.clone();
    new_snapshot.chapter = delta.chapter;
    new_snapshot.timestamp = chrono::Utc::now().to_rfc3339();

    for op in &delta.hook_ops {
        match op.op {
            HookOpType::Upsert => {
                if let Some(existing) = new_snapshot.hooks_state.hooks.iter_mut().find(|h| h.name == op.name) {
                    existing.last_advanced_chapter = delta.chapter;
                    if let Some(status) = &op.status { existing.status = status.clone(); }
                    if let Some(desc) = &op.description { if !desc.is_empty() { existing.expected_payoff = desc.clone(); } }
                } else {
                    new_snapshot.hooks_state.hooks.push(HooksStateHook {
                        hook_id: uuid::Uuid::new_v4().to_string(),
                        name: op.name.clone(),
                        status: op.status.clone().unwrap_or_else(|| "open".to_string()),
                        start_chapter: delta.chapter,
                        last_advanced_chapter: delta.chapter,
                        expected_payoff: op.description.clone().unwrap_or_default(),
                        core_hook: false,
                        promoted: false,
                    });
                }
            }
            HookOpType::Mention => {
                if let Some(hook) = new_snapshot.hooks_state.hooks.iter_mut().find(|h| h.name == op.name) {
                    hook.last_advanced_chapter = delta.chapter;
                }
            }
            HookOpType::Resolve => {
                if let Some(hook) = new_snapshot.hooks_state.hooks.iter_mut().find(|h| h.name == op.name) {
                    hook.status = "resolved".to_string();
                    hook.last_advanced_chapter = delta.chapter;
                }
            }
            HookOpType::Defer => {
                if let Some(hook) = new_snapshot.hooks_state.hooks.iter_mut().find(|h| h.name == op.name) {
                    hook.status = "deferred".to_string();
                    hook.last_advanced_chapter = delta.chapter;
                }
            }
        }
    }

    for fact in &delta.facts_new {
        new_snapshot.facts_state.facts.push(FactEntry {
            subject: fact.subject.clone(),
            predicate: fact.predicate.clone(),
            object: fact.object.clone(),
            category: fact.category.clone(),
            valid_from_chapter: delta.chapter,
        });
    }

    new_snapshot
}

pub struct StateManager {
    project_root: PathBuf,
}

impl StateManager {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    fn book_dir(&self, book_id: &str) -> PathBuf {
        self.project_root.join("books").join(book_id)
    }

    fn story_dir(&self, book_id: &str) -> PathBuf {
        self.book_dir(book_id).join("story")
    }

    fn state_file(&self, book_id: &str) -> PathBuf {
        self.story_dir(book_id).join("state.json")
    }

    fn chapters_dir(&self, book_id: &str) -> PathBuf {
        self.book_dir(book_id).join("chapters")
    }

    fn snapshots_dir(&self, book_id: &str) -> PathBuf {
        self.story_dir(book_id).join("snapshots")
    }

    fn control_dir(&self, book_id: &str) -> PathBuf {
        self.book_dir(book_id).join("control")
    }

    pub fn create_book(&self, config: &BookConfig) -> Result<(), AppError> {
        let start = Instant::now();
        tracing::info!(book_id = %config.id, "Creating book");

        let dir = self.book_dir(&config.id);
        std::fs::create_dir_all(dir.join("chapters"))
            .map_err(|e| {
                tracing::error!(book_id = %config.id, error = %e, "Failed to create chapters dir");
                AppError::internal(format!("Failed to create chapters dir: {}", e))
            })?;
        std::fs::create_dir_all(dir.join("story/state"))
            .map_err(|e| {
                tracing::error!(book_id = %config.id, error = %e, "Failed to create story dir");
                AppError::internal(format!("Failed to create story dir: {}", e))
            })?;
        std::fs::create_dir_all(dir.join("story/snapshots"))
            .map_err(|e| {
                tracing::error!(book_id = %config.id, error = %e, "Failed to create snapshots dir");
                AppError::internal(format!("Failed to create snapshots dir: {}", e))
            })?;
        std::fs::create_dir_all(dir.join("control"))
            .map_err(|e| {
                tracing::error!(book_id = %config.id, error = %e, "Failed to create control dir");
                AppError::internal(format!("Failed to create control dir: {}", e))
            })?;

        let config_path = dir.join("book.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| {
                tracing::error!(book_id = %config.id, error = %e, "Failed to serialize config");
                AppError::internal(format!("Failed to serialize config: {}", e))
            })?;
        std::fs::write(&config_path, json)
            .map_err(|e| {
                tracing::error!(book_id = %config.id, path = %config_path.display(), error = %e, "Failed to write config");
                AppError::internal(format!("Failed to write config: {}", e))
            })?;

        let state = StoryState::default();
        self.save_state(&config.id, &state)?;

        tracing::info!(
            book_id = %config.id,
            duration_ms = start.elapsed().as_millis() as u64,
            "Book created"
        );
        Ok(())
    }

    pub fn load_book_config(&self, book_id: &str) -> Result<BookConfig, AppError> {
        tracing::debug!(book_id = %book_id, "Loading book config");

        let path = self.book_dir(book_id).join("book.json");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, path = %path.display(), error = %e, "Failed to read book config");
                AppError::internal(format!("Failed to read book config: {}", e))
            })?;
        serde_json::from_str(&content)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, error = %e, "Failed to parse book config");
                AppError::internal(format!("Failed to parse book config: {}", e))
            })
    }

    pub fn load_state(&self, book_id: &str) -> Result<StoryState, AppError> {
        tracing::debug!(book_id = %book_id, "Loading story state");

        let path = self.state_file(book_id);
        if !path.exists() {
            tracing::debug!(book_id = %book_id, "State file not found, returning default");
            return Ok(StoryState::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, path = %path.display(), error = %e, "Failed to read state");
                AppError::internal(format!("Failed to read state: {}", e))
            })?;
        serde_json::from_str(&content)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, error = %e, "Failed to parse state");
                AppError::internal(format!("Failed to parse state: {}", e))
            })
    }

    pub fn save_state(&self, book_id: &str, state: &StoryState) -> Result<(), AppError> {
        tracing::debug!(book_id = %book_id, "Saving story state");

        let path = self.state_file(book_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| {
                    tracing::error!(book_id = %book_id, path = %path.display(), error = %e, "Failed to create state dir");
                    AppError::internal(format!("Failed to create state dir: {}", e))
                })?;
        }
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, error = %e, "Failed to serialize state");
                AppError::internal(format!("Failed to serialize state: {}", e))
            })?;
        std::fs::write(&path, json)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, path = %path.display(), error = %e, "Failed to write state");
                AppError::internal(format!("Failed to write state: {}", e))
            })
    }

    pub fn apply_and_save_delta(
        &self,
        book_id: &str,
        chapter: u32,
        delta: &RuntimeStateDelta,
    ) -> Result<StoryState, AppError> {
        let start = Instant::now();
        tracing::debug!(book_id = %book_id, chapter = chapter, "Applying delta");

        let mut state = self.load_state(book_id)?;
        for op in &delta.hook_ops {
            match op.op {
                HookOpType::Upsert => {
                    if let Some(existing) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        existing.last_advanced_chapter = chapter;
                        existing.updated_at = chrono::Utc::now().to_rfc3339();
                        if let Some(status) = &op.status {
                            existing.status = match status.as_str() {
                                "open" | "Open" => HookStatus::Open,
                                "progressing" | "Progressing" => HookStatus::Progressing,
                                "deferred" | "Deferred" => HookStatus::Deferred,
                                "resolved" | "Resolved" => HookStatus::Resolved,
                                _ => existing.status.clone(),
                            };
                        }
                        if let Some(desc) = &op.description { if !desc.is_empty() { existing.expected_payoff = desc.clone(); } }
                    } else {
                        let hook_type = op.hook_type.clone().unwrap_or_else(|| "foreshadowing".to_string());
                        state.hooks.push(HookRecord {
                            hook_id: uuid::Uuid::new_v4().to_string(),
                            name: op.name.clone(),
                            hook_type,
                            start_chapter: chapter,
                            status: Default::default(),
                            expected_payoff: op.description.clone().unwrap_or_default(),
                            last_advanced_chapter: chapter,
                            core_hook: false,
                            created_at: chrono::Utc::now().to_rfc3339(),
                            updated_at: chrono::Utc::now().to_rfc3339(),
                        });
                    }
                }
                HookOpType::Mention => {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        hook.last_advanced_chapter = chapter;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
                HookOpType::Resolve => {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        hook.status = HookStatus::Resolved;
                        hook.last_advanced_chapter = chapter;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
                HookOpType::Defer => {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        hook.status = HookStatus::Deferred;
                        hook.last_advanced_chapter = chapter;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
            }
        }
        for fact in &delta.facts_new {
            state.facts.push(StoryFact {
                fact_id: uuid::Uuid::new_v4().to_string(),
                subject: fact.subject.clone(),
                predicate: fact.predicate.clone(),
                object: fact.object.clone(),
                valid_from_chapter: chapter,
                valid_until_chapter: None,
                source_chapter: chapter,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        if let Some(summary) = &delta.chapter_summary {
            state.summaries.push(ChapterSummary {
                chapter: summary.chapter,
                title: summary.title.clone(),
                characters: summary.characters.clone(),
                events: summary.events.clone(),
                state_changes: summary.state_changes.clone(),
                hook_activity: summary.hook_activity.clone(),
                mood: summary.mood.clone(),
                chapter_type: summary.chapter_type.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        state.current_chapter = chapter;
        self.save_state(book_id, &state)?;

        tracing::debug!(
            book_id = %book_id,
            chapter = chapter,
            duration_ms = start.elapsed().as_millis() as u64,
            "Delta applied"
        );
        Ok(state)
    }

    pub fn save_snapshot(&self, book_id: &str, chapter: u32, state: &StoryState) -> Result<(), AppError> {
        tracing::debug!(book_id = %book_id, chapter = chapter, "Saving snapshot");

        let dir = self.snapshots_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, path = %dir.display(), error = %e, "Failed to create snapshots dir");
                AppError::internal(format!("Failed to create snapshots dir: {}", e))
            })?;
        let path = dir.join(format!("chapter_{:04}.json", chapter));
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, chapter = chapter, error = %e, "Failed to serialize snapshot");
                AppError::internal(format!("Failed to serialize snapshot: {}", e))
            })?;
        std::fs::write(&path, json)
            .map_err(|e| {
                tracing::error!(book_id = %book_id, path = %path.display(), error = %e, "Failed to write snapshot");
                AppError::internal(format!("Failed to write snapshot: {}", e))
            })
    }

    pub fn rollback_to_chapter(&self, book_id: &str, chapter: u32) -> Result<StoryState, AppError> {
        let snapshot_path = self.snapshots_dir(book_id).join(format!("chapter_{:04}.json", chapter));
        if !snapshot_path.exists() {
            return Err(AppError::not_found(format!("Snapshot for chapter {} not found", chapter)));
        }
        let content = std::fs::read_to_string(&snapshot_path)
            .map_err(|e| AppError::internal(format!("Failed to read snapshot: {}", e)))?;
        let state: StoryState = serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse snapshot: {}", e)))?;
        self.save_state(book_id, &state)?;

        let chapters_dir = self.chapters_dir(book_id);
        if chapters_dir.exists() {
            // 不用 into_iter().flatten().flatten() 静默吞 read_dir/entry 错误：
            // read_dir 失败应显式报错，entry 失败应 log warn（单条损坏不应阻塞回滚）。
            let entries = std::fs::read_dir(&chapters_dir)
                .map_err(|e| AppError::internal(format!("Failed to read chapters dir: {}", e)))?;
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(error = %e, "Skipping malformed chapter entry during rollback");
                        continue;
                    }
                };
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if let Ok(num) = name.parse::<u32>() {
                        if num > chapter {
                            if let Err(e) = std::fs::remove_file(&path) {
                                tracing::warn!(error = %e, path = %path.display(), "Failed to remove chapter file during rollback");
                            }
                        }
                    }
                }
            }
        }

        let snapshots_dir = self.snapshots_dir(book_id);
        if snapshots_dir.exists() {
            let entries = std::fs::read_dir(&snapshots_dir)
                .map_err(|e| AppError::internal(format!("Failed to read snapshots dir: {}", e)))?;
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(error = %e, "Skipping malformed snapshot entry during rollback");
                        continue;
                    }
                };
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if let Some(num_str) = name.strip_prefix("chapter_") {
                        if let Ok(num) = num_str.parse::<u32>() {
                            if num > chapter {
                                if let Err(e) = std::fs::remove_file(&path) {
                                    tracing::warn!(error = %e, path = %path.display(), "Failed to remove snapshot file during rollback");
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(state)
    }

    pub fn save_chapter(&self, book_id: &str, chapter: &ChapterMeta, content: &str) -> Result<(), AppError> {
        let dir = self.chapters_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create chapters dir: {}", e)))?;
        let path = dir.join(format!("chapter_{:04}.md", chapter.number));
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write chapter: {}", e)))
    }

    pub fn load_chapter_content(&self, book_id: &str, chapter: u32) -> Result<Option<String>, AppError> {
        let path = self.chapters_dir(book_id).join(format!("chapter_{:04}.md", chapter));
        if !path.exists() { return Ok(None); }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?;
        Ok(Some(content))
    }

    pub fn save_control_doc(&self, book_id: &str, name: &str, content: &str) -> Result<(), AppError> {
        let dir = self.control_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create control dir: {}", e)))?;
        let path = dir.join(name);
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write control doc: {}", e)))
    }

    pub fn load_control_doc(&self, book_id: &str, name: &str) -> Result<Option<String>, AppError> {
        let path = self.control_dir(book_id).join(name);
        if !path.exists() { return Ok(None); }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read control doc: {}", e)))?;
        Ok(Some(content))
    }

    pub fn save_intent(&self, book_id: &str, chapter: u32, content: &str) -> Result<(), AppError> {
        let dir = self.story_dir(book_id).join("state");
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create state dir: {}", e)))?;
        let path = dir.join(format!("chapter_{:04}_intent.json", chapter));
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write intent: {}", e)))
    }

    pub fn load_intent(&self, book_id: &str, chapter: u32) -> Result<Option<String>, AppError> {
        let path = self.story_dir(book_id).join("state").join(format!("chapter_{:04}_intent.json", chapter));
        if !path.exists() { return Ok(None); }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read intent: {}", e)))?;
        Ok(Some(content))
    }

    pub fn save_context(&self, book_id: &str, chapter: u32, content: &str) -> Result<(), AppError> {
        let dir = self.story_dir(book_id).join("state");
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create state dir: {}", e)))?;
        let path = dir.join(format!("chapter_{:04}_context.json", chapter));
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write context: {}", e)))
    }

    pub fn load_context(&self, book_id: &str, chapter: u32) -> Result<Option<String>, AppError> {
        let path = self.story_dir(book_id).join("state").join(format!("chapter_{:04}_context.json", chapter));
        if !path.exists() { return Ok(None); }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read context: {}", e)))?;
        Ok(Some(content))
    }
}