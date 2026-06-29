use crate::shared::errors::AppError;
use crate::core::agent::base::AgentContext;
use crate::infrastructure::llm_client::types::Message;
use super::task_lifecycle::TaskManager;

pub struct GoalDecomposer;

impl GoalDecomposer {
    /// Decompose a high-level goal into subtasks with agent assignments
    pub async fn decompose(
        ctx: &AgentContext,
        goal: &str,
        task_manager: &mut TaskManager,
    ) -> Result<Vec<String>, AppError> {
        let system = "You are a task decomposition expert. Given a goal, break it into concrete subtasks.
Each subtask should be assigned to one of these agent roles: architect, planner, composer, writer, auditor, reviser, observer, reflector.

Return a JSON array of objects with keys: summary (string), assigned_agent (string).
Example: [{\"summary\": \"Create story structure\", \"assigned_agent\": \"architect\"}]";

        let user_message = format!("Decompose this goal into subtasks:\n\n{}", goal);

        let response = ctx.provider.complete(
            &ctx.model,
            system,
            &[Message {
                role: "user".to_string(),
                content: user_message,
                tool_calls: None,
                tool_call_id: None,
            }],
        ).await?;

        let tasks: Vec<serde_json::Value> = serde_json::from_str(&response)
            .map_err(|e| AppError::internal(format!("Failed to parse decomposition: {}", e)))?;

        let mut task_ids = Vec::new();
        for task in &tasks {
            let summary = task["summary"].as_str().unwrap_or("Untitled task").to_string();
            let agent = task["assigned_agent"].as_str().unwrap_or("writer");

            let id = task_manager.create(summary, None);
            if let Err(e) = task_manager.start(&id, agent) {
                tracing::warn!(task_id = %id, error = %e, "Failed to start decomposed task");
            }
            task_ids.push(id);
        }

        Ok(task_ids)
    }
}
