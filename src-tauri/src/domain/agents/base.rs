use async_trait::async_trait;
use crate::errors::AppError;
use super::types::{AgentInput, AgentOutput, AgentRole};

/// Context passed to every agent execution.
pub struct AgentContext {
    pub book_id: String,
    pub chapter_number: u32,
    pub project_root: std::path::PathBuf,
}

/// The core trait that all agents must implement.
///
/// Each agent represents a specialized role in the novel-writing pipeline.
/// Agents are stateless — all state lives in the pipeline context and
/// the structured state files on disk.
#[async_trait]
pub trait BaseAgent: Send + Sync {
    /// The role identifier (e.g., "planner", "writer", "auditor").
    fn role(&self) -> AgentRole;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// What this agent does.
    fn description(&self) -> &str;

    /// Execute the agent with the given input.
    async fn execute(
        &self,
        ctx: &AgentContext,
        input: AgentInput,
    ) -> Result<AgentOutput, AppError>;

    // ── Lifecycle hooks (optional, have default no-op impls) ──

    /// Called before the agent starts processing a turn.
    async fn on_turn_start(&self, _ctx: &AgentContext) -> Result<(), AppError> {
        Ok(())
    }

    /// Called after the agent finishes processing a turn.
    async fn on_turn_end(
        &self,
        _ctx: &AgentContext,
        _output: &AgentOutput,
    ) -> Result<(), AppError> {
        Ok(())
    }

    /// Called when an error occurs during execution.
    async fn on_error(
        &self,
        _ctx: &AgentContext,
        _error: &AppError,
    ) -> Result<(), AppError> {
        Ok(())
    }

    /// Build the system prompt for this agent.
    /// Override to provide a custom prompt instead of using config.
    fn build_system_prompt(&self, _ctx: &AgentContext) -> Option<String> {
        None
    }

    /// Validate the agent's output before returning it.
    fn validate_output(&self, _output: &AgentOutput) -> Result<(), AppError> {
        Ok(())
    }
}
