//! ═══════════════════════════════════════════════════════════════════════════
//! 故事命令 - IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::validation::{validate_id, validate_path};
use tauri::State;
use std::path::PathBuf;
use tokio::sync::Mutex;
use std::sync::OnceLock;

const MAX_CONTENT_SIZE: usize = 10 * 1024 * 1024;

fn story_state_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn validate_novel_id(novel_id: &str) -> Result<(), AppError> {
    validate_id(novel_id, "novel_id").map_err(AppError::invalid_input)
}

fn build_story_path(workspace_path: &str, novel_id: &str) -> Result<PathBuf, AppError> {
    validate_path(workspace_path).map_err(AppError::invalid_input)?;
    let path = PathBuf::from(workspace_path)
        .join("books")
        .join(novel_id)
        .join("story")
        .join("state.json");
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(AppError::invalid_input("Path traversal denied"));
    }
    Ok(path)
}

#[tauri::command]
pub async fn story_state_get(
    state: State<'_, DbState>,
    novel_id: String,
) -> Result<IpcResponse<crate::domain::story::models::StoryState>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(novel_id = %novel_id, "[story] story_state_get: started");

    validate_novel_id(&novel_id)?;

    let novel = state.db.get_novel_by_id(&novel_id)?
        .ok_or_else(|| {
            tracing::warn!(novel_id = %novel_id, "[story] story_state_get: novel not found");
            AppError::not_found("Novel not found")
        })?;

    let workspace = state.db.get_workspace(&novel.workspace_id)?
        .ok_or_else(|| {
            tracing::warn!(workspace_id = %novel.workspace_id, "[story] story_state_get: workspace not found");
            AppError::not_found("Workspace not found")
        })?;

    let state_path = build_story_path(&workspace.path, &novel_id)?;

    let story_state = if state_path.exists() {
        tracing::debug!(path = %state_path.display(), "[story] story_state_get: loading from file");
        tokio::task::spawn_blocking(move || -> Result<crate::domain::story::models::StoryState, AppError> {
            let raw = std::fs::read_to_string(&state_path)
                .map_err(|e| {
                    tracing::error!(error = %e, "[story] story_state_get: failed to read file");
                    AppError::internal(format!("Failed to read state: {}", e))
                })?;
            serde_json::from_str(&raw)
                .map_err(|e| {
                    tracing::error!(error = %e, "[story] story_state_get: failed to parse JSON");
                    AppError::internal(format!("Failed to parse state: {}", e))
                })
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "[story] story_state_get: spawn_blocking failed");
            AppError::internal(format!("spawn_blocking join failed: {}", e))
        })??
    } else {
        tracing::debug!("[story] story_state_get: using default state");
        crate::domain::story::models::StoryState::default()
    };

    tracing::info!(novel_id = %novel_id, duration_ms = start.elapsed().as_millis() as u64, "[story] story_state_get: completed");
    Ok(IpcResponse::ok(story_state))
}

#[tauri::command]
pub async fn story_state_save(
    state: State<'_, DbState>,
    novel_id: String,
    story_state: crate::domain::story::models::StoryState,
) -> Result<IpcResponse<bool>, AppError> {
    validate_novel_id(&novel_id)?;

    let novel = state.db.get_novel_by_id(&novel_id)?
        .ok_or_else(|| AppError::not_found("Novel not found"))?;

    let workspace = state.db.get_workspace(&novel.workspace_id)?
        .ok_or_else(|| AppError::not_found("Workspace not found"))?;

    let state_path = build_story_path(&workspace.path, &novel_id)?;
    let parent = state_path.parent()
        .ok_or_else(|| AppError::internal("Invalid state path"))?
        .to_path_buf();

    let json = serde_json::to_string_pretty(&story_state)
        .map_err(|e| AppError::internal(format!("Failed to serialize: {}", e)))?;

    if json.len() > MAX_CONTENT_SIZE {
        return Err(AppError::invalid_input("Story state too large (max 10MB)"));
    }

    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        std::fs::create_dir_all(&parent)
            .map_err(|e| AppError::internal(format!("Failed to create directory: {}", e)))?;
        std::fs::write(&state_path, &json)
            .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(true))
}

#[tauri::command]
pub async fn hook_update_status(
    state: State<'_, DbState>,
    novel_id: String,
    hook_id: String,
    new_status: String,
) -> Result<IpcResponse<crate::domain::story::models::StoryState>, AppError> {
    validate_novel_id(&novel_id)?;
    validate_id(&hook_id, "hook_id").map_err(AppError::invalid_input)?;

    let status = match new_status.as_str() {
        "open" => crate::domain::story::models::HookStatus::Open,
        "progressing" => crate::domain::story::models::HookStatus::Progressing,
        "deferred" => crate::domain::story::models::HookStatus::Deferred,
        "resolved" => crate::domain::story::models::HookStatus::Resolved,
        other => return Err(AppError::invalid_input(format!(
            "Invalid hook status '{}': expected one of open/progressing/deferred/resolved", other
        ))),
    };

    let novel = state.db.get_novel_by_id(&novel_id)?
        .ok_or_else(|| AppError::not_found("Novel not found"))?;

    let workspace = state.db.get_workspace(&novel.workspace_id)?
        .ok_or_else(|| AppError::not_found("Workspace not found"))?;

    let state_path = build_story_path(&workspace.path, &novel_id)?;

    let _guard = story_state_lock().lock().await;

    let read_path = state_path.clone();
    let mut story_state: crate::domain::story::models::StoryState = if state_path.exists() {
        tokio::task::spawn_blocking(move || -> Result<crate::domain::story::models::StoryState, AppError> {
            let raw = std::fs::read_to_string(&read_path)
                .map_err(|e| AppError::internal(format!("Failed to read state: {}", e)))?;
            serde_json::from_str(&raw)
                .map_err(|e| AppError::internal(format!("Failed to parse state: {}", e)))
        })
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??
    } else {
        crate::domain::story::models::StoryState::default()
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut found = false;
    for hook in &mut story_state.hooks {
        if hook.hook_id == hook_id {
            hook.status = status.clone();
            hook.updated_at = now.clone();
            found = true;
            break;
        }
    }
    if !found {
        return Err(AppError::not_found(format!(
            "Hook '{}' not found in novel '{}'", hook_id, novel_id
        )));
    }

    let state_json = serde_json::to_string_pretty(&story_state)
        .map_err(|e| AppError::internal(format!("Failed to serialize state: {}", e)))?;

    let write_path = state_path.clone();
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        std::fs::write(&write_path, &state_json)
            .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(story_state))
}

#[tauri::command]
pub async fn query_facts_at_chapter(
    state: State<'_, DbState>,
    novel_id: String,
    chapter: u32,
) -> Result<IpcResponse<Vec<crate::domain::story::models::StoryFact>>, AppError> {
    validate_novel_id(&novel_id)?;
    let facts = state.db.query_facts_at_chapter(&novel_id, chapter)?;
    Ok(IpcResponse::ok(facts))
}

#[tauri::command]
pub async fn list_recent_chapter_summaries(
    state: State<'_, DbState>,
    novel_id: String,
    before_chapter: u32,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::domain::story::models::ChapterSummary>>, AppError> {
    validate_novel_id(&novel_id)?;
    let limit = limit.unwrap_or(5).clamp(1, 50);
    let summaries = state.db.list_recent_chapter_summaries(&novel_id, before_chapter, limit)?;
    Ok(IpcResponse::ok(summaries))
}

#[tauri::command]
pub async fn list_chapter_summaries_range(
    state: State<'_, DbState>,
    novel_id: String,
    from_chapter: u32,
    to_chapter: u32,
) -> Result<IpcResponse<Vec<crate::domain::story::models::ChapterSummary>>, AppError> {
    validate_novel_id(&novel_id)?;
    if from_chapter > to_chapter {
        return Err(AppError::invalid_input(format!(
            "from_chapter ({}) must be <= to_chapter ({})", from_chapter, to_chapter
        )));
    }
    if to_chapter - from_chapter > 200 {
        return Err(AppError::invalid_input(
            "Chapter range too wide (max 200 chapters per query)".to_string()
        ));
    }
    let summaries = state.db.list_chapter_summaries_range(&novel_id, from_chapter, to_chapter)?;
    Ok(IpcResponse::ok(summaries))
}