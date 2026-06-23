use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

use crate::errors::AppError;
use crate::domain::agents::base::{ToolDefinition, ToolExecutor, ToolResult};

pub struct ReadFileTool {
    work_dir: PathBuf,
}

impl ReadFileTool {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, AppError> {
        if path.contains("..") {
            return Err(AppError::path_traversal());
        }
        let resolved = self.work_dir.join(path);
        let canonical = resolved.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::file_not_found(path)
            } else {
                AppError::file_read_error(path)
            }
        })?;
        if !canonical.starts_with(&self.work_dir) {
            return Err(AppError::path_traversal());
        }
        Ok(canonical)
    }
}

#[async_trait]
impl ToolExecutor for ReadFileTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Read the contents of a file. Path is relative to the working directory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("path"))?;

        let resolved = self.resolve_path(path)?;
        let content = fs::read_to_string(&resolved).await.map_err(|_| AppError::file_read_error(path))?;
        Ok(ToolResult {
            tool_call_id: String::new(),
            content,
            is_error: false,
        })
    }
}

pub struct WriteFileTool {
    work_dir: PathBuf,
}

impl WriteFileTool {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, AppError> {
        if path.contains("..") {
            return Err(AppError::path_traversal());
        }
        let resolved = self.work_dir.join(path);
        if let Some(parent) = resolved.parent() {
            let canonical_parent = parent.canonicalize().map_err(|_| AppError::file_write_error(path))?;
            if !canonical_parent.starts_with(&self.work_dir) {
                return Err(AppError::path_traversal());
            }
        }
        Ok(resolved)
    }
}

#[async_trait]
impl ToolExecutor for WriteFileTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Write content to a file. Creates parent directories if needed. Path is relative to the working directory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file to write"
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

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("path"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("content"))?;

        let resolved = self.resolve_path(path)?;

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).await.map_err(|_| AppError::file_write_error(path))?;
        }

        fs::write(&resolved, content).await.map_err(|_| AppError::file_write_error(path))?;

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("Successfully wrote {} bytes to {}", content.len(), path),
            is_error: false,
        })
    }
}

pub struct ListFilesTool {
    work_dir: PathBuf,
}

impl ListFilesTool {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, AppError> {
        if path.contains("..") {
            return Err(AppError::path_traversal());
        }
        let resolved = self.work_dir.join(path);
        let canonical = resolved.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::directory_not_found(path)
            } else {
                AppError::file_read_error(path)
            }
        })?;
        if !canonical.starts_with(&self.work_dir) {
            return Err(AppError::path_traversal());
        }
        Ok(canonical)
    }
}

#[async_trait]
impl ToolExecutor for ListFilesTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "List files and directories at a given path. Directories are marked with a trailing '/'. Path is relative to the working directory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the directory to list (default: working directory root)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let resolved = if path.is_empty() {
            self.work_dir.clone()
        } else {
            self.resolve_path(path)?
        };

        let mut entries = fs::read_dir(&resolved)
            .await
            .map_err(|_| AppError::directory_not_found(path))?;

        let mut names = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|_| AppError::file_read_error(path))?
        {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().await.map_err(|_| AppError::file_read_error(path))?;
            if metadata.is_dir() {
                names.push(format!("{}/", file_name));
            } else {
                names.push(file_name);
            }
        }

        names.sort();
        Ok(ToolResult {
            tool_call_id: String::new(),
            content: names.join("\n"),
            is_error: false,
        })
    }
}
