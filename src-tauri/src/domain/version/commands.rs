//! ═══════════════════════════════════════════════════════════════════════════
//! 版本命令 - IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use crate::shared::version::types::{ChapterVersion, LineDiffResult};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::validation::validate_id;
use sha2::Digest;
use tauri::State;

const MAX_CONTENT_SIZE: usize = 10_000_000;

fn validate_novel_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "novel_id").map_err(AppError::invalid_input)
}

fn validate_version_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "version_id").map_err(AppError::invalid_input)
}

fn parse_revision_mode(mode: Option<&str>) -> Result<crate::domain::version::types::RevisionMode, AppError> {
    match mode {
        Some("minor") => Ok(crate::domain::version::types::RevisionMode::Minor),
        Some("major") => Ok(crate::domain::version::types::RevisionMode::Major),
        Some("rewrite") => Ok(crate::domain::version::types::RevisionMode::Rewrite),
        Some("auto") | None => Ok(crate::domain::version::types::RevisionMode::Auto),
        Some(other) => Err(AppError::invalid_input(format!("Invalid revision mode: {}", other))),
    }
}

#[tauri::command]
pub async fn version_list(
    state: State<'_, DbState>,
    novel_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<Vec<crate::domain::version::types::ChapterVersion>>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(novel_id = %novel_id, chapter_number, "[version] version_list: started");

    validate_novel_id(&novel_id)?;
    let versions = state.db.list_chapter_versions(&novel_id, chapter_number)?;

    tracing::info!(novel_id = %novel_id, chapter_number, count = versions.len(), duration_ms = start.elapsed().as_millis() as u64, "[version] version_list: completed");
    Ok(IpcResponse::ok(versions))
}

#[tauri::command]
pub async fn version_get(
    state: State<'_, DbState>,
    version_id: String,
) -> Result<IpcResponse<crate::domain::version::types::ChapterVersion>, AppError> {
    validate_version_id(&version_id)?;
    let version = state.db.get_chapter_version(&version_id)?
        .ok_or_else(|| AppError::not_found("Version not found"))?;
    Ok(IpcResponse::ok(version))
}

#[tauri::command]
pub async fn version_get_latest(
    state: State<'_, DbState>,
    novel_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<crate::domain::version::types::ChapterVersion>, AppError> {
    validate_novel_id(&novel_id)?;
    let version = state.db.get_latest_chapter_version(&novel_id, chapter_number)?
        .ok_or_else(|| AppError::not_found("No version found"))?;
    Ok(IpcResponse::ok(version))
}

#[tauri::command]
pub async fn version_save(
    state: State<'_, DbState>,
    novel_id: String,
    chapter_number: u32,
    content: String,
    revision_reason: Option<String>,
    revision_mode: Option<String>,
) -> Result<IpcResponse<crate::domain::version::types::ChapterVersion>, AppError> {
    validate_novel_id(&novel_id)?;

    if content.len() > MAX_CONTENT_SIZE {
        return Err(AppError::invalid_input(format!(
            "Content too large (max {} bytes)", MAX_CONTENT_SIZE
        )));
    }

    let mode = parse_revision_mode(revision_mode.as_deref())?;

    let version_number = state.db.get_next_version_number(&novel_id, chapter_number)?;
    let content_hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
    let word_count = crate::shared::story::types::count_words_default(&content);

    let req = crate::domain::version::types::CreateVersionRequest {
        novel_id,
        chapter_number,
        content,
        content_hash,
        word_count,
        revision_mode: mode,
        revision_reason: revision_reason.unwrap_or_default(),
    };

    let version = state.db.create_chapter_version(&req, version_number, &req.content_hash, req.word_count)?;
    Ok(IpcResponse::created(version))
}

#[tauri::command]
pub async fn version_diff(
    state: State<'_, DbState>,
    from_version_id: String,
    to_version_id: String,
) -> Result<IpcResponse<LineDiffResult>, AppError> {
    validate_id(&from_version_id, "from_version_id").map_err(AppError::invalid_input)?;
    validate_id(&to_version_id, "to_version_id").map_err(AppError::invalid_input)?;
    let from = state.db.get_chapter_version(&from_version_id)?
        .ok_or_else(|| AppError::not_found("From version not found"))?;
    let to = state.db.get_chapter_version(&to_version_id)?
        .ok_or_else(|| AppError::not_found("To version not found"))?;
    let diff = crate::domain::version::diff::compute_line_diff(&from.content, &to.content)?;
    Ok(IpcResponse::ok(diff))
}

#[tauri::command]
pub async fn version_diff_latest(
    state: State<'_, DbState>,
    novel_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<Option<LineDiffResult>>, AppError> {
    validate_novel_id(&novel_id)?;
    let mut versions = state.db.list_chapter_versions(&novel_id, chapter_number)?;
    if versions.len() < 2 {
        return Ok(IpcResponse::ok(None));
    }
    let to = versions.swap_remove(0);
    let from = versions.swap_remove(0);
    let diff = crate::domain::version::diff::compute_line_diff(&from.content, &to.content)?;
    Ok(IpcResponse::ok(Some(diff)))
}

#[tauri::command]
pub async fn version_restore(
    state: State<'_, DbState>,
    version_id: String,
    _workspace_id: String,
    _book_id: String,
) -> Result<IpcResponse<ChapterVersion>, AppError> {
    validate_id(&version_id, "version_id").map_err(AppError::invalid_input)?;
    let origin = state.db.get_chapter_version(&version_id)?
        .ok_or_else(|| AppError::not_found("Version not found"))?;
    let version_number = state.db.get_next_version_number(&origin.novel_id, origin.chapter_number)?;
    let new_version = state.db.create_chapter_version(
        &crate::shared::version::types::CreateVersionRequest {
            novel_id: origin.novel_id.clone(),
            chapter_number: origin.chapter_number,
            content: origin.content.clone(),
            content_hash: origin.content_hash.clone(),
            word_count: origin.word_count,
            revision_mode: crate::shared::version::types::RevisionMode::Minor,
            revision_reason: format!("Restored from v{}", origin.version_number),
        },
        version_number,
        &origin.content_hash,
        origin.word_count,
    )?;
    Ok(IpcResponse::ok(new_version))
}