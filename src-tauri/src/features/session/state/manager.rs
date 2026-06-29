use std::path::PathBuf;
use crate::shared::errors::AppError;
use crate::features::story::{StoryState, BookConfig, ChapterMeta};
use super::runtime_state::RuntimeStateDelta;

/// Manages story state on disk with file locking, snapshots, and rollback.
pub struct StateManager {
    project_root: PathBuf,
}

impl StateManager {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    // ── Path helpers ─────────────────────────────────────────────

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

    // ── Book operations ──────────────────────────────────────────

    pub fn create_book(&self, config: &BookConfig) -> Result<(), AppError> {
        let dir = self.book_dir(&config.id);
        std::fs::create_dir_all(dir.join("chapters"))
            .map_err(|e| AppError::internal(format!("Failed to create chapters dir: {}", e)))?;
        std::fs::create_dir_all(dir.join("story/state"))
            .map_err(|e| AppError::internal(format!("Failed to create story dir: {}", e)))?;
        std::fs::create_dir_all(dir.join("story/snapshots"))
            .map_err(|e| AppError::internal(format!("Failed to create snapshots dir: {}", e)))?;
        std::fs::create_dir_all(dir.join("control"))
            .map_err(|e| AppError::internal(format!("Failed to create control dir: {}", e)))?;

        let config_path = dir.join("book.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&config_path, json)
            .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;

        let state = StoryState::default();
        self.save_state(&config.id, &state)?;

        Ok(())
    }

    pub fn load_book_config(&self, book_id: &str) -> Result<BookConfig, AppError> {
        let path = self.book_dir(book_id).join("book.json");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read book config: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse book config: {}", e)))
    }

    // ── State operations ─────────────────────────────────────────

    pub fn load_state(&self, book_id: &str) -> Result<StoryState, AppError> {
        let path = self.state_file(book_id);
        if !path.exists() {
            return Ok(StoryState::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read state: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse state: {}", e)))
    }

    pub fn save_state(&self, book_id: &str, state: &StoryState) -> Result<(), AppError> {
        let path = self.state_file(book_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("Failed to create state dir: {}", e)))?;
        }
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| AppError::internal(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))
    }

    pub fn apply_and_save_delta(
        &self,
        book_id: &str,
        chapter: u32,
        delta: &RuntimeStateDelta,
    ) -> Result<StoryState, AppError> {
        let mut state = self.load_state(book_id)?;
        // Apply hook operations from delta
        for op in &delta.hook_ops {
            match op.op {
                super::runtime_state::HookOpType::Upsert => {
                    if let Some(existing) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        existing.last_advanced_chapter = chapter;
                        existing.updated_at = chrono::Utc::now().to_rfc3339();
                        if let Some(status) = &op.status {
                            use crate::features::story::HookStatus;
                            existing.status = match status.as_str() {
                                "open" | "Open" => HookStatus::Open,
                                "progressing" | "Progressing" => HookStatus::Progressing,
                                "deferred" | "Deferred" => HookStatus::Deferred,
                                "resolved" | "Resolved" => HookStatus::Resolved,
                                _ => existing.status.clone(),
                            };
                        }
                        if let Some(desc) = &op.description {
                            if !desc.is_empty() {
                                existing.expected_payoff = desc.clone();
                            }
                        }
                    } else {
                        let hook_type = op.hook_type.clone().unwrap_or_else(|| "foreshadowing".to_string());
                        state.hooks.push(crate::features::story::HookRecord {
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
                super::runtime_state::HookOpType::Mention => {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        hook.last_advanced_chapter = chapter;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
                super::runtime_state::HookOpType::Resolve => {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        hook.status = crate::features::story::HookStatus::Resolved;
                        hook.last_advanced_chapter = chapter;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
                super::runtime_state::HookOpType::Defer => {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                        hook.status = crate::features::story::HookStatus::Deferred;
                        hook.last_advanced_chapter = chapter;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
            }
        }
        // Add new facts
        for fact in &delta.facts_new {
            state.facts.push(crate::features::story::StoryFact {
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
        // Update chapter summary
        if let Some(summary) = &delta.chapter_summary {
            state.summaries.push(crate::features::story::ChapterSummary {
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
        Ok(state)
    }

    // ── Snapshot operations ──────────────────────────────────────

    pub fn save_snapshot(&self, book_id: &str, chapter: u32, state: &StoryState) -> Result<(), AppError> {
        let dir = self.snapshots_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create snapshots dir: {}", e)))?;
        let path = dir.join(format!("chapter_{:04}.json", chapter));
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| AppError::internal(format!("Failed to serialize snapshot: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| AppError::internal(format!("Failed to write snapshot: {}", e)))
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

        // Remove chapters after the rollback point
        let chapters_dir = self.chapters_dir(book_id);
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(&chapters_dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if let Ok(num) = name.parse::<u32>() {
                        if num > chapter {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }

        // Remove snapshots after the rollback point
        let snapshots_dir = self.snapshots_dir(book_id);
        if snapshots_dir.exists() {
            for entry in std::fs::read_dir(&snapshots_dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if let Some(num_str) = name.strip_prefix("chapter_") {
                        if let Ok(num) = num_str.parse::<u32>() {
                            if num > chapter {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }

        Ok(state)
    }

    // ── Chapter operations ───────────────────────────────────────

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
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?;
        Ok(Some(content))
    }

    // ── Control document operations ──────────────────────────────

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
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read control doc: {}", e)))?;
        Ok(Some(content))
    }

    // ── Artifact operations (for pipeline stage outputs) ─────────

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
        if !path.exists() {
            return Ok(None);
        }
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
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read context: {}", e)))?;
        Ok(Some(content))
    }
}
