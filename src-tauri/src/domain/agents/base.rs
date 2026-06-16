use async_trait::async_trait;
use crate::errors::AppError;
use crate::infra::llm::{Provider, types::Message};
use std::sync::Arc;

use super::types::{AgentRole, LlmResponse};

/// Context passed to every agent execution
#[derive(Clone)]
pub struct AgentContext {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: std::path::PathBuf,
    pub book_id: Option<String>,
}

/// Base trait for all pipeline agents.
///
/// Agents are stateless — all state lives in the pipeline context
/// and the structured state files on disk.
#[async_trait]
pub trait BaseAgent: Send + Sync {
    /// The role identifier
    fn role(&self) -> AgentRole;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Call the LLM with system + user messages
    async fn chat(
        &self,
        ctx: &AgentContext,
        system: &str,
        user: &str,
    ) -> Result<LlmResponse, AppError> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: user.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];
        let start = std::time::Instant::now();
        let content = ctx
            .provider
            .complete(&ctx.model, system, &messages)
            .await?;
        let elapsed = start.elapsed().as_millis();
        tracing::debug!(
            agent = self.name(),
            response_len = content.len(),
            elapsed_ms = elapsed,
            "LLM call completed"
        );
        Ok(LlmResponse {
            content,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        })
    }
}
