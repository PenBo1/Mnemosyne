use crate::shared::errors::{AppError, IpcResponse};
use crate::core::agent::pipeline::{Scheduler, SchedulerConfig, WriteCycleResult};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// 为某个 workspace 初始化 scheduler。
#[tauri::command]
pub async fn scheduler_init(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    let ws = state.db.get_workspace(&workspace_id).await?
        .ok_or_else(|| AppError::not_found("Workspace not found"))?;
    let workspace_path = std::path::PathBuf::from(ws.path);

    // 构建 pipeline runner
    let memory_store = state.memory_store.clone();
    let feedback_store = {
        crate::infrastructure::state_store::feedback::FeedbackStore::new() // 为 scheduler 新建一个 store
    };

    let registry = state.provider_registry.lock().await;
    let provider = registry.default()?;
    let model = registry.default_model().to_string();
    // S9: 构建 per-agent 路由
    let (model_overrides, agent_providers) = registry.build_agent_routing();
    drop(registry);

    let pipeline = crate::core::agent::pipeline::PipelineRunner::new(crate::core::agent::pipeline::PipelineConfig {
        provider,
        model,
        project_root: workspace_path,
        model_overrides,
        agent_providers,
        memory_store: Some(memory_store.clone()),
        data_dir: state.data_dir.clone(),
        user_profile: None,
        fallback_model: None,
        db: Some(state.db.clone()),
        context_budget: None,
    });

    let config = SchedulerConfig::default();
    let scheduler = Arc::new(Scheduler::new(pipeline, config, memory_store, feedback_store));

    // 启动 scheduler
    scheduler.start().await?;

    // 存入 app state
    *state.scheduler.lock().await = Some(scheduler);

    tracing::info!(workspace_id, "Scheduler initialized");
    Ok(IpcResponse::ok(format!("Scheduler initialized for workspace {}", workspace_id)))
}

/// 为某本书执行一次写作循环。
#[tauri::command]
pub async fn scheduler_write_cycle(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<WriteCycleResult>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let result = scheduler.execute_write_cycle(&book_id).await?;
    Ok(IpcResponse::ok(result))
}

/// 获取 scheduler 状态。
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

/// 暂停 scheduler。
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

/// 恢复 scheduler。
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

/// 停止 scheduler。
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

/// 在 RAG 中检索相关内容。
#[tauri::command]
pub async fn scheduler_search_rag(
    state: State<'_, AppState>,
    query: String,
    top_k: Option<usize>,
) -> Result<IpcResponse<Vec<crate::infrastructure::ai_services::rag::SearchResult>>, AppError> {
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let results = scheduler.search_rag(&query, top_k.unwrap_or(5)).await;
    Ok(IpcResponse::ok(results))
}

/// 在 memory 中检索相关 fact。
#[tauri::command]
pub async fn scheduler_search_memory(
    state: State<'_, AppState>,
    book_id: String,
    query: String,
    top_k: Option<usize>,
) -> Result<IpcResponse<Vec<crate::core::agent::MemoryEntry>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    if query.len() > 1000 {
        return Err(AppError::invalid_input("Query too long (max 1000 chars)"));
    }
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let results = scheduler.search_memory(&book_id, &query, top_k.unwrap_or(5)).await;
    Ok(IpcResponse::ok(results))
}

/// 获取用于 prompt 注入的 feedback lesson。
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

/// 从 checkpoint 恢复。
#[tauri::command]
pub async fn scheduler_restore_checkpoint(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<Option<crate::core::agent::pipeline::GraphState>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let scheduler = {
        let s = state.scheduler.lock().await;
        s.as_ref().ok_or_else(|| AppError::internal("Scheduler not initialized"))?.clone()
    };

    let state = scheduler.restore_checkpoint(&book_id).await?;
    Ok(IpcResponse::ok(state))
}
