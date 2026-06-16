use serde::{Deserialize, Serialize};
use crate::errors::AppError;

/// Recovery strategies for agent failures (P14.26)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Simple retry with the same parameters
    Retry,
    /// Simplify the task (reduce complexity)
    Simplify,
    /// Fall back to a simpler model
    FallbackModel,
    /// Request human intervention
    HumanIntervention,
    /// Skip this step and continue
    Skip,
}

/// Recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Maximum retry attempts before escalating
    pub max_retries: usize,
    /// Maximum simplification attempts
    pub max_simplifications: usize,
    /// Fallback model to use when primary fails
    pub fallback_model: Option<String>,
    /// Whether to allow human intervention
    pub allow_human_intervention: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            max_simplifications: 1,
            fallback_model: None,
            allow_human_intervention: false,
        }
    }
}

/// Error recovery manager (P14.26)
pub struct RecoveryManager {
    config: RecoveryConfig,
    retry_count: usize,
    simplification_count: usize,
    used_fallback: bool,
}

impl RecoveryManager {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            retry_count: 0,
            simplification_count: 0,
            used_fallback: false,
        }
    }

    /// Determine the next recovery strategy based on the error
    pub fn next_strategy(&mut self, error: &AppError) -> Option<RecoveryStrategy> {
        // Check error message for patterns instead of matching enum variants
        let error_msg = error.to_string();

        if error_msg.contains("internal") || error_msg.contains("LLM") || error_msg.contains("timeout") {
            // Internal errors: retry first, then simplify
            if self.retry_count < self.config.max_retries {
                self.retry_count += 1;
                Some(RecoveryStrategy::Retry)
            } else if self.simplification_count < self.config.max_simplifications {
                self.simplification_count += 1;
                Some(RecoveryStrategy::Simplify)
            } else if !self.used_fallback && self.config.fallback_model.is_some() {
                self.used_fallback = true;
                Some(RecoveryStrategy::FallbackModel)
            } else if self.config.allow_human_intervention {
                Some(RecoveryStrategy::HumanIntervention)
            } else {
                None
            }
        } else if error_msg.contains("bad_request") || error_msg.contains("invalid") {
            // Bad request: simplify the task
            if self.simplification_count < self.config.max_simplifications {
                self.simplification_count += 1;
                Some(RecoveryStrategy::Simplify)
            } else {
                Some(RecoveryStrategy::Retry)
            }
        } else if error_msg.contains("not_found") {
            // Not found: skip or retry
            Some(RecoveryStrategy::Skip)
        } else {
            // Other errors: retry
            if self.retry_count < self.config.max_retries {
                self.retry_count += 1;
                Some(RecoveryStrategy::Retry)
            } else {
                None
            }
        }
    }

    /// Reset the recovery manager for a new operation
    pub fn reset(&mut self) {
        self.retry_count = 0;
        self.simplification_count = 0;
        self.used_fallback = false;
    }
}

/// Simplification rules for reducing task complexity
pub struct SimplificationRules;

impl SimplificationRules {
    /// Simplify a chapter plan by reducing scope
    pub fn simplify_plan(plan: &str) -> String {
        // Remove complex sub-plots, keep only main plot
        let simplified = plan
            .lines()
            .filter(|line| {
                !line.contains("支线") && !line.contains("subplot") && !line.contains("次要")
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("{}\n\n[注：已简化任务复杂度]", simplified)
    }

    /// Simplify writing instructions
    pub fn simplify_instructions(instructions: &str) -> String {
        format!(
            "{}\n\n要求：\n- 只写核心场景\n- 减少对话数量\n- 简化描写",
            instructions
        )
    }
}
