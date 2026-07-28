//! ═══════════════════════════════════════════════════════════════════════════
//! Compressor Summary - LLM 摘要抽象
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 将 LLM 调用从压缩器中解耦, 便于单元测试用 mock 实现替换真实 ProviderRegistry。

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::agent::effort::EffortLevel;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::infrastructure::llm::types::Message;
use crate::shared::error::status;
use crate::shared::error::AppError;

// ── LLM 抽象 Trait ──────────────────────────────────────────────────────────

/// 摘要 LLM 调用抽象
/// 
/// 将 LLM 调用从压缩器中解耦, 便于单元测试用 mock 实现替换。
#[async_trait]
pub trait SummaryLlm: Send + Sync {
    /// 调用 LLM 生成摘要
    /// 
    /// # 参数
    /// - `system`: 系统提示词
    /// - `user`: 用户消息
    /// 
    /// # 返回值
    /// 返回纯文本响应
    async fn summarize(&self, system: &str, user: &str) -> Result<String, AppError>;
}

/// 基于 ProviderRegistry 的 SummaryLlm 实现
pub struct ProviderRegistrySummaryLlm {
    registry: Arc<ProviderRegistry>,
}

impl ProviderRegistrySummaryLlm {
    /// 创建实例
    pub fn new(registry: Arc<ProviderRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl SummaryLlm for ProviderRegistrySummaryLlm {
    async fn summarize(&self, system: &str, user: &str) -> Result<String, AppError> {
        let config = self
            .registry
            .active_model_config()
            .ok_or_else(AppError::no_active_model)?;
        let model = config.model.clone();
        let provider = self.registry.active_provider()?;
        let max_tokens = EffortLevel::default().params().max_tokens_per_call;
        let response = provider
            .complete(
                &model,
                system,
                &[Message {
                    role: "user".to_string(),
                    content: user.to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
                max_tokens,
            )
            .await?;
        Ok(response)
    }
}

// ── 结构化摘要 ────────────────────────────────────────────────────────────

/// Phase 3 LLM 生成的结构化摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ContextSummary {
    /// 活动任务
    pub(super) active_task: String,
    /// 已完成操作
    pub(super) completed_actions: Vec<String>,
    /// 关键决策
    pub(super) key_decisions: Vec<String>,
    /// 待处理问题
    pub(super) pending_questions: Vec<String>,
    /// 相关上下文
    pub(super) relevant_context: String,
}

impl ContextSummary {
    /// 渲染为摘要消息内容
    /// 
    /// 组装顺序:
    /// 1. SUMMARY_MARKER: 供下游扫描识别压缩摘要
    /// 2. SUMMARY_HIJACK_GUARD: 防 hijack 声明
    /// 3. SUMMARY_PREFIX: handoff 说明
    /// 4. body: 结构化 JSON 内容
    pub(super) fn render(&self) -> String {
        let body = match serde_json::to_string_pretty(self) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "ContextSummary serialize failed");
                format!(
                    "{{\"active_task\":\"{}\",\"relevant_context\":\"serialization failed\"}}",
                    self.active_task.replace('"', "'")
                )
            }
        };
        format!(
            "{}\n{}\n{}\n{}",
            super::SUMMARY_MARKER,
            super::SUMMARY_HIJACK_GUARD,
            super::SUMMARY_PREFIX,
            body
        )
    }
}

// ── 摘要辅助函数 ──────────────────────────────────────────────────────────

/// 扫描 messages 找到最近的 [CONTEXT_SUMMARY] 前缀消息
/// 
/// 用于 Phase 3 LLM summarization 时复用上一次压缩的 summary。
pub(super) fn find_latest_context_summary(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|m| m.content.starts_with(super::SUMMARY_MARKER))
        .map(|m| m.content.clone())
}

/// 从最近 user turns 推断 focus topic
/// 
/// 取最近 3 条 user 消息中最早一条的前 50 字符作为 focus topic。
pub(super) fn derive_auto_focus_topic(messages: &[Message]) -> Option<String> {
    let user_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == "user")
        .map(|(i, _)| i)
        .collect();
    if user_indices.is_empty() {
        return None;
    }
    let start = user_indices.len().saturating_sub(3);
    let first_idx = user_indices[start];
    let topic: String = messages[first_idx].content.chars().take(50).collect();
    let trimmed = topic.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 判断错误是否为 transient (网络/超时)
pub(super) fn is_transient_error(err: &AppError) -> bool {
    matches!(
        err.status,
        status::NETWORK_TIMEOUT
            | status::NETWORK_UNREACHABLE
            | status::DNS_RESOLUTION_FAILED
            | status::CONNECTION_REFUSED
    )
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_support::*;

    #[test]
    fn find_latest_summary_returns_most_recent() {
        let older = format!(
            "{}\n{}\n{{\"active_task\":\"old\"}}",
            super::super::SUMMARY_MARKER,
            super::super::SUMMARY_HIJACK_GUARD
        );
        let newer = format!(
            "{}\n{}\n{{\"active_task\":\"new\"}}",
            super::super::SUMMARY_MARKER,
            super::super::SUMMARY_HIJACK_GUARD
        );
        let messages = vec![
            user_msg("hi"),
            Message {
                role: "system".to_string(),
                content: older,
                tool_calls: None,
                tool_call_id: None,
            },
            user_msg("more"),
            Message {
                role: "system".to_string(),
                content: newer.clone(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let found = find_latest_context_summary(&messages).unwrap();
        assert_eq!(found, newer);
    }

    #[test]
    fn find_latest_summary_returns_none_when_no_summary() {
        let messages = vec![user_msg("hi"), assistant_msg("hello")];
        assert!(find_latest_context_summary(&messages).is_none());
    }

    #[test]
    fn derive_auto_focus_topic_uses_first_of_recent_three() {
        let messages = vec![
            user_msg("old topic 1"),
            assistant_msg("ok"),
            user_msg("old topic 2"),
            assistant_msg("ok"),
            user_msg("recent topic 1"),
            assistant_msg("ok"),
            user_msg("recent topic 2"),
            assistant_msg("ok"),
            user_msg("recent topic 3"),
        ];
        let topic = derive_auto_focus_topic(&messages).unwrap();
        assert_eq!(topic, "recent topic 1");
    }

    #[test]
    fn derive_auto_focus_topic_truncates_to_50_chars() {
        let long = "x".repeat(100);
        let messages = vec![user_msg(&long)];
        let topic = derive_auto_focus_topic(&messages).unwrap();
        assert_eq!(topic.chars().count(), 50);
        assert_eq!(topic, "x".repeat(50));
    }

    #[test]
    fn derive_auto_focus_topic_returns_none_when_no_user() {
        let messages = vec![assistant_msg("hi"), assistant_msg("there")];
        assert!(derive_auto_focus_topic(&messages).is_none());
    }

    #[test]
    fn is_transient_error_classifies_network_errors() {
        assert!(is_transient_error(&AppError::network_timeout()));
        assert!(is_transient_error(&AppError::network_unreachable()));
        assert!(is_transient_error(&AppError::dns_failed("example.com")));
        assert!(is_transient_error(&AppError::connection_refused("127.0.0.1")));
    }

    #[test]
    fn is_transient_error_rejects_non_network_errors() {
        assert!(!is_transient_error(&AppError::internal("mock failure")));
        assert!(!is_transient_error(&AppError::api_key_invalid()));
        assert!(!is_transient_error(&AppError::no_active_model()));
    }
}