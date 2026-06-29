use crate::shared::errors::AppError;
use crate::infrastructure::db::models::{LoopState, LoopRunLog};

/// Loop Engine —— 周期性扫描小说质量、依赖一致性、伏笔检测的循环工程系统。
///
/// 当前状态：**未实现**。
///
/// 设计意图（见 patterns.rs 的 7 个内置 pattern）：
/// - daily-triage / chapter-quality-check / dependency-audit
/// - pipeline-health-monitor / token-budget-watcher
/// - character-consistency-checker / plot-hole-detector
///
/// 每个 pattern 有 phases（discover/audit/verify/report/escalate），
/// 需要调用 auditor/observer/reviser 等 agent 执行真实扫描。
///
/// 为避免"假接口"污染数据库（写入假的 loop_run_log），
/// run_cycle 显式返回 not_implemented 错误，让前端诚实反馈"功能开发中"。
pub struct LoopEngine;

impl LoopEngine {
    /// 执行一次循环周期。
    ///
    /// 未实现：返回 `AppError::not_implemented`。
    /// 完整实现需按 pattern.phases 编排 agent 调用、生成 findings/actions/escalations、
    /// 累计 token 消耗并检查预算。
    pub fn run_cycle(_state: &LoopState) -> Result<LoopRunLog, AppError> {
        Err(AppError::not_implemented(
            "Loop engine cycle execution is not yet implemented. \
             Pattern execution requires auditor/observer agent orchestration.",
        ))
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
