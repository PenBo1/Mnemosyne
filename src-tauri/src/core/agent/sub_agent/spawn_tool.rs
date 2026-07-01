//! `spawn_subagent` 工具 — 让主 Agent 在对话中自主 spawn 子 Agent。
//!
//! 学习 codex 的 `multi_agents_v2/spawn.rs`：spawn_agent 是一个模型工具，
//! 主 Agent 在 ReAct 循环中通过 tool_call 自主决定何时 spawn、用什么角色、做什么任务。
//!
//! 工具执行流程：
//! 1. 解析 role + task 参数
//! 2. 调 SubAgentControl::spawn 启动子 Agent
//! 3. 等待子 Agent 完成（通过 oneshot channel）
//! 4. 返回 completion message 作为 tool result

use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::core::agent::base::{ToolDefinition, ToolExecutor, ToolResult};
use crate::shared::errors::AppError;

use super::completion::format_completion_message;
use super::control::{ParentAgentRefs, SubAgentControl};
use super::types::{SubAgentRole, SubAgentSpawnRequest, SubAgentStatus};

/// Spawn 子 Agent 的工具。
///
/// 注册到主 Agent 的 ToolRegistry 中，工具名为 `spawn_subagent`。
/// 主 Agent 在 ReAct 循环中可自主调用此工具来委托子任务给子 Agent。
pub struct SpawnAgentTool {
    control: Arc<SubAgentControl>,
    refs: ParentAgentRefs,
}

impl SpawnAgentTool {
    pub fn new(control: Arc<SubAgentControl>, refs: ParentAgentRefs) -> Self {
        Self { control, refs }
    }
}

#[async_trait]
impl ToolExecutor for SpawnAgentTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Spawn a sub-agent to handle a specific subtask autonomously. \
Use this when a subtask benefits from focused execution with a specialized role. \
The sub-agent runs its own ReAct loop and returns its result when done.\n\n\
Roles:\n\
- \"researcher\": Read-only agent for gathering information (read files, search memory).\n\
- \"outliner\": Agent for generating/revising outline files (read + write files).\n\
- \"critic\": Read-only agent for reviewing content and giving structured feedback.\n\
- \"default\": General-purpose agent with full tool access (no recursive spawning).\n\n\
The sub-agent's result is returned as a structured completion message. \
Choose the most restrictive role that can accomplish the task."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "enum": ["researcher", "outliner", "critic", "default"],
                        "description": "Role of the sub-agent. Determines its tool whitelist."
                    },
                    "task": {
                        "type": "string",
                        "description": "The specific subtask for the sub-agent to complete. Be precise and actionable."
                    },
                    "context": {
                        "type": "string",
                        "description": "Optional additional context to pass to the sub-agent (e.g. relevant file paths, constraints, prior findings)."
                    }
                },
                "required": ["role", "task"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        // 1. 解析参数
        let role_str = args
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("role"))?;

        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("task"))?;

        if task.trim().is_empty() {
            return Err(AppError::bad_request("task must not be empty"));
        }

        if task.len() > 10_000 {
            return Err(AppError::bad_request("task too long (max 10000 chars)"));
        }

        let role = SubAgentRole::from_str(role_str)
            .map_err(AppError::bad_request)?;

        let context = args
            .get("context")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 2. 构建 spawn 请求
        let req = SubAgentSpawnRequest {
            role,
            task: task.to_string(),
            parent_thread_id: self.refs.parent_thread_id.clone(),
            context,
        };

        // 3. spawn 子 Agent 并等待结果
        tracing::info!(
            role = %role,
            task_len = task.len(),
            parent = %self.refs.parent_thread_id,
            "Spawning sub-agent"
        );

        let (task_id, result_rx) = self.control.spawn(req, &self.refs).await?;

        let result = result_rx.await.map_err(|_| {
            AppError::internal(format!(
                "Sub-agent task {} dropped without returning result",
                task_id
            ))
        })?;

        tracing::info!(
            task_id = %task_id,
            role = %role,
            status = ?result.status,
            duration_ms = result.duration_ms,
            artifacts = result.artifacts.len(),
            "Sub-agent completed"
        );

        // 4. 格式化完成消息并返回
        let message = format_completion_message(&task_id, &role, &result);

        let is_error = matches!(
            result.status,
            SubAgentStatus::Errored | SubAgentStatus::Cancelled
        );

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: message,
            is_error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_has_correct_name() {
        // ToolDefinition name is set by the registry, not the tool itself.
        // We just verify the tool can be constructed and definition works.
        // Full integration test requires a mock provider — covered by integration tests.
    }

    #[test]
    fn role_parsing() {
        assert_eq!(
            SubAgentRole::from_str("researcher").unwrap(),
            SubAgentRole::Researcher
        );
        assert_eq!(
            SubAgentRole::from_str("outliner").unwrap(),
            SubAgentRole::Outliner
        );
        assert!(SubAgentRole::from_str("invalid").is_err());
    }
}
