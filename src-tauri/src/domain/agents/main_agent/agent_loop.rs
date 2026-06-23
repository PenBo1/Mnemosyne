use crate::errors::AppError;
use crate::domain::agents::base::AgentContext;
use super::types::*;
use super::safety_gate::SafetyGate;
use super::planner::Planner;
use std::sync::Arc;
use tokio::sync::mpsc;

/// The core autonomous agent loop
pub struct AgentLoop {
    ctx: AgentContext,
    progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    confirmation_tx: mpsc::UnboundedSender<ConfirmationRequest>,
    confirmation_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ConfirmationResponse>>>,
}

impl AgentLoop {
    pub fn new(
        ctx: AgentContext,
        progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
        confirmation_tx: mpsc::UnboundedSender<ConfirmationRequest>,
        confirmation_rx: mpsc::UnboundedReceiver<ConfirmationResponse>,
    ) -> Self {
        Self {
            ctx,
            progress_tx,
            confirmation_tx,
            confirmation_rx: Arc::new(tokio::sync::Mutex::new(confirmation_rx)),
        }
    }

    /// Execute a goal autonomously
    pub async fn execute(&self, goal: &str) -> Result<String, AppError> {
        let _conversation_id = uuid::Uuid::new_v4().to_string();
        let mut plan: Vec<PlanStep>;
        let mut completed_steps: Vec<PlanStep> = Vec::new();
        let mut skipped_steps: Vec<PlanStep> = Vec::new();
        let mut failed_step: Option<PlanStep> = None;
        let mut current_iteration: u32 = 0;
        let max_iterations: u32 = 30;

        // Phase 1: Planning
        self.send_progress(AgentStatus::Planning, None, None, "Creating execution plan...".to_string()).await;
        plan = Planner::create_plan(&self.ctx, goal).await?;

        self.send_progress(
            AgentStatus::Executing,
            Some(0),
            Some(plan.len() as u32),
            format!("Plan created with {} steps", plan.len()),
        ).await;

        // Phase 2: Execute steps
        let mut step_idx = 0;
        while step_idx < plan.len() {
            current_iteration += 1;
            if current_iteration > max_iterations {
                self.send_progress(
                    AgentStatus::Failed,
                    None, None,
                    "Hit iteration limit".to_string(),
                ).await;
                break;
            }

            let total_steps = plan.len() as u32;

            // Safety check
            if plan[step_idx].risk_level == RiskLevel::High || plan[step_idx].risk_level == RiskLevel::Moderate {
                let step = &plan[step_idx];
                let request = SafetyGate::create_confirmation_request(
                    step.id,
                    step.tool_name.as_deref().unwrap_or("unknown"),
                    step.tool_args.as_ref().unwrap_or(&serde_json::Value::Null),
                );

                self.send_progress(
                    AgentStatus::WaitingForConfirmation,
                    Some(step.id),
                    Some(total_steps),
                    format!("Requesting confirmation: {}", step.description),
                ).await;

                let _ = self.confirmation_tx.send(request);

                let response = {
                    let mut rx = self.confirmation_rx.lock().await;
                    rx.recv().await.unwrap_or(ConfirmationResponse::Rejected)
                };

                match response {
                    ConfirmationResponse::Approved => {}
                    ConfirmationResponse::Rejected => {
                        plan[step_idx].status = StepStatus::Skipped;
                        plan[step_idx].result = Some("Skipped by user".to_string());
                        skipped_steps.push(plan[step_idx].clone());
                        step_idx += 1;
                        continue;
                    }
                    ConfirmationResponse::Modified(new_args) => {
                        if let Ok(args) = serde_json::from_str::<serde_json::Value>(&new_args) {
                            plan[step_idx].tool_args = Some(args);
                        }
                    }
                }
            }

            // Execute the step
            plan[step_idx].status = StepStatus::InProgress;
            self.send_progress(
                AgentStatus::Executing,
                Some(plan[step_idx].id),
                Some(total_steps),
                format!("Step {}: {}", plan[step_idx].id, plan[step_idx].description),
            ).await;

            let step_id = plan[step_idx].id;

            match self.execute_step(&plan[step_idx]).await {
                Ok(result) => {
                    plan[step_idx].status = StepStatus::Completed;
                    plan[step_idx].result = Some(result);
                    completed_steps.push(plan[step_idx].clone());

                    self.send_progress(
                        AgentStatus::Executing,
                        Some(step_id),
                        Some(total_steps),
                        format!("Step {} completed", step_id),
                    ).await;

                    step_idx += 1;
                }
                Err(e) => {
                    plan[step_idx].status = StepStatus::Failed;
                    plan[step_idx].result = Some(e.to_string());
                    failed_step = Some(plan[step_idx].clone());

                    // Try to replan
                    self.send_progress(
                        AgentStatus::Executing,
                        Some(step_id),
                        None,
                        format!("Step {} failed, replanning...", step_id),
                    ).await;

                    match Planner::replan(&self.ctx, goal, &completed_steps, Some(&plan[step_idx])).await {
                        Ok(new_steps) if !new_steps.is_empty() => {
                            // Replace remaining steps with new plan
                            plan.truncate(step_idx);
                            plan.extend(new_steps);
                            // Don't increment step_idx — new steps start at current position
                        }
                        _ => {
                            self.send_progress(
                                AgentStatus::Failed,
                                Some(step_id),
                                None,
                                format!("Replanning failed, stopping: {}", e),
                            ).await;
                            break;
                        }
                    }
                }
            }
        }

        // Phase 3: Summarize
        let summary = self.summarize(goal, &completed_steps, &skipped_steps, &failed_step).await;
        self.send_progress(AgentStatus::Completed, None, None, "Execution complete".to_string()).await;

        Ok(summary)
    }

    /// Execute a single tool call
    async fn execute_step(&self, step: &PlanStep) -> Result<String, AppError> {
        let tool_name = step.tool_name.as_ref()
            .ok_or_else(|| AppError::bad_request("Step has no tool specified"))?;
        let args = step.tool_args.clone()
            .unwrap_or(serde_json::Value::Null);

        let result = self.ctx.tools.execute(tool_name, args).await?;

        if result.is_error {
            Err(AppError::internal(format!("Tool error: {}", result.content)))
        } else {
            Ok(result.content)
        }
    }

    /// Generate a summary of the execution
    async fn summarize(&self, goal: &str, completed: &[PlanStep], skipped: &[PlanStep], failed: &Option<PlanStep>) -> String {
        let completed_count = completed.len();
        let summary_lines: Vec<String> = completed.iter().map(|s| {
            format!("  Step {}: {} -> {}", s.id, s.description,
                s.result.as_deref().unwrap_or("done"))
        }).collect();

        let skipped_lines: Vec<String> = skipped.iter().map(|s| {
            format!("  Step {}: {} [SKIPPED]", s.id, s.description)
        }).collect();

        let failed_line = if let Some(f) = failed {
            format!("\n  FAILED: {} ({})", f.description, f.result.as_deref().unwrap_or("unknown"))
        } else {
            String::new()
        };

        let mut result = format!(
            "Goal: {}\nCompleted: {} steps{}\n\nResults:\n{}",
            goal,
            completed_count,
            failed_line,
            summary_lines.join("\n")
        );

        if !skipped_lines.is_empty() {
            result.push_str(&format!("\n\nSkipped:\n{}", skipped_lines.join("\n")));
        }

        result
    }

    async fn send_progress(&self, status: AgentStatus, current: Option<u32>, total: Option<u32>, message: String) {
        let _ = self.progress_tx.send(ProgressUpdate {
            status,
            current_step: current,
            total_steps: total,
            message,
        });
    }
}
