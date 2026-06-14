use crate::errors::{AppError, IpcResponse};
use crate::domain::pipeline;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn novel_create(
    state: State<'_, AppState>,
    workspace_id: String,
    title: String,
    genre: String,
    brief: Option<String>,
) -> Result<IpcResponse<crate::domain::story::BookConfig>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    let config = runner.create_book(&title, &genre, brief.as_deref()).await?;

    {
        let db = state.db.lock().await;
        db.create_novel_with_workspace(&config.id, &workspace_id, &title, &genre)?;
    }

    Ok(IpcResponse::created(config))
}

#[tauri::command]
pub async fn novel_write_next(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    target_words: Option<u32>,
) -> Result<IpcResponse<crate::domain::story::WriteResult>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    let result = runner.write_next_chapter(&book_id, target_words).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_plan(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    context: Option<String>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    let result = runner.plan_chapter(&book_id, context.as_deref()).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_audit(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<crate::domain::story::AuditResult>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    let result = runner.audit_chapter(&book_id, chapter_number).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_revise(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<String>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    let audit = runner.audit_chapter(&book_id, chapter_number).await?;
    let result = runner.revise_chapter(&book_id, chapter_number, &audit).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_observe(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    let result = runner.observe_chapter(&book_id, chapter_number).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_reflect(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<()>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = pipeline::build_runner_with_harness(
        &state.provider_registry,
        &state.global_harness,
        &state.agent_configs,
        workspace_path,
        state.db.clone(),
    ).await?;
    runner.reflect_chapter(&book_id, chapter_number).await?;
    Ok(IpcResponse::ok(()))
}
