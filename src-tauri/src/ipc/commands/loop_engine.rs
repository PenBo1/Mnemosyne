use tauri::State;
use crate::shared::errors::{IpcResponse, AppError};
use crate::infrastructure::db::models::{CreateLoopStateRequest, UpdateLoopStateRequest, UpsertLoopPatternRequest};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;

#[tauri::command]
pub async fn loop_create_state(
    state: State<'_, AppState>,
    novel_id: String,
    pattern_id: String,
    readiness_level: Option<String>,
    config: Option<serde_json::Value>,
    token_cap_daily: Option<i64>,
) -> Result<IpcResponse<crate::infrastructure::db::models::LoopState>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    if pattern_id.trim().is_empty() {
        return Err(AppError::invalid_input("Pattern ID cannot be empty"));
    }

    tracing::info!(novel_id = %novel_id, pattern_id = %pattern_id, "loop_create_state");
    let loop_state = state.db.create_loop_state(&novel_id, CreateLoopStateRequest {
        pattern_id,
        readiness_level,
        config,
        token_cap_daily,
    }).await?;
    tracing::info!(loop_id = %loop_state.id, "Loop state created");
    Ok(IpcResponse::created(loop_state))
}

#[tauri::command]
pub async fn loop_get_states(
    state: State<'_, AppState>,
    novel_id: String,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::models::LoopState>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    let states = state.db.get_loop_states(&novel_id).await?;
    Ok(IpcResponse::ok(states))
}

#[tauri::command]
pub async fn loop_update_state(
    state: State<'_, AppState>,
    state_id: String,
    status: Option<String>,
    readiness_level: Option<String>,
    config: Option<serde_json::Value>,
    token_cap_daily: Option<i64>,
) -> Result<IpcResponse<crate::infrastructure::db::models::LoopState>, AppError> {
    validate_id_component(&state_id, "state_id")?;
    let loop_state = state.db.update_loop_state(&state_id, UpdateLoopStateRequest {
        status,
        readiness_level,
        config,
        token_cap_daily,
        ..Default::default()
    }).await?;
    Ok(IpcResponse::ok(loop_state))
}

#[tauri::command]
pub async fn loop_delete_state(
    state: State<'_, AppState>,
    state_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&state_id, "state_id")?;
    state.db.delete_loop_state(&state_id).await?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn loop_run_cycle(
    state: State<'_, AppState>,
    state_id: String,
) -> Result<IpcResponse<crate::infrastructure::db::models::LoopRunLog>, AppError> {
    validate_id_component(&state_id, "state_id")?;

    let ls = state.db.get_loop_state_by_id(&state_id).await?;
    let pattern_id = ls.pattern_id.clone();
    let token_cap = ls.token_cap_daily;
    let token_usage = ls.token_usage_today;

    if token_usage >= token_cap {
        return Err(AppError::internal("Token budget exceeded for this loop"));
    }

    // 先标记 running，确保后续无论 Ok/Err 都会更新到 idle/failed（避免状态分裂）
    state.db.update_loop_state(&state_id, UpdateLoopStateRequest {
        status: Some("running".to_string()),
        ..Default::default()
    }).await?;

    tracing::info!(state_id = %state_id, pattern_id = %pattern_id, "loop_run_cycle");

    // 查找 pattern：DB 优先，回退到内置 registry（应对 DB 未种子化的情况）
    let pattern = match state.db.get_loop_patterns().await?.into_iter().find(|p| p.id == pattern_id) {
        Some(p) => p,
        None => crate::core::agent::loop_engine::PatternRegistry::built_in_patterns()
            .into_iter()
            .find(|p| p.id == pattern_id)
            .ok_or_else(|| AppError::not_found(format!("Loop pattern not found: {}", pattern_id)))?,
    };

    let now_rfc = chrono::Utc::now().to_rfc3339();

    // 调用 engine；用 match 保证无论 Ok/Err 都会写 log + 更新 state
    let log = match crate::core::agent::loop_engine::LoopEngine::run_cycle(&ls, &pattern) {
        Ok(log) => {
            // 成功：写 log，更新 state 为 idle
            state.db.create_loop_run_log(&log).await?;
            state.db.update_loop_state(&state_id, UpdateLoopStateRequest {
                status: Some("idle".to_string()),
                last_run_at: Some(now_rfc.clone()),
                last_run_result: Some(serde_json::json!({
                    "findings": log.findings,
                    "actions": log.actions_taken,
                    "escalations": log.escalations
                })),
                ..Default::default()
            }).await?;
            log
        }
        Err(e) => {
            // 失败：仍写一条 failed log，更新 state 为 failed（避免状态分裂）
            let failed_log = crate::infrastructure::db::models::LoopRunLog {
                id: uuid::Uuid::new_v4().to_string(),
                loop_state_id: state_id.clone(),
                pattern_id: pattern_id.clone(),
                status: "failed".to_string(),
                phase_results: vec![],
                tokens_used: 0,
                duration_ms: 0,
                findings: vec![],
                actions_taken: vec![],
                escalations: vec![],
                error_message: Some(e.to_string()),
                created_at: now_rfc.clone(),
            };
            state.db.create_loop_run_log(&failed_log).await?;
            state.db.update_loop_state(&state_id, UpdateLoopStateRequest {
                status: Some("failed".to_string()),
                last_run_at: Some(now_rfc),
                last_run_result: Some(serde_json::json!({ "error": e.to_string() })),
                ..Default::default()
            }).await?;
            return Err(e);
        }
    };

    Ok(IpcResponse::ok(log))
}

#[tauri::command]
pub async fn loop_get_run_logs(
    state: State<'_, AppState>,
    state_id: String,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::models::LoopRunLog>>, AppError> {
    validate_id_component(&state_id, "state_id")?;
    let logs = state.db.get_loop_run_logs(&state_id, limit.unwrap_or(50)).await?;
    Ok(IpcResponse::ok(logs))
}

#[tauri::command]
pub async fn loop_get_patterns(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::models::LoopPattern>>, AppError> {
    let patterns = state.db.get_loop_patterns().await?;
    Ok(IpcResponse::ok(patterns))
}

#[tauri::command]
pub async fn loop_upsert_pattern(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    description: Option<String>,
    goal: Option<String>,
    cadence: Option<String>,
    risk_level: Option<String>,
    phases: Option<Vec<serde_json::Value>>,
    human_gates: Option<Vec<String>>,
    cost_config: Option<serde_json::Value>,
    skills_required: Option<Vec<String>>,
    state_schema: Option<serde_json::Value>,
    is_active: Option<bool>,
) -> Result<IpcResponse<crate::infrastructure::db::models::LoopPattern>, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("Pattern name cannot be empty"));
    }

    let pattern = state.db.upsert_loop_pattern(id.as_deref(), UpsertLoopPatternRequest {
        name,
        description,
        goal,
        cadence,
        risk_level,
        phases,
        human_gates,
        cost_config,
        skills_required,
        state_schema,
        is_active,
    }).await?;
    Ok(IpcResponse::created(pattern))
}

#[tauri::command]
pub async fn loop_pause(
    state: State<'_, AppState>,
    state_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&state_id, "state_id")?;
    state.db.update_loop_state(&state_id, UpdateLoopStateRequest {
        status: Some("paused".to_string()),
        ..Default::default()
    }).await?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn loop_resume(
    state: State<'_, AppState>,
    state_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&state_id, "state_id")?;
    state.db.update_loop_state(&state_id, UpdateLoopStateRequest {
        status: Some("idle".to_string()),
        ..Default::default()
    }).await?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn loop_get_budget_status(
    state: State<'_, AppState>,
    state_id: String,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    validate_id_component(&state_id, "state_id")?;
    let ls = state.db.get_loop_state_by_id(&state_id).await?;
    let remaining = ls.token_cap_daily - ls.token_usage_today;
    let usage_percent = if ls.token_cap_daily > 0 {
        (ls.token_usage_today as f64 / ls.token_cap_daily as f64 * 100.0) as i64
    } else {
        0
    };
    Ok(IpcResponse::ok(serde_json::json!({
        "used": ls.token_usage_today,
        "cap": ls.token_cap_daily,
        "remaining": remaining.max(0),
        "usage_percent": usage_percent,
        "exceeded": ls.token_usage_today >= ls.token_cap_daily,
    })))
}
