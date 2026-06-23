use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;

use crate::errors::AppError;
use crate::domain::agents::base::{ToolDefinition, ToolExecutor, ToolResult};
use crate::infra::sandbox::SandboxEnforcer;

pub struct BashTool {
    work_dir: PathBuf,
    sandbox: Option<Arc<SandboxEnforcer>>,
}

impl BashTool {
    pub fn new(work_dir: PathBuf, sandbox: Option<Arc<SandboxEnforcer>>) -> Self {
        Self { work_dir, sandbox }
    }
}

#[async_trait]
impl ToolExecutor for BashTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Execute a shell command".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("command"))?;

        if let Some(ref sandbox) = self.sandbox {
            if let Err(violation) = sandbox.validate_command(command) {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("Sandbox violation: {}", violation),
                    is_error: true,
                });
            }
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.work_dir)
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&stderr);
        }
        if result.is_empty() {
            result = format!("Command completed with exit code {}", exit_code);
        } else if exit_code != 0 {
            result.push_str(&format!("\n[exit code: {}]", exit_code));
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: result,
            is_error: !output.status.success(),
        })
    }
}
