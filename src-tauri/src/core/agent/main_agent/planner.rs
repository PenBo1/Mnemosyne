use crate::shared::errors::AppError;
use crate::core::agent::base::AgentContext;
use crate::infrastructure::llm_client::types::Message;
use super::types::{PlanStep, RiskLevel};
use super::safety_gate::SafetyGate;

pub struct Planner;

impl Planner {
    /// Create an execution plan from a user goal using LLM
    pub async fn create_plan(
        ctx: &AgentContext,
        goal: &str,
    ) -> Result<Vec<PlanStep>, AppError> {
        let system = r#"You are a task planning expert. Given a user goal, create a concrete execution plan.

Each step should use one of these tools:
- read_file: Read a file (args: {"path": "..."})
- write_file: Write to a file (args: {"path": "...", "content": "..."})
- list_files: List directory contents (args: {"path": "..."})
- bash: Execute a shell command (args: {"command": "..."})
- search_memory: Search agent memory (args: {"query": "..."})
- archive_memory: Store information in memory (args: {"content": "...", "entry_type": "fact"})

Return a JSON array of steps. Each step:
{"description": "what this step does", "tool_name": "tool to use", "tool_args": {...}}

Rules:
- Be specific and concrete
- Include all necessary file paths
- If you need to read before writing, include read steps first
- For complex tasks, break into smaller steps
- Return ONLY the JSON array, no other text"#;

        let response = ctx.provider.complete(
            &ctx.model,
            system,
            &[Message {
                role: "user".to_string(),
                content: format!("Create an execution plan for this goal:\n\n{}", goal),
                tool_calls: None,
                tool_call_id: None,
            }],
        ).await?;

        let steps_json: Vec<serde_json::Value> = serde_json::from_str(&response)
            .map_err(|e| AppError::internal(format!("Failed to parse plan: {}", e)))?;

        let steps: Vec<PlanStep> = steps_json.into_iter().enumerate().map(|(i, step)| {
            let tool_name = step["tool_name"].as_str().map(|s| s.to_string());
            let tool_args = step.get("tool_args").cloned();
            let risk_level = match &tool_name {
                Some(name) => SafetyGate::evaluate_risk(name, tool_args.as_ref().unwrap_or(&serde_json::Value::Null)),
                None => RiskLevel::Safe,
            };

            PlanStep {
                id: (i as u32) + 1,
                description: step["description"].as_str().unwrap_or("Unknown step").to_string(),
                tool_name,
                tool_args,
                risk_level,
                status: super::types::StepStatus::Pending,
                result: None,
            }
        }).collect();

        Ok(steps)
    }

    /// Replan: adjust the remaining plan based on execution results
    pub async fn replan(
        ctx: &AgentContext,
        original_goal: &str,
        completed_steps: &[PlanStep],
        failed_step: Option<&PlanStep>,
    ) -> Result<Vec<PlanStep>, AppError> {
        let completed_summary: Vec<String> = completed_steps.iter().map(|s| {
            format!("- Step {}: {} → {}", s.id, s.description,
                s.result.as_deref().unwrap_or("no result"))
        }).collect();

        let failed_info = if let Some(failed) = failed_step {
            format!("\nFailed step: {} (error: {})",
                failed.description,
                failed.result.as_deref().unwrap_or("unknown error"))
        } else {
            String::new()
        };

        let system = r#"You are a task replanning expert. Given a goal, completed steps, and any failures, create new remaining steps.

Available tools: read_file, write_file, list_files, bash, search_memory, archive_memory

Return a JSON array of steps. Each step:
{"description": "...", "tool_name": "...", "tool_args": {...}}

Rules:
- Don't repeat completed steps
- Address any failures with alternative approaches
- Be concrete with file paths and commands
- Return ONLY the JSON array"#;

        let user_msg = format!(
            "Goal: {}\n\nCompleted steps:\n{}\n{}\n\nCreate the remaining steps:",
            original_goal,
            completed_summary.join("\n"),
            failed_info
        );

        let response = ctx.provider.complete(
            ctx.model.as_str(),
            system,
            &[Message {
                role: "user".to_string(),
                content: user_msg,
                tool_calls: None,
                tool_call_id: None,
            }],
        ).await?;

        let steps_json: Vec<serde_json::Value> = serde_json::from_str(&response)
            .map_err(|e| AppError::internal(format!("Failed to parse replan: {}", e)))?;

        let max_id = completed_steps.iter().map(|s| s.id).max().unwrap_or(0);

        let steps: Vec<PlanStep> = steps_json.into_iter().enumerate().map(|(i, step)| {
            let tool_name = step["tool_name"].as_str().map(|s| s.to_string());
            let tool_args = step.get("tool_args").cloned();
            let risk_level = match &tool_name {
                Some(name) => SafetyGate::evaluate_risk(name, tool_args.as_ref().unwrap_or(&serde_json::Value::Null)),
                None => RiskLevel::Safe,
            };

            PlanStep {
                id: max_id + (i as u32) + 1,
                description: step["description"].as_str().unwrap_or("Unknown step").to_string(),
                tool_name,
                tool_args,
                risk_level,
                status: super::types::StepStatus::Pending,
                result: None,
            }
        }).collect();

        Ok(steps)
    }
}
