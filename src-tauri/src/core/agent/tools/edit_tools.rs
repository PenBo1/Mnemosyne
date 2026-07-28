//! ═══════════════════════════════════════════════════════════════════════════
//! EditTools - 文件编辑工具
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;

use crate::core::agent::approval::ApprovalManager;
use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};
use crate::security_kernel::validation::path::{
    check_symlink_target, validate_path_with_base,
};

pub struct EditTool {
    pub workspace_root: PathBuf,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct EditArgs {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit".to_string(),
            description: "Replace an exact string in a file. The old_string must be found exactly once. Requires user approval.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact string to find and replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement string"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: EditArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "edit", path = %args.path, "[tool] call");

        let path = resolve_path(&self.workspace_root, &args.path);

        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "edit", path = %args.path, error = ?e, "[tool] path validation failed");
                ToolError::PathTraversal
            })?;
        check_symlink_target(canonical.as_path(), &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "edit", path = %args.path, error = ?e, "[tool] symlink check failed");
                ToolError::PathTraversal
            })?;

        tracing::debug!(tool = "edit", path = %canonical.as_path().display(), "[tool] reading file");

        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| {
                tracing::error!(tool = "edit", path = %args.path, error = %e, "[tool] read failed");
                ToolError::Io(e.to_string())
            })?;

        let count = content.matches(&args.old_string).count();
        if count == 0 {
            tracing::warn!(tool = "edit", path = %args.path, "[tool] old string not found");
            return Err(ToolError::Execution("Old string not found in file".to_string()));
        }
        if count > 1 {
            tracing::warn!(tool = "edit", path = %args.path, count, "[tool] old string found multiple times");
            return Err(ToolError::Execution("Old string found multiple times — provide more context".to_string()));
        }

        tracing::info!(tool = "edit", path = %args.path, "[tool] requesting approval");

        let approved = self.approval.request_approval(
            "edit",
            &serde_json::json!({ "path": args.path, "old_string": args.old_string, "new_string": args.new_string }),
        ).await;

        if !approved {
            tracing::warn!(tool = "edit", path = %args.path, "[tool] approval denied");
            return Err(ToolError::Execution("Permission denied".to_string()));
        }

        tracing::debug!(tool = "edit", path = %args.path, "[tool] writing changes");

        let new_content = content.replacen(&args.old_string, &args.new_string, 1);
        tokio::fs::write(&canonical, &new_content).await
            .map_err(|e| {
                tracing::error!(tool = "edit", path = %args.path, error = %e, "[tool] write failed");
                ToolError::Io(e.to_string())
            })?;

        tracing::info!(tool = "edit", path = %args.path, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(format!("Edited {}", args.path)))
    }
}

pub struct MultiEditTool {
    pub workspace_root: PathBuf,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct EditOperation {
    pub old_string: String,
    pub new_string: String,
}

#[derive(Deserialize)]
pub struct MultiEditArgs {
    pub path: String,
    pub edits: Vec<EditOperation>,
}

#[async_trait]
impl Tool for MultiEditTool {
    fn name(&self) -> &str {
        "multi_edit"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "multi_edit".to_string(),
            description: "Apply multiple edits to a file atomically. All edits must succeed or none are applied. Requires user approval.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to edit"
                    },
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "old_string": { "type": "string" },
                                "new_string": { "type": "string" }
                            },
                            "required": ["old_string", "new_string"]
                        },
                        "description": "List of edit operations to apply in order"
                    }
                },
                "required": ["path", "edits"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: MultiEditArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "multi_edit", path = %args.path, edit_count = args.edits.len(), "[tool] call");

        let path = resolve_path(&self.workspace_root, &args.path);

        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "multi_edit", path = %args.path, error = ?e, "[tool] path validation failed");
                ToolError::PathTraversal
            })?;
        check_symlink_target(canonical.as_path(), &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "multi_edit", path = %args.path, error = ?e, "[tool] symlink check failed");
                ToolError::PathTraversal
            })?;

        let mut content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| {
                tracing::error!(tool = "multi_edit", path = %args.path, error = %e, "[tool] read failed");
                ToolError::Io(e.to_string())
            })?;

        for (i, edit) in args.edits.iter().enumerate() {
            let count = content.matches(&edit.old_string).count();
            if count == 0 {
                tracing::error!(tool = "multi_edit", edit_index = i, "[tool] old string not found");
                return Err(ToolError::Execution(format!("Edit {}: old string not found", i)));
            }
            if count > 1 {
                tracing::error!(tool = "multi_edit", edit_index = i, count, "[tool] old string found multiple times");
                return Err(ToolError::Execution(format!("Edit {}: old string found multiple times", i)));
            }
        }

        let approved = self.approval.request_approval(
            "multi_edit",
            &serde_json::json!({ "path": args.path, "edit_count": args.edits.len() }),
        ).await;

        if !approved {
            tracing::error!(tool = "multi_edit", path = %args.path, "[tool] approval denied");
            return Err(ToolError::Execution("Permission denied".to_string()));
        }

        for edit in &args.edits {
            content = content.replacen(&edit.old_string, &edit.new_string, 1);
        }

        tokio::fs::write(&canonical, &content).await
            .map_err(|e| {
                tracing::error!(tool = "multi_edit", path = %args.path, error = %e, "[tool] write failed");
                ToolError::Io(e.to_string())
            })?;

        tracing::info!(tool = "multi_edit", path = %args.path, edit_count = args.edits.len(), duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(format!("Applied {} edits to {}", args.edits.len(), args.path)))
    }
}

fn resolve_path(workspace_root: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workspace_root.join(p)
    }
}
