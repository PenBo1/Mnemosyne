//! ═══════════════════════════════════════════════════════════════════════════
//! Types - 循环引擎类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 与前端类型对齐的 DTO 定义，使用 camelCase 序列化。
//! DB Row 类型使用 snake_case，由本模块负责转换。

use serde::{Deserialize, Serialize};

/// 循环状态(per-novel 的循环实例)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopStateDto {
    pub id: String,
    pub novel_id: String,
    pub pattern_id: String,
    /// "idle" / "running" / "paused" / "error"
    pub status: String,
    /// "L0" / "L1" / "L2" / "L3"
    pub readiness_level: String,
    /// 循环状态快照(自由 JSON)
    pub state_payload: serde_json::Value,
    pub config: LoopConfigDto,
    pub token_usage_today: u64,
    pub token_cap_daily: u64,
    pub last_run_at: Option<String>,
    pub last_run_result: Option<LoopRunResultDto>,
    pub created_at: String,
    pub updated_at: String,
}

/// 循环配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopConfigDto {
    pub cadence: String,
    pub denylist: Vec<String>,
    pub human_gates: Vec<String>,
    pub max_retries: u32,
}

/// 最近一次运行结果摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopRunResultDto {
    pub findings: Vec<String>,
    pub actions: Vec<String>,
    pub escalations: Vec<String>,
}

/// 循环模式定义(builtin + user-defined)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopPatternDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub goal: Option<String>,
    /// "manual" / "hourly" / "daily" / "per-chapter"
    pub cadence: String,
    /// "low" / "medium" / "high"
    pub risk_level: String,
    pub phases: Vec<PhaseDefDto>,
    pub human_gates: Vec<String>,
    pub cost_config: CostConfigDto,
    pub skills_required: Vec<String>,
    pub is_active: bool,
    /// builtin pattern 不可删除
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseDefDto {
    pub name: String,
    pub description: String,
    /// "discover" / "deliver" / "verify" / "persist" / "schedule"
    #[serde(rename = "type")]
    pub phase_type: String,
}

/// 成本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostConfigDto {
    pub tokens_noop: u64,
    pub tokens_report: u64,
    pub tokens_action: u64,
    pub daily_cap: u64,
    pub early_exit_required: bool,
}

/// 运行日志(对应前端 LoopRunLog)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopRunLogDto {
    pub id: String,
    pub loop_state_id: Option<String>,
    pub pattern_id: String,
    /// "success" / "partial" / "failed" / "escalated"
    pub status: String,
    pub phase_results: Vec<PhaseResultDto>,
    pub tokens_used: u64,
    pub duration_ms: u64,
    pub findings: Vec<String>,
    pub actions_taken: Vec<String>,
    pub escalations: Vec<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// 阶段执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseResultDto {
    pub phase: String,
    pub status: String,
    pub output: String,
    pub duration_ms: u64,
}

// ── IPC 请求类型 ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLoopStateRequest {
    pub pattern_id: String,
    pub readiness_level: Option<String>,
    pub config: Option<LoopConfigDto>,
    pub token_cap_daily: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLoopStateRequest {
    pub status: Option<String>,
    pub readiness_level: Option<String>,
    pub config: Option<LoopConfigDto>,
    pub token_cap_daily: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertLoopPatternRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub goal: Option<String>,
    pub cadence: Option<String>,
    pub risk_level: Option<String>,
    pub phases: Option<Vec<PhaseDefDto>>,
    pub human_gates: Option<Vec<String>>,
    pub cost_config: Option<CostConfigDto>,
    pub skills_required: Option<Vec<String>>,
    pub state_schema: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

/// 将 loop_runs.outcome 映射为前端 LoopRunLog.status
///
/// outcome(后端) → status(前端):
/// - "running" → "partial"(运行中不返回,兜底)
/// - "report-only" → "partial"
/// - "fix-proposed" → "success"
/// - "escalated" → "escalated"
/// - "no-op" → "success"
/// - "failed" → "failed"
pub fn outcome_to_status(outcome: &str) -> &'static str {
    match outcome {
        "fix-proposed" | "no-op" => "success",
        "escalated" => "escalated",
        "failed" => "failed",
        _ => "partial",
    }
}
