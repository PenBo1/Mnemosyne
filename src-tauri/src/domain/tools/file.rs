use std::fs;
use std::path::Path;
use super::types::*;
use crate::errors::AppError;
use crate::infra::llm::ToolSpec;

pub struct ReadFileTool;

impl ToolExecutor for ReadFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file".into(),
            description: "Read the contents of a file at the given path".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or relative file path" },
                    "offset": { "type": "integer", "description": "Line number to start reading from (0-indexed)" },
                    "limit": { "type": "integer", "description": "Maximum number of lines to read" }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let path = call.args["path"].as_str()
            .ok_or_else(|| AppError::missing_field("path"))?;
        let offset = call.args["offset"].as_u64().unwrap_or(0) as usize;
        let limit = call.args["limit"].as_u64().unwrap_or(2000) as usize;
        let resolved = resolve_path(path, &ctx.work_dir);
        tracing::debug!(path = %path, resolved = %resolved, offset, limit, "read_file");
        if let Some(ref sandbox) = ctx.sandbox {
            sandbox.validate_file_operation(&std::path::PathBuf::from(&resolved), false)
                .map_err(|v| {
                    tracing::warn!(path = %resolved, reason = ?v, "Sandbox denied read");
                    AppError::sandbox_violation(format!("{:?}", v))
                })?;
        }
        let content = fs::read_to_string(&resolved)
            .map_err(|e| {
                tracing::error!(error = %e, path = %resolved, "Failed to read file");
                AppError::file_read_error(resolved)
            })?;
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.min(lines.len());
        let end = (start + limit).min(lines.len());
        let selected: Vec<String> = lines[start..end].iter().enumerate()
            .map(|(i, l)| format!("{}: {}", start + i + 1, l)).collect();
        tracing::debug!(path = %path, total_lines = lines.len(), returned = end - start, "read_file completed");
        Ok(ToolOutput::success(selected.join("\n")))
    }
}

pub struct WriteFileTool;

impl ToolExecutor for WriteFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_file".into(),
            description: "Write content to a file, creating it if it doesn't exist".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to write to" },
                    "content": { "type": "string", "description": "Content to write" }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let path = call.args["path"].as_str()
            .ok_or_else(|| AppError::missing_field("path"))?;
        let content = call.args["content"].as_str()
            .ok_or_else(|| AppError::missing_field("content"))?;
        let resolved = resolve_path(path, &ctx.work_dir);
        tracing::info!(path = %path, resolved = %resolved, content_len = content.len(), "write_file");
        if let Some(ref sandbox) = ctx.sandbox {
            sandbox.validate_file_operation(&std::path::PathBuf::from(&resolved), true)
                .map_err(|v| {
                    tracing::warn!(path = %resolved, reason = ?v, "Sandbox denied write");
                    AppError::sandbox_violation(format!("{:?}", v))
                })?;
        }
        if let Some(parent) = Path::new(&resolved).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| {
                    tracing::error!(error = %e, path = %resolved, "Failed to create directory");
                    AppError::file_write_error(resolved.clone())
                })?;
        }
        fs::write(&resolved, content)
            .map_err(|e| {
                tracing::error!(error = %e, path = %resolved, "Failed to write file");
                AppError::file_write_error(resolved)
            })?;
        tracing::info!(path = %path, bytes = content.len(), "write_file completed");
        Ok(ToolOutput::success(format!("Written {} bytes to {}", content.len(), path)))
    }
}

pub struct ListDirTool;

impl ToolExecutor for ListDirTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_dir".into(),
            description: "List files and directories at the given path".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path (default: working directory)" }
                }
            }),
        }
    }

    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let path = call.args["path"].as_str().unwrap_or(".");
        let resolved = resolve_path(path, &ctx.work_dir);
        tracing::debug!(path = %path, resolved = %resolved, "list_dir");
        if let Some(ref sandbox) = ctx.sandbox {
            sandbox.validate_file_operation(&std::path::PathBuf::from(&resolved), false)
                .map_err(|v| {
                    tracing::warn!(path = %resolved, reason = ?v, "Sandbox denied list");
                    AppError::sandbox_violation(format!("{:?}", v))
                })?;
        }
        let entries = fs::read_dir(&resolved)
            .map_err(|e| {
                tracing::error!(error = %e, path = %resolved, "Failed to read directory");
                AppError::directory_not_found(resolved)
            })?;
        let mut items: Vec<String> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| AppError::internal(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type()
                .map_err(|e| AppError::internal(e.to_string()))?;
            let prefix = if file_type.is_dir() { "[dir] " } else { "      " };
            items.push(format!("{}{}", prefix, name));
        }
        items.sort();
        tracing::debug!(path = %path, count = items.len(), "list_dir completed");
        Ok(ToolOutput::success(items.join("\n")))
    }
}

fn resolve_path(path: &str, work_dir: &str) -> String {
    let p = Path::new(path);
    if p.is_absolute() { path.to_string() } else { Path::new(work_dir).join(path).to_string_lossy().to_string() }
}
