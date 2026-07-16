use std::path::PathBuf;
use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use crate::core::agent::approval::ApprovalManager;
use crate::security_kernel::validation::path::{
    check_symlink_target, validate_path_with_base,
};

// ── EditTool (single string replace) ─────────────────────

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

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Path traversal not allowed")]
    PathTraversal,
    #[error("Permission denied")]
    Denied,
    #[error("Old string not found in file")]
    NotFound,
    #[error("Old string found multiple times — provide more context")]
    MultipleMatches,
}

impl Tool for EditTool {
    const NAME: &'static str = "edit";

    type Error = EditError;
    type Args = EditArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：canonicalize-based，防止符号链接绕过 workspace 边界
        // （EditTool 要求文件已存在，因为后续要读其内容做替换）
        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|_| EditError::PathTraversal)?;
        // 若目标为符号链接，校验其 target 仍在 workspace 内
        check_symlink_target(canonical.as_path(), &self.workspace_root)
            .map_err(|_| EditError::PathTraversal)?;

        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| EditError::Io(e.to_string()))?;

        let count = content.matches(&args.old_string).count();
        if count == 0 {
            return Err(EditError::NotFound);
        }
        if count > 1 {
            return Err(EditError::MultipleMatches);
        }

        let approved = self.approval.request_approval(
            "edit",
            &serde_json::json!({ "path": args.path, "old_string": args.old_string, "new_string": args.new_string }),
        ).await;

        if !approved {
            return Err(EditError::Denied);
        }

        let new_content = content.replacen(&args.old_string, &args.new_string, 1);
        tokio::fs::write(&canonical, &new_content).await
            .map_err(|e| EditError::Io(e.to_string()))?;

        Ok(format!("Edited {}", args.path))
    }
}

// ── MultiEditTool (atomic batch replace) ──────────────────

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

#[derive(Debug, thiserror::Error)]
pub enum MultiEditError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Path traversal not allowed")]
    PathTraversal,
    #[error("Permission denied")]
    Denied,
    #[error("Edit {0}: old string not found")]
    NotFound(usize),
    #[error("Edit {0}: old string found multiple times")]
    MultipleMatches(usize),
}

impl Tool for MultiEditTool {
    const NAME: &'static str = "multi_edit";

    type Error = MultiEditError;
    type Args = MultiEditArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：canonicalize-based，防止符号链接绕过 workspace 边界
        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|_| MultiEditError::PathTraversal)?;
        check_symlink_target(canonical.as_path(), &self.workspace_root)
            .map_err(|_| MultiEditError::PathTraversal)?;

        let mut content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| MultiEditError::Io(e.to_string()))?;

        // Validate all edits can apply
        for (i, edit) in args.edits.iter().enumerate() {
            let count = content.matches(&edit.old_string).count();
            if count == 0 {
                return Err(MultiEditError::NotFound(i));
            }
            if count > 1 {
                return Err(MultiEditError::MultipleMatches(i));
            }
        }

        let approved = self.approval.request_approval(
            "multi_edit",
            &serde_json::json!({ "path": args.path, "edit_count": args.edits.len() }),
        ).await;

        if !approved {
            return Err(MultiEditError::Denied);
        }

        // Apply edits sequentially
        for edit in &args.edits {
            content = content.replacen(&edit.old_string, &edit.new_string, 1);
        }

        tokio::fs::write(&canonical, &content).await
            .map_err(|e| MultiEditError::Io(e.to_string()))?;

        Ok(format!("Applied {} edits to {}", args.edits.len(), args.path))
    }
}

// ── Helper ────────────────────────────────────────────────

fn resolve_path(workspace_root: &PathBuf, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workspace_root.join(p)
    }
}
