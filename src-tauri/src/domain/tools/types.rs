use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use crate::infra::llm::ToolSpec;
use crate::infra::sandbox::enforce::SandboxEnforcer;

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: false, metadata: None }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self { content: message.into(), is_error: true, metadata: None }
    }
}

pub struct ToolContext {
    pub session_id: String,
    pub work_dir: String,
    pub novel_id: Option<String>,
    pub sandbox: Option<std::sync::Arc<SandboxEnforcer>>,
}

pub trait ToolExecutor: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError>;
}
