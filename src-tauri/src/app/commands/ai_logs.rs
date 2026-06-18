use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn ai_log_llm_calls(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::infra::db::ai_log_store::LlmCall>>, AppError> {
    let db = state.db.lock().await;
    let calls = db.get_llm_calls(&session_id, limit.unwrap_or(50))?;
    Ok(IpcResponse::ok(calls))
}

#[tauri::command]
pub async fn ai_log_tool_executions(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::infra::db::ai_log_store::ToolExecution>>, AppError> {
    let db = state.db.lock().await;
    let execs = db.get_tool_executions(&session_id, limit.unwrap_or(50))?;
    Ok(IpcResponse::ok(execs))
}

#[tauri::command]
pub async fn ai_log_token_usage(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    let (input, output) = db.get_session_token_usage(&session_id)?;
    let tool_stats = db.get_session_tool_stats(&session_id)?;
    let model_stats = db.get_model_usage_stats(&session_id)?;
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
) -> Result<IpcResponse<Vec<crate::infra::db::ai_log_store::SandboxViolation>>, AppError> {
    let db = state.db.lock().await;
    let mut stmt = db.conn.prepare(
        "SELECT id, session_id, violation_type, resource, action, rule_matched, tool_name, arguments_json, detected_at, created_at FROM sandbox_violations WHERE session_id = ?1 ORDER BY detected_at DESC LIMIT ?2"
    ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
    let rows = stmt.query_map(rusqlite::params![session_id, limit.unwrap_or(50)], |row| {
        Ok(crate::infra::db::ai_log_store::SandboxViolation {
            id: row.get(0)?,
            session_id: row.get(1)?,
            violation_type: row.get(2)?,
            resource: row.get(3)?,
            action: row.get(4)?,
            rule_matched: row.get(5)?,
            tool_name: row.get(6)?,
            arguments_json: row.get(7)?,
            detected_at: row.get(8)?,
            created_at: row.get(9)?,
        })
    }).map_err(|e| AppError::internal(format!("Failed to query violations: {}", e)))?;
    let violations = rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(format!("Failed to collect violations: {}", e)))?;
    Ok(IpcResponse::ok(violations))
}
