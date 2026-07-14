use std::path::PathBuf;

use rig::completion::Prompt;
use rig::agent::AgentBuilder;

use crate::shared::error::AppError;

use super::types::{SubAgentRole, SubAgentResult};

pub struct SubAgent {
    role: SubAgentRole,
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

impl SubAgent {
    pub fn new(role: SubAgentRole, workspace_root: PathBuf) -> Self {
        Self { role, workspace_root }
    }

    pub fn role(&self) -> SubAgentRole {
        self.role
    }

    pub async fn execute<M>(
        &self,
        agent_builder: AgentBuilder<M>,
        task: &str,
        context: &str,
        skill_prompt: Option<&str>,
    ) -> Result<SubAgentResult, AppError>
    where
        M: rig::completion::CompletionModel + Send + Sync + 'static,
    {
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

        let agent = agent_builder
            .preamble(&preamble)
            .max_tokens(4096)
            .build();

        let response = agent.prompt(&full_prompt).await
            .map_err(|e| AppError::stream_error(format!("Sub-agent {} failed: {}", self.role.as_str(), e)))?;

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
    workspace_root: PathBuf,
}

impl SubAgentBuilder {
    pub fn new(role: SubAgentRole, workspace_root: PathBuf) -> Self {
        Self { role, workspace_root }
    }

    pub fn build(self) -> SubAgent {
        SubAgent::new(self.role, self.workspace_root)
    }
}