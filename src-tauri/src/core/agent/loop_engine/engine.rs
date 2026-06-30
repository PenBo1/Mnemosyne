use crate::shared::errors::AppError;
use crate::infrastructure::db::models::{LoopState, LoopRunLog, LoopPattern};

/// Loop Engine —— 周期性扫描小说质量、依赖一致性、伏笔检测的循环工程系统。
///
/// 当前实现：**noop 模式**。
///
/// 设计意图（见 patterns.rs 的 7 个内置 pattern）：
/// - daily-triage / chapter-quality-check / dependency-audit
/// - pipeline-health-monitor / token-budget-watcher
/// - character-consistency-checker / plot-hole-detector
///
/// 每个 pattern 有 phases（discover/audit/verify/report/escalate）。
/// 完整实现需要调用 auditor/observer/reviser 等 agent 执行真实扫描。
///
/// 为避免"假接口"污染数据库（编造 findings/actions），当前 run_cycle 显式以
/// `completed_noop` 状态运行：phase 结构性执行并记录，但不调用 agent。
/// 这让前端能渲染真实的 phase 时间线，同时诚实告知用户"agent 编排未接入"。
pub struct LoopEngine;

impl LoopEngine {
    /// 执行一次循环周期。
    ///
    /// - 遍历 `pattern.phases`，记录每个 phase 为 `executed_noop`
    /// - 从 `pattern.cost_config.tokens_noop` 提取本次 noop 消耗（默认 500）
    /// - 返回 `status: "completed_noop"` 的 LoopRunLog
    ///
    /// 不调用任何 agent，不编造 findings。agent 编排接入后，
    /// 把 phase 执行替换为真实 agent 调用即可。
    pub fn run_cycle(state: &LoopState, pattern: &LoopPattern) -> Result<LoopRunLog, AppError> {
        let started = std::time::Instant::now();
        let now = chrono::Utc::now().to_rfc3339();
        let log_id = uuid::Uuid::new_v4().to_string();

        // 从 cost_config 提取 noop token 成本（无配置则默认 500）
        let tokens_noop = pattern
            .cost_config
            .get("tokens_noop")
            .and_then(|v| v.as_i64())
            .unwrap_or(500);

        // 遍历 phases，每个 phase 标记为 executed_noop
        // 不调用 agent，但记录结构性执行，便于前端渲染 phase 时间线
        let phase_results: Vec<serde_json::Value> = pattern
            .phases
            .iter()
            .map(|phase| {
                let name = phase
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                serde_json::json!({
                    "name": name,
                    "status": "executed_noop",
                    "reason": "agent orchestration not wired yet"
                })
            })
            .collect();

        let phase_count = phase_results.len();
        let duration_ms = started.elapsed().as_millis() as i64;

        Ok(LoopRunLog {
            id: log_id,
            loop_state_id: state.id.clone(),
            pattern_id: pattern.id.clone(),
            status: "completed_noop".to_string(),
            phase_results,
            tokens_used: tokens_noop,
            duration_ms,
            findings: vec![format!(
                "{} cycle executed in noop mode; {} phases processed; agent orchestration not wired",
                pattern.name, phase_count
            )],
            actions_taken: vec![],
            escalations: vec![],
            error_message: None,
            created_at: now,
        })
    }

    /// 检查 token 预算状态（已实现，纯计算无副作用）。
    pub fn check_budget(state: &LoopState) -> BudgetStatus {
        let used = state.token_usage_today;
        let cap = state.token_cap_daily;
        let remaining = cap.saturating_sub(used);
        let usage_percent = if cap > 0 {
            (used as f64 / cap as f64 * 100.0) as i64
        } else {
            0
        };

        BudgetStatus {
            used,
            cap,
            remaining,
            usage_percent,
            exceeded: used >= cap,
        }
    }
}

pub struct BudgetStatus {
    pub used: i64,
    pub cap: i64,
    pub remaining: i64,
    pub usage_percent: i64,
    pub exceeded: bool,
}
