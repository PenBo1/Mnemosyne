use std::path::PathBuf;
use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use super::executor::SubAgentTask;
use super::types::{SubAgentRole, SubAgentResult};
use crate::core::agent::engine::AgentEngine;

pub struct SubAgentTool {
    pub engine: Arc<AgentEngine>,
    pub workspace_root: PathBuf,
}

#[derive(Deserialize)]
pub struct SubAgentToolArgs {
    pub role: SubAgentRole,
    pub task: String,
    pub context: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SubAgentToolError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid role: {0}")]
    InvalidRole(String),
}

impl Tool for SubAgentTool {
    const NAME: &'static str = "subagent";

    type Error = SubAgentToolError;
    type Args = SubAgentToolArgs;
    type Output = SubAgentResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delegate a task to a specialized sub-agent (researcher/outliner/critic)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "enum": ["researcher", "outliner", "critic"],
                        "description": "The sub-agent role to use"
                    },
                    "task": {
                        "type": "string",
                        "description": "The task to delegate to the sub-agent"
                    },
                    "context": {
                        "type": "string",
                        "description": "Context information for the sub-agent (files, code, requirements)"
                    }
                },
                "required": ["role", "task", "context"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = self.engine.subagent_executor(self.workspace_root.clone());
        let task = SubAgentTask::new(args.role, args.task, args.context);
        let outcome = executor.execute(task).await
            .map_err(|e| SubAgentToolError::ExecutionFailed(e.to_string()))?;
        // ExecutionResult.result 现为 Arc<SubAgentResult>，在 Tool 边界 deref + clone
        // （rig 的 Tool::Output 必须是 owned SubAgentResult，无法直接返回 Arc）。
        Ok((*outcome.result).clone())
    }
}