use crate::errors::{AppError, IpcResponse};
use crate::infra::mcp::McpRequest;
use crate::infra::fs_utils::validate_id_component;
use crate::AppState;
use tauri::State;

/// Handle an MCP JSON-RPC request.
#[tauri::command]
pub async fn mcp_handle_request(
    state: State<'_, AppState>,
    session_id: String,
    request: McpRequest,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let server = state.mcp_server.lock().await;
    let response = server.handle_request(request, &session_id).await;
    Ok(IpcResponse::ok(serde_json::to_value(&response).unwrap_or_default()))
}

/// Get MCP server info (capabilities, tools, resources, prompts).
#[tauri::command]
pub async fn mcp_server_info(
    state: State<'_, AppState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let _server = state.mcp_server.lock().await;
    Ok(IpcResponse::ok(serde_json::json!({
        "protocol_version": "2025-03-26",
        "server_info": { "name": "mnemosyne", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false }
        }
    })))
}

/// Detect tool poisoning in a tool description.
#[tauri::command]
pub async fn mcp_check_tool_safety(
    tool_name: String,
    description: String,
) -> Result<IpcResponse<bool>, AppError> {
    if tool_name.trim().is_empty() {
        return Err(AppError::invalid_input("Tool name cannot be empty"));
    }
    if tool_name.len() > 255 {
        return Err(AppError::invalid_input("Tool name too long (max 255 chars)"));
    }
    if description.len() > 50_000 {
        return Err(AppError::invalid_input("Tool description too long (max 50000 chars)"));
    }
    let mut detector = crate::infra::mcp::ToolPoisoningDetector::new();
    detector.register_tool_hash(&tool_name, &description);
    match detector.check_tool(&tool_name, &description) {
        Ok(()) => Ok(IpcResponse::ok(true)),
        Err(e) => {
            tracing::warn!(tool = %tool_name, error = %e, "Tool poisoning detected");
            Ok(IpcResponse::ok(false))
        }
    }
}
