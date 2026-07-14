// Loop-Engineering 预算守卫 —— 三档阈值决策。
//
// 决策矩阵(以 audit-revise-loop 为例,daily_cap=1_500_000):
// ┌─────────────────────────────┬─────────────────────────────────┐
// │ 条件                        │ 决策                            │
// ├─────────────────────────────┼─────────────────────────────────┤
// │ 累计 ≥ 100% daily_cap       │ Exit(立即停止,防止超支)         │
// │ 累计 ≥ 80% daily_cap        │ DegradeToReportOnly(只看不修)    │
// │ 累计 ≥ tokens_per_run_cap   │ DegradeToReportOnly(单次超限)    │
// │ 累计 < early_exit_tokens     │ EarlyExit(空 watchlist,跳过)    │
// │ 否则                        │ Allow(正常执行)                │
// └─────────────────────────────┴─────────────────────────────────┘
//
// early_exit 语义(对齐 operating-loops.md 第 4 节):
// - 当循环刚开始、watchlist 为空、预计 token < early_exit_tokens 时直接退出
// - 避免 spawn sub-agent 的固定开销
// - high-cadence pattern(audit-revise)强制 early_exit_required = true
//
// attempt cap(不在此处检查,由 chapter_review_cycle 在循环内自行检查):
// - max_attempts = 3(对齐 failure-modes.md S2 缓解)
// - 超出后 outcome = Escalated

use chrono::Utc;

use crate::infrastructure::db::connection::Database;
use crate::shared::error::AppError;

use super::types::{BudgetCheckResult, LoopPattern, LoopPatternId};

/// 今日 UTC 日期边界(返回 [start_iso, end_iso])
///
/// SQLite TEXT 比较 ISO8601 字符串时,字典序等价于时间序(只要长度一致)。
/// 用 `date >= start AND date < end` 范围扫描 idx_loop_runs_started_at。
fn today_utc_bounds() -> (String, String) {
    let today = Utc::now().date_naive();
    let start = today.format("%Y-%m-%d").to_string();
    let next = today.succ_opt().unwrap_or(today).format("%Y-%m-%d").to_string();
    // started_at 为 ISO8601,前 10 字符即 YYYY-MM-DD
    // 用 LIKE 'YYYY-MM-DD%' 等价于 >= start AND < next_day
    (format!("{}T00:00:00Z", start), format!("{}T00:00:00Z", next))
}

/// 查询今日累计 token 用量(所有 pattern 合计)
///
/// 用于跨 pattern 的全局预算控制。例如 audit-revise + observation 共享
/// 一个总预算池,任一超 80% 都降级。
pub fn daily_token_usage(db: &Database) -> Result<u64, AppError> {
    let (start, end) = today_utc_bounds();
    let conn = db.conn()?;
    let total: Option<i64> = conn
        .query_row(
            "SELECT COALESCE(SUM(total_tokens), 0) FROM loop_runs \
             WHERE started_at >= ?1 AND started_at < ?2",
            rusqlite::params![&start, &end],
            |row| row.get(0),
        )
        .map_err(|e| AppError::internal(format!("Failed to query daily token usage: {}", e)))?;
    Ok(total.unwrap_or(0) as u64)
}

/// 查询今日累计运行次数(用于 attempt cap 全局检查)
pub fn run_count_today(db: &Database, pattern_id: &LoopPatternId) -> Result<u32, AppError> {
    let (start, end) = today_utc_bounds();
    let conn = db.conn()?;
    let count: Option<i64> = conn
        .query_row(
            "SELECT COUNT(*) FROM loop_runs \
             WHERE pattern_id = ?1 AND started_at >= ?2 AND started_at < ?3",
            rusqlite::params![pattern_id.as_str(), &start, &end],
            |row| row.get(0),
        )
        .map_err(|e| AppError::internal(format!("Failed to query run count: {}", e)))?;
    Ok(count.unwrap_or(0) as u32)
}

/// 检查预算,返回三档决策之一。
///
/// 参数:
/// - db:数据库连接(用于查询今日累计)
/// - pattern_id:循环模式 ID(查 daily_cap / per_run_cap / early_exit)
/// - run_tokens_estimate:本次运行预计 token 量(由 caller 估算)
///
/// 决策逻辑(按优先级从高到低):
/// 1. 累计 + 本次 ≥ 100% daily_cap → Exit
/// 2. 累计 ≥ 80% daily_cap → DegradeToReportOnly
/// 3. 本次 + 累计 ≥ tokens_per_run_cap → DegradeToReportOnly
/// 4. 本次 < early_exit_tokens 且 pattern 强制早退 → EarlyExit
/// 5. 默认 → Allow
pub fn check_budget(
    db: &Database,
    pattern_id: &LoopPatternId,
    run_tokens_estimate: u64,
) -> Result<BudgetCheckResult, AppError> {
    let pattern = LoopPattern::get(pattern_id);
    let daily_used = daily_token_usage(db)?;
    let projected = daily_used.saturating_add(run_tokens_estimate);

    // 1. 累计 + 本次 ≥ 100% daily_cap → Exit
    if projected >= pattern.tokens_daily_cap {
        return Ok(BudgetCheckResult::Exit {
            reason: format!(
                "Projected {} tokens ≥ daily cap {} (used {})",
                projected, pattern.tokens_daily_cap, daily_used
            ),
        });
    }

    // 2. 累计 ≥ 80% daily_cap → DegradeToReportOnly
    let degrade_threshold = pattern.tokens_daily_cap * 4 / 5; // 80%
    if daily_used >= degrade_threshold {
        return Ok(BudgetCheckResult::DegradeToReportOnly {
            reason: format!(
                "Daily used {} ≥ 80% of cap {}",
                daily_used, pattern.tokens_daily_cap
            ),
        });
    }

    // 3. 本次 + 累计 ≥ tokens_per_run_cap → DegradeToReportOnly
    if projected >= pattern.tokens_per_run_cap {
        return Ok(BudgetCheckResult::DegradeToReportOnly {
            reason: format!(
                "Projected {} ≥ per-run cap {}",
                projected, pattern.tokens_per_run_cap
            ),
        });
    }

    // 4. 本次 < early_exit_tokens 且 pattern 强制早退 → EarlyExit
    //    语义:watchlist 为空、预计开销很小、且 pattern 标记 early_exit_required
    //    (caller 应当只在 watchlist 为空时传入小值)
    if pattern.early_exit_required && run_tokens_estimate < pattern.early_exit_tokens {
        return Ok(BudgetCheckResult::EarlyExit {
            reason: format!(
                "Run estimate {} < early-exit threshold {} (required for {})",
                run_tokens_estimate,
                pattern.early_exit_tokens,
                pattern_id.as_str()
            ),
        });
    }

    // 5. 默认 → Allow
    Ok(BudgetCheckResult::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        // 使用内存数据库做隔离测试,会自动跑迁移建好 loop_runs 表
        Database::connect_in_memory().expect("in-memory db should init")
    }

    #[test]
    fn today_bounds_are_iso8601() {
        let (start, end) = today_utc_bounds();
        assert!(start.ends_with("T00:00:00Z"));
        assert!(end.ends_with("T00:00:00Z"));
        assert!(start < end);
    }

    #[test]
    fn daily_token_usage_empty_returns_zero() {
        let db = make_in_memory_db();
        let total = daily_token_usage(&db).unwrap();
        assert_eq!(total, 0);
    }

    #[test]
    fn run_count_today_empty_returns_zero() {
        let db = make_in_memory_db();
        let count = run_count_today(&db, &LoopPatternId::AuditReviseLoop).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn check_budget_allow_when_under_caps() {
        let db = make_in_memory_db();
        let result = check_budget(&db, &LoopPatternId::ObservationLoop, 10_000).unwrap();
        assert_eq!(result, BudgetCheckResult::Allow);
    }

    #[test]
    fn check_budget_exit_when_projected_exceeds_daily_cap() {
        let db = make_in_memory_db();
        // observation-loop daily_cap = 300_000
        // 投射一个大于 daily_cap 的量
        let result = check_budget(&db, &LoopPatternId::ObservationLoop, 350_000).unwrap();
        match result {
            BudgetCheckResult::Exit { .. } => {}
            other => panic!("expected Exit, got {:?}", other),
        }
    }

    #[test]
    fn check_budget_early_exit_for_audit_revise_when_small_estimate() {
        let db = make_in_memory_db();
        // audit-revise-loop: early_exit_required=true, early_exit_tokens=5_000
        let result = check_budget(&db, &LoopPatternId::AuditReviseLoop, 1_000).unwrap();
        match result {
            BudgetCheckResult::EarlyExit { .. } => {}
            other => panic!("expected EarlyExit, got {:?}", other),
        }
    }
}
