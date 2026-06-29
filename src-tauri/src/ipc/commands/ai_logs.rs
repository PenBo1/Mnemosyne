use crate::shared::errors::{AppError, IpcResponse};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn ai_log_llm_calls(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::ai_log_store::LlmCall>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let calls = state.db.get_llm_calls(&session_id, limit.unwrap_or(50)).await?;
    Ok(IpcResponse::ok(calls))
}

#[tauri::command]
pub async fn ai_log_tool_executions(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::ai_log_store::ToolExecution>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let execs = state.db.get_tool_executions(&session_id, limit.unwrap_or(50)).await?;
    Ok(IpcResponse::ok(execs))
}

#[tauri::command]
pub async fn ai_log_token_usage(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let (input, output) = state.db.get_session_token_usage(&session_id).await?;
    let tool_stats = state.db.get_session_tool_stats(&session_id).await?;
    let model_stats = state.db.get_model_usage_stats(&session_id).await?;
    Ok(IpcResponse::ok(serde_json::json!({
        "input_tokens": input,
        "output_tokens": output,
        "total_tokens": input + output,
        "tools": tool_stats,
        "models": model_stats,
    })))
}

#[tauri::command]
pub async fn ai_log_sandbox_violations(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::ai_log_store::SandboxViolation>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let violations = state.db.get_sandbox_violations(&session_id, limit.unwrap_or(50)).await?;
    Ok(IpcResponse::ok(violations))
}
