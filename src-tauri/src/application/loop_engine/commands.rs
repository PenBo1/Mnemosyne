//! ═══════════════════════════════════════════════════════════════════════════
//! Commands - 循环引擎 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供前端 loop_* 命令的后端实现：
//! - loop_create_state：创建循环状态
//! - loop_get_states：列出所有循环状态
//! - loop_get_state：获取单个循环状态
//! - loop_update_state：更新循环状态
//! - loop_delete_state：删除循环状态
//! - loop_pause/loop_resume：暂停/恢复循环
//! - loop_get_run_logs：获取运行日志
//! - loop_get_patterns：获取所有模式
//! - loop_upsert_pattern：创建或更新模式
//! - loop_delete_pattern：删除用户定义模式

use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::loop_pattern::LoopPatternRow;
use crate::infrastructure::db::stores::loop_run::LoopRunRow;
use crate::infrastructure::db::stores::loop_state::LoopStateRow;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;
use uuid::Uuid;

use super::conversions::{
    default_config, pattern_row_to_dto, run_row_to_log_dto, state_row_to_dto,
};
use super::types::{CostConfigDto, LoopConfigDto, LoopPatternDto, LoopRunLogDto, LoopStateDto};

// ── 校验辅助函数 ────────────────────────────────────────────────────────

fn validate_loop_status(status: &str) -> Result<(), AppError> {
    match status {
        "idle" | "running" | "paused" | "error" => Ok(()),
        _ => Err(AppError::invalid_input(format!(
            "Invalid loop status: {} (expected idle/running/paused/error)",
            status
        ))),
    }
}

fn validate_readiness_level(level: &str) -> Result<(), AppError> {
    match level {
        "L0" | "L1" | "L2" | "L3" => Ok(()),
        _ => Err(AppError::invalid_input(format!(
            "Invalid readiness level: {} (expected L0/L1/L2/L3)",
            level
        ))),
    }
}

fn validate_risk_level(level: &str) -> Result<(), AppError> {
    match level {
        "low" | "medium" | "high" => Ok(()),
        _ => Err(AppError::invalid_input(format!(
            "Invalid risk level: {} (expected low/medium/high)",
            level
        ))),
    }
}

// ── IPC 命令实现 ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn loop_create_state(
    state: State<'_, DbState>,
    novel_id: String,
    pattern_id: String,
    readiness_level: Option<String>,
    config: Option<LoopConfigDto>,
    token_cap_daily: Option<u64>,
) -> Result<IpcResponse<LoopStateDto>, AppError> {
    validate_id_component(&novel_id, "novelId")?;
    validate_id_component(&pattern_id, "patternId")?;

    let readiness = readiness_level.unwrap_or_else(|| "L0".to_string());
    validate_readiness_level(&readiness)?;

    // 验证 pattern 存在
    let pattern = state
        .db
        .get_loop_pattern(&pattern_id)?
        .ok_or_else(|| AppError::not_found(format!("Loop pattern '{}' not found", pattern_id)))?;
    if pattern.is_active == 0 {
        return Err(AppError::invalid_input(format!(
            "Loop pattern '{}' is not active",
            pattern_id
        )));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config = config.unwrap_or_else(default_config);
    let config_json = serde_json::to_string(&config)
        .map_err(|e| AppError::invalid_format(format!("Failed to serialize config: {}", e)))?;

    let row = LoopStateRow {
        id: id.clone(),
        novel_id: novel_id.clone(),
        pattern_id: pattern_id.clone(),
        status: "idle".to_string(),
        readiness_level: readiness,
        state_payload: None,
        config: Some(config_json),
        token_usage_today: 0,
        token_cap_daily: token_cap_daily.unwrap_or(50_000) as i64,
        last_run_at: None,
        last_run_result: None,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.insert_loop_state(&row)?;
    tracing::info!(state_id = %id, novel_id = %novel_id, pattern_id = %pattern_id, "Loop state created");

    let dto = state_row_to_dto(row)?;
    Ok(IpcResponse::created(dto))
}

#[tauri::command]
pub async fn loop_get_states(
    state: State<'_, DbState>,
    novel_id: String,
) -> Result<IpcResponse<Vec<LoopStateDto>>, AppError> {
    validate_id_component(&novel_id, "novelId")?;
    let rows = state.db.list_loop_states(&novel_id)?;
    let dtos: Vec<LoopStateDto> = rows
        .into_iter()
        .map(state_row_to_dto)
        .collect::<Result<_, _>>()?;
    Ok(IpcResponse::ok(dtos))
}

#[tauri::command]
pub async fn loop_get_state(
    state: State<'_, DbState>,
    state_id: String,
) -> Result<IpcResponse<LoopStateDto>, AppError> {
    validate_id_component(&state_id, "stateId")?;
    let row = state
        .db
        .get_loop_state(&state_id)?
        .ok_or_else(|| AppError::not_found(format!("Loop state '{}' not found", state_id)))?;
    let dto = state_row_to_dto(row)?;
    Ok(IpcResponse::ok(dto))
}

#[tauri::command]
pub async fn loop_update_state(
    state: State<'_, DbState>,
    state_id: String,
    status: Option<String>,
    readiness_level: Option<String>,
    config: Option<LoopConfigDto>,
    token_cap_daily: Option<u64>,
) -> Result<IpcResponse<LoopStateDto>, AppError> {
    validate_id_component(&state_id, "stateId")?;
    if let Some(ref s) = status {
        validate_loop_status(s)?;
    }
    if let Some(ref r) = readiness_level {
        validate_readiness_level(r)?;
    }

    let config_json = if let Some(ref c) = config {
        Some(serde_json::to_string(c).map_err(|e| AppError::invalid_format(format!("Failed to serialize config: {}", e)))?)
    } else {
        None
    };

    let now = chrono::Utc::now().to_rfc3339();
    let updated = state.db.update_loop_state(
        &state_id,
        status.as_deref(),
        readiness_level.as_deref(),
        None,
        config_json.as_deref(),
        None,
        token_cap_daily.map(|v| v as i64),
        None,
        None,
        &now,
    )?;

    if !updated {
        return Err(AppError::not_found(format!(
            "Loop state '{}' not found",
            state_id
        )));
    }

    let row = state
        .db
        .get_loop_state(&state_id)?
        .ok_or_else(|| AppError::not_found(format!("Loop state '{}' not found", state_id)))?;
    let dto = state_row_to_dto(row)?;
    Ok(IpcResponse::updated(dto))
}

#[tauri::command]
pub async fn loop_delete_state(
    state: State<'_, DbState>,
    state_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&state_id, "stateId")?;
    let deleted = state.db.delete_loop_state(&state_id)?;
    if !deleted {
        return Err(AppError::not_found(format!(
            "Loop state '{}' not found",
            state_id
        )));
    }
    tracing::info!(state_id = %state_id, "Loop state deleted");
    Ok(IpcResponse::no_content())
}

#[tauri::command]
pub async fn loop_pause(
    state: State<'_, DbState>,
    state_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&state_id, "stateId")?;
    let now = chrono::Utc::now().to_rfc3339();
    let updated = state.db.update_loop_state(
        &state_id,
        Some("paused"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &now,
    )?;
    if !updated {
        return Err(AppError::not_found(format!(
            "Loop state '{}' not found",
            state_id
        )));
    }
    tracing::info!(state_id = %state_id, "Loop paused");
    Ok(IpcResponse::no_content())
}

#[tauri::command]
pub async fn loop_resume(
    state: State<'_, DbState>,
    state_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&state_id, "stateId")?;
    let now = chrono::Utc::now().to_rfc3339();
    let updated = state.db.update_loop_state(
        &state_id,
        Some("idle"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &now,
    )?;
    if !updated {
        return Err(AppError::not_found(format!(
            "Loop state '{}' not found",
            state_id
        )));
    }
    tracing::info!(state_id = %state_id, "Loop resumed");
    Ok(IpcResponse::no_content())
}

#[tauri::command]
pub async fn loop_get_run_logs(
    state: State<'_, DbState>,
    state_id: Option<String>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<LoopRunLogDto>>, AppError> {
    if let Some(ref sid) = state_id {
        validate_id_component(sid, "stateId")?;
    }
    let limit = limit.unwrap_or(100);
    let rows: Vec<LoopRunRow> = if let Some(sid) = state_id {
        state.db.list_loop_runs_by_state(&sid, limit)?
    } else {
        state.db.list_recent_loop_runs(limit)?
    };
    let dtos: Vec<LoopRunLogDto> = rows.into_iter().map(run_row_to_log_dto).collect();
    Ok(IpcResponse::ok(dtos))
}

#[tauri::command]
pub async fn loop_get_patterns(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<LoopPatternDto>>, AppError> {
    let rows = state.db.list_loop_patterns()?;
    let dtos: Vec<LoopPatternDto> = rows
        .into_iter()
        .map(pattern_row_to_dto)
        .collect::<Result<_, _>>()?;
    Ok(IpcResponse::ok(dtos))
}

#[tauri::command]
pub async fn loop_upsert_pattern(
    state: State<'_, DbState>,
    id: Option<String>,
    name: String,
    description: Option<String>,
    goal: Option<String>,
    cadence: Option<String>,
    risk_level: Option<String>,
    phases: Option<Vec<super::types::PhaseDefDto>>,
    human_gates: Option<Vec<String>>,
    cost_config: Option<CostConfigDto>,
    skills_required: Option<Vec<String>>,
    is_active: Option<bool>,
) -> Result<IpcResponse<LoopPatternDto>, AppError> {
    // 校验
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::invalid_input("Pattern name cannot be empty"));
    }
    if name.len() > 255 {
        return Err(AppError::invalid_input("Pattern name too long (max 255 chars)"));
    }
    let risk = risk_level.unwrap_or_else(|| "low".to_string());
    validate_risk_level(&risk)?;

    // 如果提供了 id,检查是否为 builtin(builtin 不可修改)
    if let Some(ref existing_id) = id {
        validate_id_component(existing_id, "id")?;
        if let Some(existing) = state.db.get_loop_pattern(existing_id)? {
            if existing.is_builtin != 0 {
                return Err(AppError::forbidden(format!(
                    "Cannot modify builtin pattern '{}'",
                    existing_id
                )));
            }
        }
    }

    let pattern_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().to_rfc3339();

    let phases_json = phases.map(|p| serde_json::to_string(&p).unwrap_or_else(|_| "[]".to_string()));
    let human_gates_json = human_gates.map(|h| serde_json::to_string(&h).unwrap_or_else(|_| "[]".to_string()));
    let cost_config_json = cost_config.map(|c| serde_json::to_string(&c).unwrap_or_else(|_| "{}".to_string()));
    let skills_json = skills_required.map(|s| serde_json::to_string(&s).unwrap_or_else(|_| "[]".to_string()));

    let row = LoopPatternRow {
        id: pattern_id.clone(),
        name,
        description,
        goal,
        cadence: cadence.unwrap_or_else(|| "manual".to_string()),
        risk_level: risk,
        phases: phases_json,
        human_gates: human_gates_json,
        cost_config: cost_config_json,
        skills_required: skills_json,
        is_active: if is_active.unwrap_or(true) { 1 } else { 0 },
        is_builtin: 0, // user-defined pattern
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.upsert_loop_pattern(&row)?;
    tracing::info!(pattern_id = %pattern_id, "Loop pattern upserted");

    let row = state
        .db
        .get_loop_pattern(&pattern_id)?
        .ok_or_else(|| AppError::internal("Failed to read back upserted pattern"))?;
    let dto = pattern_row_to_dto(row)?;
    Ok(IpcResponse::ok(dto))
}

#[tauri::command]
pub async fn loop_delete_pattern(
    state: State<'_, DbState>,
    pattern_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&pattern_id, "patternId")?;
    let deleted = state.db.delete_loop_pattern(&pattern_id)?;
    if !deleted {
        // 区分"不存在"和"builtin 不可删"
        let existing = state.db.get_loop_pattern(&pattern_id)?;
        if existing.is_some() {
            return Err(AppError::forbidden(format!(
                "Cannot delete builtin pattern '{}'",
                pattern_id
            )));
        }
        return Err(AppError::not_found(format!(
            "Loop pattern '{}' not found",
            pattern_id
        )));
    }
    tracing::info!(pattern_id = %pattern_id, "Loop pattern deleted");
    Ok(IpcResponse::no_content())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_to_status_maps_correctly() {
        assert_eq!(outcome_to_status("fix-proposed"), "success");
        assert_eq!(outcome_to_status("no-op"), "success");
        assert_eq!(outcome_to_status("escalated"), "escalated");
        assert_eq!(outcome_to_status("failed"), "failed");
        assert_eq!(outcome_to_status("running"), "partial");
        assert_eq!(outcome_to_status("report-only"), "partial");
    }

    #[test]
    fn validate_loop_status_rejects_invalid() {
        assert!(validate_loop_status("idle").is_ok());
        assert!(validate_loop_status("invalid").is_err());
    }

    #[test]
    fn validate_readiness_level_rejects_invalid() {
        assert!(validate_readiness_level("L0").is_ok());
        assert!(validate_readiness_level("L3").is_ok());
        assert!(validate_readiness_level("L4").is_err());
    }

    #[test]
    fn validate_risk_level_rejects_invalid() {
        assert!(validate_risk_level("low").is_ok());
        assert!(validate_risk_level("critical").is_err());
    }
}
