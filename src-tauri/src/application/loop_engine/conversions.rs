//! ═══════════════════════════════════════════════════════════════════════════
//! Conversions - Row 与 DTO 转换逻辑
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 负责 infrastructure 层 Row 类型到 application 层 DTO 类型的转换。
//! 这些函数编排 JSON 解析、默认值填充和类型映射。

use crate::infrastructure::db::stores::loop_pattern::LoopPatternRow;
use crate::infrastructure::db::stores::loop_run::LoopRunRow;
use crate::infrastructure::db::stores::loop_state::LoopStateRow;
use crate::shared::error::AppError;
use serde_json::json;

use super::types::{
    outcome_to_status, CostConfigDto, LoopConfigDto, LoopPatternDto, LoopRunLogDto,
    LoopRunResultDto, LoopStateDto,
};

// ── 默认值辅助函数 ────────────────────────────────────────────────────────

/// 生成默认循环配置
pub fn default_config() -> LoopConfigDto {
    LoopConfigDto {
        cadence: "manual".to_string(),
        denylist: Vec::new(),
        human_gates: Vec::new(),
        max_retries: 3,
    }
}

// ── JSON 解析辅助函数 ────────────────────────────────────────────────────────

/// 解析配置 JSON
pub fn parse_config(json: &Option<String>) -> Result<LoopConfigDto, AppError> {
    match json {
        Some(s) if !s.is_empty() => {
            serde_json::from_str(s).map_err(|e| AppError::invalid_format(format!("Invalid config JSON: {}", e)))
        }
        _ => Ok(default_config()),
    }
}

/// 解析状态载荷 JSON
pub fn parse_state_payload(json: &Option<String>) -> serde_json::Value {
    match json {
        Some(s) if !s.is_empty() => {
            serde_json::from_str(s).unwrap_or_else(|_| json!({}))
        }
        _ => json!({}),
    }
}

/// 解析运行结果 JSON
pub fn parse_run_result(json: &Option<String>) -> Option<LoopRunResultDto> {
    json.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            serde_json::from_str(s).ok()
        }
    })
}

// ── Row → DTO 转换函数 ────────────────────────────────────────────────────────

/// 将 LoopStateRow 转换为 LoopStateDto
pub fn state_row_to_dto(row: LoopStateRow) -> Result<LoopStateDto, AppError> {
    Ok(LoopStateDto {
        id: row.id,
        novel_id: row.novel_id,
        pattern_id: row.pattern_id,
        status: row.status,
        readiness_level: row.readiness_level,
        state_payload: parse_state_payload(&row.state_payload),
        config: parse_config(&row.config)?,
        token_usage_today: row.token_usage_today as u64,
        token_cap_daily: row.token_cap_daily as u64,
        last_run_at: row.last_run_at,
        last_run_result: parse_run_result(&row.last_run_result),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// 将 LoopPatternRow 转换为 LoopPatternDto
pub fn pattern_row_to_dto(row: LoopPatternRow) -> Result<LoopPatternDto, AppError> {
    let phases: Vec<_> = row
        .phases
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let human_gates: Vec<String> = row
        .human_gates
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let cost_config: CostConfigDto = row
        .cost_config
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(CostConfigDto {
            tokens_noop: 0,
            tokens_report: 0,
            tokens_action: 0,
            daily_cap: 0,
            early_exit_required: false,
        });
    let skills_required: Vec<String> = row
        .skills_required
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Ok(LoopPatternDto {
        id: row.id,
        name: row.name,
        description: row.description,
        goal: row.goal,
        cadence: row.cadence,
        risk_level: row.risk_level,
        phases,
        human_gates,
        cost_config,
        skills_required,
        is_active: row.is_active != 0,
        is_builtin: row.is_builtin != 0,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// 将 LoopRunRow 转换为 LoopRunLogDto
pub fn run_row_to_log_dto(row: LoopRunRow) -> LoopRunLogDto {
    let findings: Vec<String> = row
        .findings_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let actions_taken: Vec<String> = row
        .actions_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let escalations: Vec<String> = row
        .escalations_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let phase_results: Vec<_> = row
        .phase_results_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    LoopRunLogDto {
        id: row.run_id,
        loop_state_id: row.loop_state_id,
        pattern_id: row.pattern_id,
        status: outcome_to_status(&row.outcome).to_string(),
        phase_results,
        tokens_used: row.total_tokens,
        duration_ms: row.duration_s.unwrap_or(0) * 1000,
        findings,
        actions_taken,
        escalations,
        error_message: row.error_message.or(row.notes),
        created_at: row.started_at,
    }
}