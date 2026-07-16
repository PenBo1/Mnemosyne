use std::path::PathBuf;
use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use crate::core::agent::approval::ApprovalManager;
use crate::security_kernel::validation::path::{
    check_symlink_target, validate_path_for_creation, validate_path_with_base,
};

// ── ReadFileTool ──────────────────────────────────────────

pub struct ReadFileTool {
    pub workspace_root: PathBuf,
}

#[derive(Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadFileError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Path traversal not allowed")]
    PathTraversal,
    #[error("File too large: {0} bytes (max 256KB)")]
    TooLarge(u64),
}

impl Tool for ReadFileTool {
    const NAME: &'static str = "read_file";

    type Error = ReadFileError;
    type Args = ReadFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read the contents of a file. Returns the file content as text.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative file path to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：canonicalize-based，防止符号链接绕过 workspace 边界
        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|_| ReadFileError::PathTraversal)?;

        let meta = tokio::fs::metadata(&canonical).await
            .map_err(|e| ReadFileError::Io(e.to_string()))?;

        if meta.len() > 256 * 1024 {
            return Err(ReadFileError::TooLarge(meta.len()));
        }

        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| ReadFileError::Io(e.to_string()))?;

        // Truncate to 2000 lines
        let lines: Vec<&str> = content.lines().take(2001).collect();
        if lines.len() > 2000 {
            Ok(lines[..2000].join("\n"))
        } else {
            Ok(content)
        }
    }
}

// ── ListDirectoryTool ─────────────────────────────────────

pub struct ListDirectoryTool {
    pub workspace_root: PathBuf,
}

#[derive(Deserialize)]
pub struct ListDirectoryArgs {
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ListDirectoryError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Path traversal not allowed")]
    PathTraversal,
}

impl Tool for ListDirectoryTool {
    const NAME: &'static str = "list_directory";

    type Error = ListDirectoryError;
    type Args = ListDirectoryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List the contents of a directory. Returns file and folder names.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative directory path to list"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&self.workspace_root, &args.path);

        // canonicalize-based 校验，防止符号链接绕过
        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|_| ListDirectoryError::PathTraversal)?;

        let mut entries = tokio::fs::read_dir(&canonical).await
            .map_err(|e| ListDirectoryError::Io(e.to_string()))?;

        let mut result = Vec::new();
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ListDirectoryError::Io(e.to_string()))? {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            result.push(if is_dir { format!("{}/", name) } else { name });
        }

        result.sort();
        Ok(result.join("\n"))
    }
}

// ── WriteFileTool (requires approval) ─────────────────────

pub struct WriteFileTool {
    pub workspace_root: PathBuf,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WriteFileError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Path traversal not allowed")]
    PathTraversal,
    #[error("Permission denied")]
    Denied,
}

impl Tool for WriteFileTool {
    const NAME: &'static str = "write_file";

    type Error = WriteFileError;
    type Args = WriteFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Requires user approval.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative file path to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：目标可能不存在，用 creation 校验；同时检查父目录 canonicalize
        let canonical = validate_path_for_creation(&path, &self.workspace_root)
            .map_err(|_| WriteFileError::PathTraversal)?;
        let canonical = canonical.as_path();

        // 父目录若存在，必须 canonicalize 后仍在 workspace 内（防止通过 symlink 父目录逃逸）
        if let Some(parent) = canonical.parent() {
            if parent.exists() {
                validate_path_with_base(parent, &self.workspace_root)
                    .map_err(|_| WriteFileError::PathTraversal)?;
            }
        }

        // 若目标已存在且为符号链接，校验其目标仍在 workspace 内
        if canonical.exists() {
            check_symlink_target(canonical, &self.workspace_root)
                .map_err(|_| WriteFileError::PathTraversal)?;
        }

        let approved = self.approval.request_approval(
            "write_file",
            &serde_json::json!({ "path": args.path }),
        ).await;

        if !approved {
            return Err(WriteFileError::Denied);
        }

        if let Some(parent) = canonical.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| WriteFileError::Io(e.to_string()))?;
        }

        tokio::fs::write(canonical, &args.content).await
            .map_err(|e| WriteFileError::Io(e.to_string()))?;

        Ok(format!("Written {} bytes to {}", args.content.len(), args.path))
    }
}

// ── CreateDirectoryTool (requires approval) ───────────────

pub struct CreateDirectoryTool {
    pub workspace_root: PathBuf,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct CreateDirectoryArgs {
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateDirectoryError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Path traversal not allowed")]
    PathTraversal,
    #[error("Permission denied")]
    Denied,
}

impl Tool for CreateDirectoryTool {
    const NAME: &'static str = "create_directory";

    type Error = CreateDirectoryError;
    type Args = CreateDirectoryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a directory (and parent directories if needed). Requires user approval.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative directory path to create"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：目标可能不存在，用 creation 校验
        let canonical = validate_path_for_creation(&path, &self.workspace_root)
            .map_err(|_| CreateDirectoryError::PathTraversal)?;
        let canonical = canonical.as_path();

        // 父目录若存在，必须 canonicalize 后仍在 workspace 内
        if let Some(parent) = canonical.parent() {
            if parent.exists() {
                validate_path_with_base(parent, &self.workspace_root)
                    .map_err(|_| CreateDirectoryError::PathTraversal)?;
            }
        }

        // 若目标已存在且为符号链接，校验其目标仍在 workspace 内
        if canonical.exists() {
            check_symlink_target(canonical, &self.workspace_root)
                .map_err(|_| CreateDirectoryError::PathTraversal)?;
        }

        let approved = self.approval.request_approval(
            "create_directory",
            &serde_json::json!({ "path": args.path }),
        ).await;

        if !approved {
            return Err(CreateDirectoryError::Denied);
        }

        tokio::fs::create_dir_all(canonical).await
            .map_err(|e| CreateDirectoryError::Io(e.to_string()))?;

        Ok(format!("Created directory {}", args.path))
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
