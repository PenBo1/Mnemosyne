use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use super::error_classifier::{classify_api_error, FailoverReason};

/// Recovery strategies for agent failures (P14.26)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Simple retry with the same parameters
    Retry,
    /// Simplify the task (reduce complexity)
    Simplify,
    /// Fall back to a simpler model
    FallbackModel,
    /// Compress context (上下文溢出时)
    CompressContext,
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
    compressed_context: bool,
}

impl RecoveryManager {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            retry_count: 0,
            simplification_count: 0,
            used_fallback: false,
            compressed_context: false,
        }
    }

    /// Determine the next recovery strategy based on the error.
    /// Uses ErrorClassifier for intelligent recovery decisions.
    pub fn next_strategy(&mut self, error: &AppError) -> Option<RecoveryStrategy> {
        let error_msg = error.to_string();

        // Use the error classifier for intelligent classification
        let classified = classify_api_error(&error_msg, None, "", "");

        // Log classification for debugging
        tracing::debug!(
            reason = ?classified.reason,
            retryable = classified.retryable,
            should_compress = classified.should_compress,
            should_fallback = classified.should_fallback,
            "Error classified for recovery"
        );

        // 1. 不可重试的错误 — 直接失败
        if !classified.retryable {
            return match classified.reason {
                FailoverReason::Auth | FailoverReason::AuthPermanent => {
                    if self.config.allow_human_intervention {
                        Some(RecoveryStrategy::HumanIntervention)
                    } else {
                        None
                    }
                }
                FailoverReason::Billing => {
                    // 尝试降级到备用模型
                    if !self.used_fallback && self.config.fallback_model.is_some() {
                        self.used_fallback = true;
                        Some(RecoveryStrategy::FallbackModel)
                    } else if self.config.allow_human_intervention {
                        Some(RecoveryStrategy::HumanIntervention)
                    } else {
                        None
                    }
                }
                FailoverReason::ModelNotFound => {
                    if !self.used_fallback && self.config.fallback_model.is_some() {
                        self.used_fallback = true;
                        Some(RecoveryStrategy::FallbackModel)
                    } else {
                        Some(RecoveryStrategy::Skip)
                    }
                }
                FailoverReason::ContentPolicyBlocked => {
                    // 内容策略拦截 — 尝试简化任务
                    if self.simplification_count < self.config.max_simplifications {
                        self.simplification_count += 1;
                        Some(RecoveryStrategy::Simplify)
                    } else {
                        None
                    }
                }
                FailoverReason::FormatError => {
                    // 格式错误 — 尝试简化
                    if self.simplification_count < self.config.max_simplifications {
                        self.simplification_count += 1;
                        Some(RecoveryStrategy::Simplify)
                    } else {
                        None
                    }
                }
                _ => None,
            };
        }

        // 2. 上下文溢出 — 压缩上下文
        if classified.should_compress && !self.compressed_context {
            self.compressed_context = true;
            return Some(RecoveryStrategy::CompressContext);
        }

        // 3. 可重试的错误 — 根据类型选择策略
        match classified.reason {
            FailoverReason::RateLimit | FailoverReason::Overloaded => {
                // 限流/过载 — 简单重试
                if self.retry_count < self.config.max_retries {
                    self.retry_count += 1;
                    Some(RecoveryStrategy::Retry)
                } else if self.simplification_count < self.config.max_simplifications {
                    self.simplification_count += 1;
                    Some(RecoveryStrategy::Simplify)
                } else {
                    None
                }
            }
            FailoverReason::ServerError | FailoverReason::Timeout => {
                // 服务器错误/超时 — 重试
                if self.retry_count < self.config.max_retries {
                    self.retry_count += 1;
                    Some(RecoveryStrategy::Retry)
                } else if !self.used_fallback && self.config.fallback_model.is_some() {
                    self.used_fallback = true;
                    Some(RecoveryStrategy::FallbackModel)
                } else {
                    None
                }
            }
            _ => {
                // 其他可重试错误 — 重试
                if self.retry_count < self.config.max_retries {
                    self.retry_count += 1;
                    Some(RecoveryStrategy::Retry)
                } else {
                    None
                }
            }
        }
    }

    /// 重置恢复状态（新的 Pipeline 阶段开始时调用）
    pub fn reset(&mut self) {
        self.retry_count = 0;
        self.simplification_count = 0;
        // 注意: used_fallback 和 compressed_context 不重置 — 跨阶段保持
    }

    /// 完全重置（新的 Pipeline 开始时调用）
    pub fn reset_all(&mut self) {
        self.retry_count = 0;
        self.simplification_count = 0;
        self.used_fallback = false;
        self.compressed_context = false;
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
