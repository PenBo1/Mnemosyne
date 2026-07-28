//! ═══════════════════════════════════════════════════════════════════════════
//! FSTools - 文件系统工具
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;

use crate::core::agent::approval::ApprovalManager;
use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};
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

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: ReadFileArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        let path_display = path_relative_or_name(&args.path, &self.workspace_root);
        tracing::info!(tool = "read_file", path = %path_display, "[tool] call");

        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：canonicalize-based，防止符号链接绕过 workspace 边界
        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "read_file", path = %path_display, error = %e, "[tool] path validation failed");
                ToolError::PathTraversal
            })?;

        let meta = tokio::fs::metadata(&canonical).await
            .map_err(|e| {
                tracing::error!(tool = "read_file", path = %path_display, error = %e, "[tool] metadata failed");
                ToolError::Io(e.to_string())
            })?;

        if meta.len() > 256 * 1024 {
            tracing::error!(tool = "read_file", path = %path_display, size = meta.len(), max = 262144, "[tool] file too large");
            return Err(ToolError::Execution(format!("File too large: {} bytes", meta.len())));
        }

        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| {
                tracing::error!(tool = "read_file", path = %path_display, error = %e, "[tool] read failed");
                ToolError::Io(e.to_string())
            })?;

        // Truncate to 2000 lines
        let lines: Vec<&str> = content.lines().take(2001).collect();
        let result = if lines.len() > 2000 {
            lines[..2000].join("\n")
        } else {
            content
        };

        tracing::info!(tool = "read_file", path = %path_display, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(result))
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

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_directory".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: ListDirectoryArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        let path_display = path_relative_or_name(&args.path, &self.workspace_root);
        tracing::info!(tool = "list_directory", path = %path_display, "[tool] call");

        let path = resolve_path(&self.workspace_root, &args.path);

        // canonicalize-based 校验，防止符号链接绕过
        let canonical = validate_path_with_base(&path, &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "list_directory", path = %path_display, error = %e, "[tool] path validation failed");
                ToolError::PathTraversal
            })?;

        let mut entries = tokio::fs::read_dir(&canonical).await
            .map_err(|e| {
                tracing::error!(tool = "list_directory", path = %path_display, error = %e, "[tool] read_dir failed");
                ToolError::Io(e.to_string())
            })?;

        let mut result = Vec::new();
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| {
                tracing::error!(tool = "list_directory", path = %path_display, error = %e, "[tool] next_entry failed");
                ToolError::Io(e.to_string())
            })? {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            result.push(if is_dir { format!("{}/", name) } else { name });
        }

        result.sort();
        tracing::info!(tool = "list_directory", path = %path_display, entry_count = result.len(), duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(result.join("\n")))
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

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: WriteFileArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        let path_display = path_relative_or_name(&args.path, &self.workspace_root);
        let content_len = args.content.len();
        tracing::info!(tool = "write_file", path = %path_display, content_len, "[tool] call");

        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：目标可能不存在，用 creation 校验；同时检查父目录 canonicalize
        let canonical = validate_path_for_creation(&path, &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "write_file", path = %path_display, error = %e, "[tool] path validation failed");
                ToolError::PathTraversal
            })?;
        let canonical = canonical.as_path();

        // 父目录若存在，必须 canonicalize 后仍在 workspace 内（防止通过 symlink 父目录逃逸）
        if let Some(parent) = canonical.parent() {
            if parent.exists() {
                validate_path_with_base(parent, &self.workspace_root)
                    .map_err(|e| {
                        tracing::error!(tool = "write_file", path = %path_display, error = %e, "[tool] parent path validation failed");
                        ToolError::PathTraversal
                    })?;
            }
        }

        // 若目标已存在且为符号链接，校验其目标仍在 workspace 内
        if canonical.exists() {
            check_symlink_target(canonical, &self.workspace_root)
                .map_err(|e| {
                    tracing::error!(tool = "write_file", path = %path_display, error = %e, "[tool] symlink target validation failed");
                    ToolError::PathTraversal
                })?;
        }

        let approved = self.approval.request_approval(
            "write_file",
            &serde_json::json!({ "path": args.path }),
        ).await;

        if !approved {
            tracing::error!(tool = "write_file", path = %path_display, "[tool] approval denied");
            return Err(ToolError::Execution("Permission denied".to_string()));
        }

        if let Some(parent) = canonical.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| {
                    tracing::error!(tool = "write_file", path = %path_display, error = %e, "[tool] create_dir_all failed");
                    ToolError::Io(e.to_string())
                })?;
        }

        tokio::fs::write(canonical, &args.content).await
            .map_err(|e| {
                tracing::error!(tool = "write_file", path = %path_display, error = %e, "[tool] write failed");
                ToolError::Io(e.to_string())
            })?;

        tracing::info!(tool = "write_file", path = %path_display, content_len, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(format!("Written {} bytes to {}", args.content.len(), args.path)))
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

#[async_trait]
impl Tool for CreateDirectoryTool {
    fn name(&self) -> &str {
        "create_directory"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_directory".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: CreateDirectoryArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        let path_display = path_relative_or_name(&args.path, &self.workspace_root);
        tracing::info!(tool = "create_directory", path = %path_display, "[tool] call");

        let path = resolve_path(&self.workspace_root, &args.path);

        // 路径校验：目标可能不存在，用 creation 校验
        let canonical = validate_path_for_creation(&path, &self.workspace_root)
            .map_err(|e| {
                tracing::error!(tool = "create_directory", path = %path_display, error = %e, "[tool] path validation failed");
                ToolError::PathTraversal
            })?;
        let canonical = canonical.as_path();

        // 父目录若存在，必须 canonicalize 后仍在 workspace 内
        if let Some(parent) = canonical.parent() {
            if parent.exists() {
                validate_path_with_base(parent, &self.workspace_root)
                    .map_err(|e| {
                        tracing::error!(tool = "create_directory", path = %path_display, error = %e, "[tool] parent path validation failed");
                        ToolError::PathTraversal
                    })?;
            }
        }

        // 若目标已存在且为符号链接，校验其目标仍在 workspace 内
        if canonical.exists() {
            check_symlink_target(canonical, &self.workspace_root)
                .map_err(|e| {
                    tracing::error!(tool = "create_directory", path = %path_display, error = %e, "[tool] symlink target validation failed");
                    ToolError::PathTraversal
                })?;
        }

        let approved = self.approval.request_approval(
            "create_directory",
            &serde_json::json!({ "path": args.path }),
        ).await;

        if !approved {
            tracing::error!(tool = "create_directory", path = %path_display, "[tool] approval denied");
            return Err(ToolError::Execution("Permission denied".to_string()));
        }

        tokio::fs::create_dir_all(canonical).await
            .map_err(|e| {
                tracing::error!(tool = "create_directory", path = %path_display, error = %e, "[tool] create_dir_all failed");
                ToolError::Io(e.to_string())
            })?;

        tracing::info!(tool = "create_directory", path = %path_display, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::Value::String(format!("Created directory {}", args.path)))
    }
}

// ── Helper ────────────────────────────────────────────────

fn resolve_path(workspace_root: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        workspace_root.join(p)
    }
}

fn path_relative_or_name(path: &str, workspace_root: &PathBuf) -> String {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        if let Ok(rel) = p.strip_prefix(workspace_root) {
            rel.to_string_lossy().to_string()
        } else {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.to_string())
        }
    } else {
        path.to_string()
    }
}
