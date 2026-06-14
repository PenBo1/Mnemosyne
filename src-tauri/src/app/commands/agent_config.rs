use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::infra::db::models::{AgentRow, UpdateAgentRequest};

#[tauri::command]
pub async fn list_agents(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<AgentRow>>, AppError> {
    tracing::debug!("list_agents");
    let db = state.db.lock().await;
    let agents = db.list_agents()?;
    tracing::debug!(count = agents.len(), "Agents listed");
    Ok(IpcResponse::ok(agents))
}

#[tauri::command]
pub async fn update_agent(
    state: State<'_, AppState>,
    req: UpdateAgentRequest,
) -> Result<IpcResponse<AgentRow>, AppError> {
    tracing::info!(agent_id = %req.id, "update_agent");
    let db = state.db.lock().await;
    let agent = db.update_agent(req)?;
    tracing::info!(agent_id = %agent.id, "Agent updated");
    Ok(IpcResponse::ok(agent))
}

#[tauri::command]
pub async fn toggle_agent_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<AgentRow>, AppError> {
    tracing::info!(agent_id = %id, "toggle_agent_status");
    let db = state.db.lock().await;
    let agent = db.toggle_agent_status(&id)?;
    tracing::info!(agent_id = %agent.id, status = %agent.status, "Agent status toggled");
    Ok(IpcResponse::ok(agent))
}
