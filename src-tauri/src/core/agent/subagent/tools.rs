use std::path::PathBuf;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use super::types::{SubAgentRole, SubAgentResult};

pub struct SubAgentTool {
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
        let task_clone = args.task.clone();
        Ok(SubAgentResult {
            role: args.role,
            task: args.task,
            output: format!("Sub-agent execution simulated for: {}", task_clone),
            tokens_used: 0,
        })
    }
}