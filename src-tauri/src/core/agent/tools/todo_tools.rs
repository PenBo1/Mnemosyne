//! ═══════════════════════════════════════════════════════════════════════════
//! TodoTools - Todo 列表工具
//! ═══════════════════════════════════════════════════════════════════════════

use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;

use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};

pub struct TodoWriteTool;

#[derive(Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String, // "pending" | "in_progress" | "completed"
}

#[derive(Deserialize)]
pub struct TodoWriteArgs {
    pub session_id: String,
    pub todos: Vec<TodoItem>,
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "todo_write".to_string(),
            description: "Replace the current task list for a session. At most one todo can be in_progress at a time.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Current session ID"
                    },
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "content": { "type": "string" },
                                "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                            },
                            "required": ["id", "content", "status"]
                        },
                        "description": "Complete list of todos to set"
                    }
                },
                "required": ["session_id", "todos"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: TodoWriteArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "todo_write", session_id = %args.session_id, todo_count = args.todos.len(), "[tool] call");

        let in_progress_count = args.todos.iter()
            .filter(|t| t.status == "in_progress")
            .count();
        if in_progress_count > 1 {
            tracing::error!(tool = "todo_write", session_id = %args.session_id, in_progress_count, "[tool] multiple in_progress todos");
            return Err(ToolError::Execution("At most one todo can be in_progress".to_string()));
        }

        let count = args.todos.len();
        tracing::info!(tool = "todo_write", session_id = %args.session_id, todo_count = count, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(format!("Updated {} todos for session {}", count, args.session_id)))
    }
}
