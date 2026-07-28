//! ═══════════════════════════════════════════════════════════════════════════
//! Loop Engine - 循环模式、预算、运行记录子系统
//! ═══════════════════════════════════════════════════════════════════════════

pub mod types;
pub mod budget;
pub mod prompts;

pub use types::{
    BUILTIN_PATTERNS, BudgetCheckResult, LoopOutcome, LoopPattern, LoopPatternId, LoopRun,
};
pub use budget::{check_budget, daily_token_usage, run_count_today};
