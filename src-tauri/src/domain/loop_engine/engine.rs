use crate::errors::AppError;
use crate::infra::db::models::{LoopState, LoopRunLog};

pub struct LoopEngine;

impl LoopEngine {
    pub fn run_cycle(state: &LoopState) -> Result<LoopRunLog, AppError> {
        let start = std::time::Instant::now();
        let findings = vec![format!("Loop '{}' completed cycle", state.pattern_id)];

        Ok(LoopRunLog {
            id: uuid::Uuid::new_v4().to_string(),
            loop_state_id: state.id.clone(),
            pattern_id: state.pattern_id.clone(),
            status: "success".to_string(),
            phase_results: vec![],
            tokens_used: 0,
            duration_ms: start.elapsed().as_millis() as i64,
            findings,
            actions_taken: vec![],
            escalations: vec![],
            error_message: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

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
