use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use crate::domain::agent::{AgentLoop, Op, Submission};
use crate::app::state::AgentHandle;
use tauri::State;
use tauri::Emitter;

async fn ensure_agent_running(state: &AppState) -> Result<AgentHandle, AppError> {
    let mut handle_guard = state.agent_handle.lock().await;
    if let Some(handle) = handle_guard.as_ref() {
        return Ok(AgentHandle { tx_sub: handle.tx_sub.clone() });
    }

    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider = registry.default()
            .map_err(|e| AppError::provider_not_found(e.to_string()))?;
        let model = registry.default_model().to_string();
        (provider, model)
    };

    let (agent_loop, tx_sub) = AgentLoop::new();
    let resources = AgentLoop::build_resources(
        state.db.clone(),
        state.tool_registry.clone(),
        provider,
        model,
        std::env::current_dir().unwrap_or_default().to_string_lossy().to_string(),
    );

    let (tx_event, mut rx_event) = tokio::sync::mpsc::channel(256);
    let app_handle = state.app_handle.clone();

    tokio::spawn(async move {
        let mut agent = agent_loop;
        agent.run(resources, tx_event).await;
    });

    tokio::spawn(async move {
        while let Some(event) = rx_event.recv().await {
            tracing::debug!(event_type = ?std::mem::discriminant(&event), "Emitting agent event");
            if let Err(e) = app_handle.emit("agent-event", &event) {
                tracing::error!("Failed to emit agent event: {}", e);
            }
        }
    });

    let handle = AgentHandle { tx_sub };
    *handle_guard = Some(AgentHandle { tx_sub: handle.tx_sub.clone() });
    tracing::info!("Agent loop started (lazy)");
    Ok(handle)
}

async fn restart_agent(state: &AppState) -> Result<AgentHandle, AppError> {
    let mut handle_guard = state.agent_handle.lock().await;
    *handle_guard = None;
    drop(handle_guard);
    ensure_agent_running(state).await
}

#[tauri::command]
pub async fn agent_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
) -> Result<IpcResponse<String>, AppError> {
    tracing::info!(session_id = %session_id, content_len = content.len(), "agent_send_message called");

    let handle = ensure_agent_running(&state).await?;

    let submission = Submission {
        id: uuid::Uuid::new_v4().to_string(),
        op: Op::UserInput { session_id, content },
    };

    handle.tx_sub.send(submission).await
        .map_err(|e| AppError::internal(format!("Failed to send to agent: {}", e)))?;

    tracing::info!("Message sent to agent loop");
    Ok(IpcResponse::ok("Message received".to_string()))
}

#[tauri::command]
pub async fn agent_approve_tool(
    state: State<'_, AppState>,
    tool_call_id: String,
    approved: bool,
) -> Result<IpcResponse<()>, AppError> {
    let handle = ensure_agent_running(&state).await?;

    let submission = Submission {
        id: uuid::Uuid::new_v4().to_string(),
        op: Op::ToolApproval { tool_call_id, approved },
    };

    handle.tx_sub.send(submission).await
        .map_err(|e| AppError::internal(format!("Failed to send approval: {}", e)))?;

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_cancel(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    let handle = ensure_agent_running(&state).await?;

    let submission = Submission {
        id: uuid::Uuid::new_v4().to_string(),
        op: Op::Cancel { session_id },
    };

    handle.tx_sub.send(submission).await
        .map_err(|e| AppError::internal(format!("Failed to send cancel: {}", e)))?;

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_compact(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    let handle = ensure_agent_running(&state).await?;

    let submission = Submission {
        id: uuid::Uuid::new_v4().to_string(),
        op: Op::Compact { session_id },
    };

    handle.tx_sub.send(submission).await
        .map_err(|e| AppError::internal(format!("Failed to send compact: {}", e)))?;

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_restart(
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    restart_agent(&state).await?;
    Ok(IpcResponse::ok(()))
}
