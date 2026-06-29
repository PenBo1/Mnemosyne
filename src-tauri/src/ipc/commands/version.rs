use std::path::PathBuf;
use tauri::State;
use crate::shared::errors::{AppError, IpcResponse};
use crate::features::version::{ChapterVersion, RevisionMode, LineDiffResult};
use crate::features::version::DiffEngine;
use crate::infrastructure::utils::text_utils::count_words;
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;

/// 列出某章的所有版本
#[tauri::command]
pub async fn version_list(
    state: State<'_, AppState>,
    novel_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<Vec<ChapterVersion>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if chapter_number == 0 || chapter_number > 100_000 {
        return Err(AppError::invalid_input("chapter_number must be between 1 and 100000"));
    }
    tracing::debug!(novel_id = %novel_id, chapter_number = chapter_number, "version_list");
    
    let versions = state.db.list_chapter_versions(&novel_id, chapter_number).await?;
    
    tracing::debug!(count = versions.len(), "Chapter versions listed");
    Ok(IpcResponse::ok(versions))
}

/// 按 ID 获取特定版本
#[tauri::command]
pub async fn version_get(
    state: State<'_, AppState>,
    version_id: String,
) -> Result<IpcResponse<ChapterVersion>, AppError> {
    validate_id_component(&version_id, "version_id")?;
    tracing::debug!(version_id = %version_id, "version_get");
    
    let version = state.db.get_chapter_version(&version_id).await?
        .ok_or_else(|| AppError::not_found("Version not found"))?;
    
    Ok(IpcResponse::ok(version))
}

/// 获取某章的最新版本
#[tauri::command]
pub async fn version_get_latest(
    state: State<'_, AppState>,
    novel_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<Option<ChapterVersion>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if chapter_number == 0 || chapter_number > 100_000 {
        return Err(AppError::invalid_input("chapter_number must be between 1 and 100000"));
    }
    tracing::debug!(novel_id = %novel_id, chapter_number = chapter_number, "version_get_latest");
    
    let version = state.db.get_latest_chapter_version(&novel_id, chapter_number).await?;
    
    Ok(IpcResponse::ok(version))
}

/// 计算两个版本之间的 diff
#[tauri::command]
pub async fn version_diff(
    state: State<'_, AppState>,
    from_version_id: String,
    to_version_id: String,
) -> Result<IpcResponse<LineDiffResult>, AppError> {
    validate_id_component(&from_version_id, "from_version_id")?;
    validate_id_component(&to_version_id, "to_version_id")?;
    tracing::debug!(from = %from_version_id, to = %to_version_id, "version_diff");
    
    let from_version = state.db.get_chapter_version(&from_version_id).await?
        .ok_or_else(|| AppError::not_found("From version not found"))?;
    
    let to_version = state.db.get_chapter_version(&to_version_id).await?
        .ok_or_else(|| AppError::not_found("To version not found"))?;
    
    let diff = DiffEngine::compute_line_diff(&from_version.content, &to_version.content);
    
    tracing::debug!(
        added = diff.stats.lines_added,
        removed = diff.stats.lines_removed,
        hunks = diff.hunks.len(),
        "Diff computed"
    );
    
    Ok(IpcResponse::ok(diff))
}

/// 计算某章最近两个版本之间的 diff
#[tauri::command]
pub async fn version_diff_latest(
    state: State<'_, AppState>,
    novel_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<Option<LineDiffResult>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if chapter_number == 0 || chapter_number > 100_000 {
        return Err(AppError::invalid_input("chapter_number must be between 1 and 100000"));
    }
    tracing::debug!(novel_id = %novel_id, chapter_number = chapter_number, "version_diff_latest");
    
    let versions = state.db.list_chapter_versions(&novel_id, chapter_number).await?;
    
    if versions.len() < 2 {
        return Ok(IpcResponse::ok(None));
    }
    
    let to_version = &versions[0];
    let from_version = &versions[1];
    
    let diff = DiffEngine::compute_line_diff(&from_version.content, &to_version.content);
    
    Ok(IpcResponse::ok(Some(diff)))
}

/// 将章节恢复到之前的版本
#[tauri::command]
pub async fn version_restore(
    state: State<'_, AppState>,
    version_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&version_id, "version_id")?;
    tracing::info!(version_id = %version_id, "version_restore");
    
    // 获取版本内容
    let version = state.db.get_chapter_version(&version_id).await?
        .ok_or_else(|| AppError::not_found("Version not found"))?;
    
    // 获取 novel 的 workspace 路径以找到 book_dir
    let novel = state.db.get_novel_by_id(&version.novel_id).await?
        .ok_or_else(|| AppError::not_found("Novel not found"))?;
    let workspace = state.db.get_workspace(&novel.workspace_id).await?
        .ok_or_else(|| AppError::not_found("Workspace not found"))?;
    let workspace_path = PathBuf::from(workspace.path);
    
    let book_dir = workspace_path.join(&version.novel_id);
    
    // 恢复内容
    crate::core::agent::pipeline::chapter_persistence::save_chapter_file(
        &book_dir,
        version.chapter_number,
        "",  // title preserved from file
        &version.content,
    )?;
    
    tracing::info!(
        version_id = %version_id,
        chapter_number = version.chapter_number,
        "Version restored"
    );
    
    Ok(IpcResponse::ok(true))
}

/// 手动保存新版本（用于测试或手动版本管理）
#[tauri::command]
pub async fn version_save(
    state: State<'_, AppState>,
    novel_id: String,
    chapter_number: u32,
    content: String,
    revision_mode: String,
    revision_reason: String,
) -> Result<IpcResponse<ChapterVersion>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if chapter_number == 0 || chapter_number > 100_000 {
        return Err(AppError::invalid_input("chapter_number must be between 1 and 100000"));
    }
    if content.is_empty() {
        return Err(AppError::invalid_input("Content cannot be empty"));
    }
    if content.len() > 10_000_000 {
        return Err(AppError::invalid_input("Content too long (max 10MB)"));
    }
    if revision_reason.len() > 1000 {
        return Err(AppError::invalid_input("Revision reason too long (max 1000 chars)"));
    }
    tracing::info!(novel_id = %novel_id, chapter_number = chapter_number, "version_save");
    
    let mode = revision_mode.parse::<RevisionMode>()
        .map_err(|e| AppError::invalid_input(e))?;
    
    // 获取下一个版本号
    let next_version_number = state.db.get_next_version_number(&novel_id, chapter_number).await?;
    
    // 计算内容 hash
    let content_hash = DiffEngine::compute_hash(&content);
    
    // 统计字数
    let word_count = count_words(&content);
    
    let request = crate::features::version::CreateVersionRequest {
        novel_id,
        chapter_number,
        content,
        revision_mode: mode,
        revision_reason,
    };
    
    let version = state.db.create_chapter_version(&request, next_version_number, &content_hash, word_count).await?;
    
    tracing::info!(version_id = %version.id, version_number = version.version_number, "Version saved");
    Ok(IpcResponse::created(version))
}
