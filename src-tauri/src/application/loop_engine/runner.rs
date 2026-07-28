//! ═══════════════════════════════════════════════════════════════════════════
//! Runner - 循环执行器
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 执行单次 Loop tick，管理状态转换：
//! - 执行循环阶段（discover/deliver/verify/persist）
//! - 计算动态节奏（下次唤醒时间）
//! - 更新 LoopState 和 LoopRunLog
//! - 预算检查

use std::sync::Arc;

use chrono::{DateTime, Timelike, Utc};

use crate::core::agent::loop_engine::types::{LoopOutcome, LoopPattern, LoopPatternId, LoopRun};
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::loop_run::LoopRunRow;
use crate::infrastructure::db::stores::loop_state::LoopStateRow;
use crate::shared::error::AppError;

use super::types::{LoopConfigDto, LoopRunResultDto, PhaseResultDto};

#[derive(Debug)]
struct BudgetCheckResult {
    allowed: bool,
    reason: String,
}

/// Loop 执行器
pub struct LoopRunner {
    db: Database,
    state_id: String,
    pattern_id: LoopPatternId,
}

impl LoopRunner {
    pub fn new(db: Database, state_id: String, pattern_id: LoopPatternId) -> Self {
        Self {
            db,
            state_id,
            pattern_id,
        }
    }

    /// 执行单次 Loop tick
    ///
    /// 运行流程:
    /// 1. 加载 LoopState 和 LoopPattern
    /// 2. 检查预算(是否允许执行)
    /// 3. 执行 phases(discover/deliver/verify/persist)
    /// 4. 更新 LoopRunLog
    /// 5. 计算下次唤醒时间
    pub async fn run(
        &self,
        executor: Arc<dyn LoopExecutor + Send + Sync>,
    ) -> Result<LoopRunResult, AppError> {
        let state_row = self
            .db
            .get_loop_state(&self.state_id)?
            .ok_or_else(|| AppError::not_found(format!("Loop state '{}' not found", self.state_id)))?;

        if state_row.status != "idle" {
            return Err(AppError::conflict(format!(
                "Loop state '{}' is not idle (current: {})",
                self.state_id, state_row.status
            )));
        }

        let pattern = LoopPattern::get(&self.pattern_id);
        let config: LoopConfigDto = self.parse_config(&state_row.config)?;

        let budget_result = self.check_budget_simple(
            state_row.token_usage_today as u64,
            pattern.tokens_daily_cap,
        );

        if !budget_result.allowed {
            tracing::info!(
                state_id = %self.state_id,
                reason = ?budget_result.reason,
                "Loop skipped due to budget constraint"
            );
            return Ok(LoopRunResult {
                outcome: LoopOutcome::NoOp,
                findings: vec![],
                actions: vec![],
                escalations: vec![],
                tokens_used: 0,
                duration_ms: 0,
            });
        }

        let now = Utc::now();
        let mut run_record = LoopRun::new(
            self.pattern_id.clone(),
            None,
            None,
        );

        self.update_state_status("running")?;

        let start = std::time::Instant::now();
        let result = executor.execute(&state_row, pattern, &config).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        let (outcome, findings, actions, escalations, tokens_used) = match result {
            Ok(exec_result) => {
                run_record.add_usage(exec_result.prompt_tokens, exec_result.completion_tokens);
                (
                    exec_result.outcome,
                    exec_result.findings,
                    exec_result.actions,
                    exec_result.escalations,
                    run_record.total_tokens,
                )
            }
            Err(e) => {
                tracing::error!(error = %e, state_id = %self.state_id, "Loop execution failed");
                (LoopOutcome::Failed, vec![], vec![], vec![], 0)
            }
        };

        run_record.finish(outcome);
        let run_id = run_record.run_id.clone();

        self.persist_run_log(&run_record, &findings, &actions, &escalations, duration_ms)?;

        let updated_usage = state_row.token_usage_today as u64 + tokens_used;
        self.update_state_after_run(
            &outcome,
            updated_usage,
            &findings,
            &actions,
            &escalations,
            &now,
        )?;

        tracing::info!(
            state_id = %self.state_id,
            run_id = %run_id,
            outcome = ?outcome,
            tokens_used = tokens_used,
            duration_ms = duration_ms,
            "Loop tick completed"
        );

        Ok(LoopRunResult {
            outcome,
            findings,
            actions,
            escalations,
            tokens_used,
            duration_ms,
        })
    }

    /// 计算下次唤醒时间(动态节奏)
    ///
    /// 节奏规则(对齐 loop-engineering/operating-loops.md):
    /// - "manual": 无自动唤醒(返回 None)
    /// - "hourly": 下一个整点
    /// - "daily": 明天同一时刻
    /// - "per-chapter": 基于上一章完成时间,每 30 分钟一次
    pub fn calculate_next_wake(&self, cadence: &str, last_run: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        match cadence {
            "manual" => None,
            "hourly" => {
                let now = Utc::now();
                let next_hour = now + chrono::Duration::hours(1);
                Some(next_hour.with_minute(0).unwrap_or(next_hour).with_second(0).unwrap_or(next_hour))
            }
            "daily" => {
                let now = Utc::now();
                Some(now + chrono::Duration::days(1))
            }
            "per-chapter" => {
                let base = last_run.unwrap_or_else(Utc::now);
                Some(base + chrono::Duration::minutes(30))
            }
            _ => {
                tracing::warn!(cadence = cadence, "Unknown cadence, defaulting to manual");
                None
            }
        }
    }

    fn parse_config(&self, json: &Option<String>) -> Result<LoopConfigDto, AppError> {
        match json {
            Some(s) if !s.is_empty() => {
                serde_json::from_str(s).map_err(|e| AppError::invalid_format(format!("Invalid config JSON: {}", e)))
            }
            _ => Ok(LoopConfigDto {
                cadence: "manual".to_string(),
                denylist: vec![],
                human_gates: vec![],
                max_retries: 3,
            }),
        }
    }

    fn update_state_status(&self, status: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        self.db.update_loop_state(
            &self.state_id,
            Some(status),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &now,
        )?;
        Ok(())
    }

    fn update_state_after_run(
        &self,
        outcome: &LoopOutcome,
        token_usage: u64,
        findings: &[String],
        actions: &[String],
        escalations: &[String],
        last_run_at: &DateTime<Utc>,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result_json = serde_json::to_string(&LoopRunResultDto {
            findings: findings.to_vec(),
            actions: actions.to_vec(),
            escalations: escalations.to_vec(),
        })
        .map_err(|e| AppError::invalid_format(format!("Failed to serialize result: {}", e)))?;

        let status = match outcome {
            LoopOutcome::Escalated => "paused",
            LoopOutcome::Failed => "error",
            _ => "idle",
        };

        self.db.update_loop_state(
            &self.state_id,
            Some(status),
            None,
            None,
            None,
            Some(token_usage as i64),
            None,
            Some(&result_json),
            Some(&last_run_at.to_rfc3339()),
            &now,
        )?;

        Ok(())
    }

    fn persist_run_log(
        &self,
        run: &LoopRun,
        findings: &[String],
        actions: &[String],
        escalations: &[String],
        duration_ms: u64,
    ) -> Result<(), AppError> {
        let phase_results = vec![
            PhaseResultDto {
                phase: "execute".to_string(),
                status: run.outcome.as_str().to_string(),
                output: format!("tokens={}, attempts={}", run.total_tokens, run.attempts),
                duration_ms,
            },
        ];

        let row = LoopRunRow {
            run_id: run.run_id.clone(),
            loop_state_id: Some(self.state_id.clone()),
            pattern_id: run.pattern_id.as_str().to_string(),
            book_id: run.book_id.clone(),
            chapter_number: run.chapter_number,
            started_at: run.started_at.clone(),
            ended_at: run.ended_at.clone(),
            duration_s: run.duration_s,
            outcome: run.outcome.as_str().to_string(),
            items_found: run.items_found,
            actions_taken: run.actions_taken,
            escalations: run.escalations,
            tokens_estimate: run.tokens_estimate,
            prompt_tokens: run.prompt_tokens,
            completion_tokens: run.completion_tokens,
            total_tokens: run.total_tokens,
            attempts: run.attempts,
            notes: run.notes.clone(),
            phase_results_json: Some(serde_json::to_string(&phase_results).unwrap_or_else(|_| "[]".to_string())),
            findings_json: Some(serde_json::to_string(findings).unwrap_or_else(|_| "[]".to_string())),
            actions_json: Some(serde_json::to_string(actions).unwrap_or_else(|_| "[]".to_string())),
            escalations_json: Some(serde_json::to_string(escalations).unwrap_or_else(|_| "[]".to_string())),
            error_message: None,
        };

        self.db.insert_loop_run(&row)?;
        Ok(())
    }

    fn check_budget_simple(&self, used_today: u64, daily_cap: u64) -> BudgetCheckResult {
        if used_today >= daily_cap {
            BudgetCheckResult {
                allowed: false,
                reason: format!("Daily cap reached: {} >= {}", used_today, daily_cap),
            }
        } else {
            BudgetCheckResult {
                allowed: true,
                reason: String::new(),
            }
        }
    }
}

/// Loop 执行结果
#[derive(Debug)]
pub struct LoopRunResult {
    pub outcome: LoopOutcome,
    pub findings: Vec<String>,
    pub actions: Vec<String>,
    pub escalations: Vec<String>,
    pub tokens_used: u64,
    pub duration_ms: u64,
}

/// Loop 执行器 trait(由 AgentEngine 实现)
#[async_trait::async_trait]
pub trait LoopExecutor {
    async fn execute(
        &self,
        state: &LoopStateRow,
        pattern: &LoopPattern,
        config: &LoopConfigDto,
    ) -> Result<ExecutorResult, AppError>;
}

/// 执行器返回结果
#[derive(Debug)]
pub struct ExecutorResult {
    pub outcome: LoopOutcome,
    pub findings: Vec<String>,
    pub actions: Vec<String>,
    pub escalations: Vec<String>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // 修复：Database 实际方法为 connect_in_memory（cfg(test) 下可用），历史测试误写为 new_in_memory

    #[test]
    fn calculate_next_wake_manual_returns_none() {
        let runner = LoopRunner::new(Database::connect_in_memory().unwrap(), "test".to_string(), LoopPatternId::ObservationLoop);
        assert!(runner.calculate_next_wake("manual", None).is_none());
    }

    #[test]
    fn calculate_next_wake_hourly_returns_next_hour() {
        let runner = LoopRunner::new(Database::connect_in_memory().unwrap(), "test".to_string(), LoopPatternId::ObservationLoop);
        let next = runner.calculate_next_wake("hourly", None);
        assert!(next.is_some());
        let next = next.unwrap();
        let now = Utc::now();
        assert!(next > now);
        assert!(next < now + chrono::Duration::hours(2));
    }

    #[test]
    fn calculate_next_wake_daily_returns_tomorrow() {
        let runner = LoopRunner::new(Database::connect_in_memory().unwrap(), "test".to_string(), LoopPatternId::ObservationLoop);
        let next = runner.calculate_next_wake("daily", None);
        assert!(next.is_some());
        let next = next.unwrap();
        let now = Utc::now();
        assert!(next > now);
        assert!(next >= now + chrono::Duration::hours(23));
    }
}