//! ═══════════════════════════════════════════════════════════════════════════
//! SubAgentTool - 子代理工具
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};

use super::executor::SubAgentTask;
use super::types::SubAgentRole;
use crate::core::agent::engine::AgentEngine;

pub struct SubAgentTool {
    pub engine: Arc<AgentEngine>,
}

#[derive(Deserialize)]
pub struct SubAgentToolArgs {
    pub role: SubAgentRole,
    pub task: String,
    pub context: String,
    #[serde(default)]
    pub skill: Option<String>,
}

#[async_trait]
impl Tool for SubAgentTool {
    fn name(&self) -> &str {
        "subagent"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
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
                    },
                    "skill": {
                        "type": "string",
                        "enum": ["loop-triage", "loop-verifier", "minimal-fix"],
                        "description": "Optional skill behavior constraint to inject (loop-engineering skills)"
                    }
                },
                "required": ["role", "task", "context"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: SubAgentToolArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let executor = self.engine.subagent_executor();
        let mut task = SubAgentTask::new(args.role, args.task, args.context);
        if let Some(skill) = args.skill {
            task = task.with_skill_prompt(&skill);
        }
        let outcome = executor
            .execute(task)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;
        Ok(serde_json::to_value(&*outcome.result).map_err(|e| ToolError::Other(e.to_string()))?)
    }
}
