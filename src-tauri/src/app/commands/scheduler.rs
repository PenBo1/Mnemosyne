use crate::errors::{AppError, IpcResponse};
use crate::domain::pipeline::{Scheduler, SchedulerConfig, WriteCycleResult};
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// Initialize the scheduler for a workspace.
/// Must be called before any scheduler operations.
#[tauri::command]
pub async fn scheduler_init(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<IpcResponse<String>, AppError> {
    // Get workspace path
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    // Build pipeline runner
    let memory_store = state.memory_store.clone();
    let feedback_store = {
        crate::infra::feedback::FeedbackStore::new() // fresh store for scheduler
    };

    let registry = state.provider_registry.lock().await;
    let provider = registry.default()?;
    let model = registry.default_model().to_string();
    drop(registry);

    let pipeline = crate::domain::pipeline::PipelineRunner::new(crate::domain::pipeline::PipelineConfig {
        provider,
        model,
        project_root: workspace_path,
        model_overrides: std::collections::HashMap::new(),
        memory_store: Some(memory_store.clone()),
        data_dir: state.data_dir.clone(),
        harness_config: None,
        user_profile: None,
    });

    let config = SchedulerConfig::default();
    let scheduler = Arc::new(Scheduler::new(pipeline, config, memory_store, feedback_store));

    // Start the scheduler
    scheduler.start().await?;

    // Store in app state
    *state.scheduler.lock().await = Some(scheduler);

    tracing::info!(workspace_id, "Scheduler initialized");
    Ok(IpcResponse::ok(format!("Scheduler initialized for workspace {}", workspace_id)))
}

/// Execute a write cycle for a book.
#[tauri::command]
pub async fn scheduler_write_cycle(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<WriteCycleResult>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let result = scheduler.execute_write_cycle(&book_id).await?;
    Ok(IpcResponse::ok(result))
}

/// Get scheduler status.
#[tauri::command]
pub async fn scheduler_status(
    state: State<'_, AppState>,
) -> Result<IpcResponse<String>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let status = scheduler.status().await;
    let status_str = format!("{:?}", status);
    Ok(IpcResponse::ok(status_str))
}

/// List all scheduled tasks.
#[tauri::command]
pub async fn scheduler_list_tasks(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::domain::pipeline::scheduler::ScheduledTask>>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let tasks = scheduler.list_tasks().await;
    Ok(IpcResponse::ok(tasks))
}

/// Pause the scheduler.
#[tauri::command]
pub async fn scheduler_pause(
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    scheduler.pause().await;
    Ok(IpcResponse::ok(()))
}

/// Resume the scheduler.
#[tauri::command]
pub async fn scheduler_resume(
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    scheduler.resume().await;
    Ok(IpcResponse::ok(()))
}

/// Stop the scheduler.
#[tauri::command]
pub async fn scheduler_stop(
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    scheduler.stop().await;
    Ok(IpcResponse::ok(()))
}

/// Search RAG for relevant content.
#[tauri::command]
pub async fn scheduler_search_rag(
    state: State<'_, AppState>,
    query: String,
    top_k: Option<usize>,
) -> Result<IpcResponse<Vec<crate::infra::rag::SearchResult>>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let results = scheduler.search_rag(&query, top_k.unwrap_or(5)).await;
    Ok(IpcResponse::ok(results))
}

/// Search memory for relevant facts.
#[tauri::command]
pub async fn scheduler_search_memory(
    state: State<'_, AppState>,
    book_id: String,
    query: String,
    top_k: Option<usize>,
) -> Result<IpcResponse<Vec<crate::domain::agents::MemoryEntry>>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let results = scheduler.search_memory(&book_id, &query, top_k.unwrap_or(5)).await;
    Ok(IpcResponse::ok(results))
}

/// Get feedback lessons for prompt injection.
#[tauri::command]
pub async fn scheduler_get_lessons(
    state: State<'_, AppState>,
) -> Result<IpcResponse<String>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let lessons = scheduler.get_lessons_for_prompt().await;
    Ok(IpcResponse::ok(lessons))
}

/// Restore from checkpoint.
#[tauri::command]
pub async fn scheduler_restore_checkpoint(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<Option<crate::domain::pipeline::GraphState>>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let state = scheduler.restore_checkpoint(&book_id).await?;
    Ok(IpcResponse::ok(state))
}
