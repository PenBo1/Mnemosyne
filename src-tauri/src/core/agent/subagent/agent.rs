//! ═══════════════════════════════════════════════════════════════════════════
//! SubAgent - 子代理执行器
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use std::time::Instant;

use crate::shared::error::AppError;
use crate::infrastructure::llm::types::{Message, Provider};
use crate::core::agent::effort::EffortLevel;

use super::types::{SubAgentRole, SubAgentResult};

pub struct SubAgent {
    role: SubAgentRole,
}

impl SubAgent {
    pub fn new(role: SubAgentRole) -> Self {
        Self { role }
    }

    pub fn role(&self) -> SubAgentRole {
        self.role
    }

    pub async fn execute(
        &self,
        provider: &Arc<dyn Provider>,
        model: &str,
        task: &str,
        context: &str,
        skill_prompt: Option<&str>,
    ) -> Result<SubAgentResult, AppError> {
        let start = Instant::now();
        let task_preview = if task.len() > 100 {
            format!("{}...", &task[..100])
        } else {
            task.to_string()
        };
        tracing::info!(
            role = %self.role.as_str(),
            task_preview = %task_preview,
            has_skill_prompt = skill_prompt.is_some(),
            "[subagent] execute: starting"
        );

        let full_prompt = format!(
            "Context:\n{}\n\nTask: {}\n\nPlease complete the task based on the context provided.",
            context, task
        );

        // 拼接 role system_prompt 与 skill 行为 prompt(若有)
        let preamble = match skill_prompt {
            Some(sp) => format!(
                "{}\n\n---\n\n# Skill Behavior Constraint\n\n{}",
                self.role.system_prompt(),
                sp
            ),
            None => self.role.system_prompt().to_string(),
        };

        // Provider::complete 签名：(model, system, messages, max_tokens)
        // system 单独传，messages 只含 user/assistant 消息
        // sub-agent 为单次非工具调用，取默认 EffortLevel(Medium) 的 max_tokens_per_call 作为输出上限
        let max_tokens = EffortLevel::default().params().max_tokens_per_call;
        let messages = vec![Message {
            role: "user".to_string(),
            content: full_prompt,
            tool_calls: None,
            tool_call_id: None,
        }];

        let response = provider.complete(model, &preamble, &messages, max_tokens).await
            .map_err(|e| {
                tracing::error!(
                    role = %self.role.as_str(),
                    task_preview = %task_preview,
                    error = %e,
                    "[subagent] execute: prompt failed"
                );
                AppError::stream_error(format!("Sub-agent {} failed: {}", self.role.as_str(), e))
            })?;

        tracing::info!(
            role = %self.role.as_str(),
            task_preview = %task_preview,
            duration_ms = start.elapsed().as_millis() as u64,
            "[subagent] execute: completed"
        );

        Ok(SubAgentResult {
            role: self.role,
            task: task.to_string(),
            output: response,
            tokens_used: 0,
        })
    }
}

pub struct SubAgentBuilder {
    role: SubAgentRole,
}

impl SubAgentBuilder {
    pub fn new(role: SubAgentRole) -> Self {
        Self { role }
    }

    pub fn build(self) -> SubAgent {
        SubAgent::new(self.role)
    }
}
