//! ═══════════════════════════════════════════════════════════════════════════
//! PromptOptimizer - 提示词优化服务
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供提示词优化功能，将用户输入的提示词转换为更加清晰、具体、可执行的形式。

use crate::infrastructure::llm::client::LlmClient;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::infrastructure::llm::types::Message;
use crate::core::agent::prompts::PROMPT_OPTIMIZER_SYSTEM;
use crate::shared::error::AppError;

// ── 优化器服务 ────────────────────────────────────────────────────────────────

/// 提示词优化服务
///
/// 负责将用户输入的提示词优化为更加清晰、具体、可执行的形式。
/// 使用核心层的系统提示词进行优化。
pub struct PromptOptimizer;

impl PromptOptimizer {
    /// 优化提示词
    ///
    /// # 参数
    /// - `registry`: Provider 注册表，用于获取激活的模型
    /// - `prompt`: 用户输入的原始提示词
    ///
    /// # 返回值
    /// 返回优化后的提示词，或错误信息
    ///
    /// # 流程
    /// 1. 获取当前激活的 Provider 和模型配置
    /// 2. 构造优化请求消息
    /// 3. 调用 LLM 进行优化
    /// 4. 返回优化结果
    pub async fn optimize(
        registry: &ProviderRegistry,
        prompt: &str,
    ) -> Result<String, AppError> {
        let start = std::time::Instant::now();
        tracing::info!(prompt_len = prompt.len(), "prompt_optimizer: enter");

        // 获取激活的 Provider
        let provider = registry.active_provider()?;
        let config = registry.active_model_config()
            .ok_or_else(|| AppError::internal("no active model configured"))?;

        // 创建 LLM 客户端
        let client = LlmClient::new(provider);

        // 构造消息
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        // 调用 LLM 进行优化
        let optimized = client.complete(&config.model, PROMPT_OPTIMIZER_SYSTEM, &messages, 2048).await?;

        let duration_ms = start.elapsed().as_millis();
        tracing::info!(
            original_len = prompt.len(),
            optimized_len = optimized.len(),
            duration_ms = duration_ms,
            "prompt_optimizer: exit"
        );

        Ok(optimized)
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_not_empty() {
        assert!(!PROMPT_OPTIMIZER_SYSTEM.is_empty());
    }
}