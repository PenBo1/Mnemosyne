use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::infra::db::models::{CreateKanbanTaskRequest, UpdateKanbanTaskRequest, CreateKanbanColumnRequest, UpdateKanbanColumnRequest};
use crate::infra::fs_utils::validate_id_component;
use crate::AppState;

#[tauri::command]
pub async fn kanban_create_task(
    state: State<'_, AppState>,
    novel_id: String,
    title: String,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assigned_agent: Option<String>,
    chapter_id: Option<String>,
    parent_task_id: Option<String>,
    tags: Option<Vec<String>>,
    due_date: Option<String>,
) -> Result<IpcResponse<crate::infra::db::models::KanbanTask>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Task title cannot be empty"));
    }
    if title.len() > 500 {
        return Err(AppError::invalid_input("Task title too long (max 500 chars)"));
    }
    if let Some(ref ch) = chapter_id {
        validate_id_component(ch, "chapter_id")?;
    }
    if let Some(ref pt) = parent_task_id {
        validate_id_component(pt, "parent_task_id")?;
    }

    tracing::info!(novel_id = %novel_id, title = %title, "kanban_create_task");
    let task = state.db.create_kanban_task(&novel_id, CreateKanbanTaskRequest {
        title,
        description,
        status,
        priority,
        assigned_agent,
        chapter_id,
        parent_task_id,
        tags,
        due_date,
    }).await?;
    tracing::info!(task_id = %task.id, "Kanban task created");
    Ok(IpcResponse::created(task))
}

#[tauri::command]
pub async fn kanban_get_tasks(
    state: State<'_, AppState>,
    novel_id: String,
    status_filter: Option<String>,
) -> Result<IpcResponse<Vec<crate::infra::db::models::KanbanTask>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    let tasks = state.db.get_kanban_tasks(&novel_id, status_filter.as_deref()).await?;
    Ok(IpcResponse::ok(tasks))
}

#[tauri::command]
pub async fn kanban_update_task(
    state: State<'_, AppState>,
    task_id: String,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assigned_agent: Option<String>,
    chapter_id: Option<String>,
    parent_task_id: Option<String>,
    sort_order: Option<i32>,
    due_date: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<IpcResponse<crate::infra::db::models::KanbanTask>, AppError> {
    validate_id_component(&task_id, "task_id")?;

    let task = state.db.update_kanban_task(&task_id, UpdateKanbanTaskRequest {
        title,
        description,
        status,
        priority,
        assigned_agent,
        chapter_id,
        parent_task_id,
        sort_order,
        due_date,
        tags,
    }).await?;
    Ok(IpcResponse::ok(task))
}

#[tauri::command]
pub async fn kanban_delete_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&task_id, "task_id")?;
    state.db.delete_kanban_task(&task_id).await?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn kanban_reorder_tasks(
    state: State<'_, AppState>,
    task_ids: Vec<String>,
) -> Result<IpcResponse<()>, AppError> {
    for id in &task_ids {
        validate_id_component(id, "task_id")?;
    }
    state.db.reorder_kanban_tasks(&task_ids).await?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn kanban_get_columns(
    state: State<'_, AppState>,
    novel_id: String,
) -> Result<IpcResponse<Vec<crate::infra::db::models::KanbanColumn>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    let columns = state.db.get_kanban_columns(&novel_id).await?;
    Ok(IpcResponse::ok(columns))
}

#[tauri::command]
pub async fn kanban_create_column(
    state: State<'_, AppState>,
    novel_id: String,
    name: String,
    status_key: String,
    color: Option<String>,
    sort_order: Option<i32>,
    wip_limit: Option<i32>,
) -> Result<IpcResponse<crate::infra::db::models::KanbanColumn>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("Column name cannot be empty"));
    }

    let column = state.db.create_kanban_column(&novel_id, CreateKanbanColumnRequest {
        name,
        status_key,
        color,
        sort_order,
        wip_limit,
    }).await?;
    Ok(IpcResponse::created(column))
}

#[tauri::command]
pub async fn kanban_update_column(
    state: State<'_, AppState>,
    column_id: String,
    name: Option<String>,
    color: Option<String>,
    sort_order: Option<i32>,
    wip_limit: Option<i32>,
) -> Result<IpcResponse<crate::infra::db::models::KanbanColumn>, AppError> {
    validate_id_component(&column_id, "column_id")?;
    let column = state.db.update_kanban_column(&column_id, UpdateKanbanColumnRequest {
        name,
        color,
        sort_order,
        wip_limit,
    }).await?;
    Ok(IpcResponse::ok(column))
}

#[tauri::command]
pub async fn kanban_delete_column(
    state: State<'_, AppState>,
    column_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&column_id, "column_id")?;
    state.db.delete_kanban_column(&column_id).await?;
    Ok(IpcResponse::ok(()))
}
