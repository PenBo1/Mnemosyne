use serde::{Deserialize, Serialize};

/// Main agent execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Planning,
    Executing,
    WaitingForConfirmation,
    Paused,
    Completed,
    Failed,
}

/// A step in the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: u32,
    pub description: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<serde_json::Value>,
    pub risk_level: RiskLevel,
    pub status: StepStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
    AwaitingConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Moderate,
    High,
}

/// Confirmation request sent to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationRequest {
    pub step_id: u32,
    pub description: String,
    pub details: String,
    pub risk_level: RiskLevel,
}

/// User's response to a confirmation request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfirmationResponse {
    Approved,
    Rejected,
    Modified(String),
}

/// Progress update sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub status: AgentStatus,
    pub current_step: Option<u32>,
    pub total_steps: Option<u32>,
    pub message: String,
}

/// The agent's execution context for a single run
#[derive(Debug, Clone)]
pub struct AgentRunContext {
    pub goal: String,
    pub conversation_id: String,
    pub max_iterations: u32,
    pub current_iteration: u32,
    pub plan: Vec<PlanStep>,
    pub conversation_history: Vec<ConversationMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Agent,
    System,
    Tool,
}
