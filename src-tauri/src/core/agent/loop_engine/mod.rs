// Agent Loop-Engineering 子系统 —— 循环模式、预算、运行记录。
//
// 三个核心模块:
// - types:LoopPattern / LoopRun / LoopOutcome / BudgetCheckResult
// - budget:check_budget() 实时预算检查(今日累计 vs daily_cap)
// - store:SQLite CRUD(loop_runs 表,append-only + GC 30 天)
//
// 集成点:
// - chapter_review_cycle 在 audit→revise 循环开始/结束/每次 add_usage 时调用
// - 其他循环(observation/consolidation)通过 store.append_run() 记录

pub mod types;
pub mod budget;
pub mod prompts;

pub use types::{
    BUILTIN_PATTERNS, BudgetCheckResult, LoopOutcome, LoopPattern, LoopPatternId, LoopRun,
};
pub use budget::{check_budget, daily_token_usage, run_count_today};
