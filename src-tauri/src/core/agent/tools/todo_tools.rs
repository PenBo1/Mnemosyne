use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

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

#[derive(Debug, thiserror::Error)]
#[error("Todo error: {0}")]
pub struct TodoError(String);

impl Tool for TodoWriteTool {
    const NAME: &'static str = "todo_write";

    type Error = TodoError;
    type Args = TodoWriteArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let in_progress_count = args.todos.iter()
            .filter(|t| t.status == "in_progress")
            .count();
        if in_progress_count > 1 {
            return Err(TodoError("At most one todo can be in_progress".to_string()));
        }

        let count = args.todos.len();
        Ok(format!("Updated {} todos for session {}", count, args.session_id))
    }
}
