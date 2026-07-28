//! ═══════════════════════════════════════════════════════════════════════════
//! 文件系统命令 - Tauri IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供 Tauri IPC 命令：
//! - fs_read_file: 读取文件
//! - fs_write_file: 写入文件
//! - fs_list_directory: 列出目录
//! - fs_create_directory: 创建目录
//! - fs_delete_file: 删除文件/目录
//! - fs_exists: 检查文件是否存在
//! - fs_copy_file: 复制文件
//! - editor_read_file: 编辑器读取文件
//! - editor_write_file: 编辑器写入文件

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::validation::validate_path;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;
use crate::security_kernel::{
    SecurityKernelState, OperationContext,
    WorkspaceId, UserId, SessionId,
};
use crate::security_kernel::permission::{Operation, FsScope, FsOperation};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::State;

// ── 常量 ────────────────────────────────────────────────────────────────────

/// 最大文件大小：10MB
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
/// 最大路径长度
const MAX_PATH_LEN: usize = 4096;

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 文件条目
#[derive(Serialize, Clone)]
pub struct FileEntry {
    /// 文件名
    pub name: String,
    /// 路径
    pub path: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 扩展名
    pub extension: Option<String>,
    /// 文件大小
    pub size: u64,
}

/// 编辑器文件内容
#[derive(Serialize, Clone)]
pub struct EditorFileContent {
    /// 文件内容
    pub content: String,
    /// 语言标识
    pub language: String,
    /// 文件大小
    pub size: u64,
    /// 文件路径
    pub path: String,
}

/// 最近文件
#[derive(Serialize, Clone)]
pub struct RecentFile {
    /// 路径
    pub path: String,
    /// 文件名
    pub name: String,
    /// 扩展名
    pub extension: Option<String>,
    /// 修改时间
    pub modified_at: u64,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 校验文件路径
fn validate_file_path(path: &str) -> Result<PathBuf, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > MAX_PATH_LEN {
        return Err(AppError::invalid_input(format!(
            "Path too long (max {} chars)", MAX_PATH_LEN
        )));
    }
    validate_path(path).map_err(AppError::invalid_input)?;

    let path_buf = PathBuf::from(path);
    if path_buf.components().any(|c| c.as_os_str() == "..") {
        return Err(AppError::invalid_input("Path traversal denied"));
    }
    Ok(path_buf)
}

/// 检查工作区授权
fn check_workspace_authorization(
    path: &PathBuf,
    registry: &WorkspaceRegistry,
) -> Result<(), AppError> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| AppError::internal(format!("Failed to canonicalize path: {}", e)))?;

    if !registry.is_authorized(&canonical) {
        return Err(AppError::forbidden("Path not in authorized workspace"));
    }
    Ok(())
}

/// 创建操作上下文
fn create_operation_context(workspace_id: Option<String>) -> OperationContext {
    let workspace = workspace_id
        .and_then(|s| uuid::Uuid::parse_str(&s).ok())
        .map(WorkspaceId)
        .unwrap_or_else(|| WorkspaceId(uuid::Uuid::nil()));

    OperationContext {
        workspace,
        user: UserId(uuid::Uuid::nil()),
        session: SessionId(uuid::Uuid::nil()),
        approval_token: None,
    }
}

/// 根据文件扩展名推断语言标识
fn detect_language(path: &Path) -> String {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    match ext.as_deref() {
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("rs") => "rust",
        Some("json") => "json",
        Some("md") => "markdown",
        Some("html") => "html",
        Some("css") => "css",
        Some("py") => "python",
        Some("toml") => "toml",
        Some("yaml") | Some("yml") => "yaml",
        Some("txt") => "text",
        _ => "text",
    }.to_string()
}

// ── Tauri 命令 ──────────────────────────────────────────────────────────────

/// 读取文件
#[tauri::command]
pub async fn fs_read_file(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<String>, AppError> {
    let path_buf = validate_file_path(&path)?;
    check_workspace_authorization(&path_buf, &workspace_registry)?;

    if !path_buf.exists() {
        return Err(AppError::not_found(format!("File not found: {}", path)));
    }
    if path_buf.is_dir() {
        return Err(AppError::invalid_input("Path is a directory, not a file"));
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Read,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let content = kernel.execute_blocking("fs_read_file", &op, &ctx, move || {
        let metadata = std::fs::metadata(&path_buf)
            .map_err(|e| AppError::internal(format!("Failed to get file metadata: {}", e)))?;
        if metadata.len() > MAX_FILE_SIZE as u64 {
            return Err(AppError::invalid_input(format!(
                "File too large (max {} bytes)", MAX_FILE_SIZE
            )));
        }
        std::fs::read_to_string(&path_buf)
            .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))
    }).await?;

    Ok(IpcResponse::ok(content))
}

/// 写入文件
#[tauri::command]
pub async fn fs_write_file(
    path: String,
    content: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<u64>, AppError> {
    let path_buf = validate_file_path(&path)?;

    if content.len() > MAX_FILE_SIZE {
        return Err(AppError::invalid_input(format!(
            "Content too large (max {} bytes)", MAX_FILE_SIZE
        )));
    }

    if let Some(parent) = path_buf.parent() {
        if parent.exists() {
            check_workspace_authorization(&parent.to_path_buf(), &workspace_registry)?;
        }
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Write,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let bytes = content.len() as u64;
    kernel.execute_blocking("fs_write_file", &op, &ctx, move || {
        std::fs::write(&path_buf, content)
            .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))
    }).await?;

    Ok(IpcResponse::ok(bytes))
}

/// 列出目录
#[tauri::command]
pub async fn fs_list_directory(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<Vec<FileEntry>>, AppError> {
    let start = Instant::now();
    tracing::info!(path = %path, "fs_list_directory: enter");
    
    let path_buf = validate_file_path(&path)?;
    check_workspace_authorization(&path_buf, &workspace_registry)?;
    tracing::debug!(path = %path_buf.display(), "fs_list_directory: path validated");

    if !path_buf.exists() {
        tracing::error!(path = %path, "fs_list_directory: directory not found");
        return Err(AppError::not_found(format!("Directory not found: {}", path)));
    }
    if !path_buf.is_dir() {
        tracing::error!(path = %path, "fs_list_directory: path is not a directory");
        return Err(AppError::invalid_input("Path is not a directory"));
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::List,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let entries = kernel.execute_blocking("fs_list_directory", &op, &ctx, move || {
        // 忽略的目录
        let ignore_dirs = [
            "node_modules", ".git", "target", "dist", ".next",
            "__pycache__", ".venv", "venv",
        ];

        let read_dir = std::fs::read_dir(&path_buf)
            .map_err(|e| AppError::internal(format!("Failed to read directory: {}", e)))?;

        let mut entries: Vec<FileEntry> = Vec::new();
        for entry in read_dir {
            let entry = entry.map_err(|e| AppError::internal(format!("Failed to read entry: {}", e)))?;
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') || ignore_dirs.contains(&name.as_str()) {
                continue;
            }

            let is_dir = entry.path().is_dir();
            let extension = entry.path().extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_string());
            let size = if !is_dir {
                entry.metadata().map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            entries.push(FileEntry {
                name,
                path: entry.path().to_string_lossy().to_string(),
                is_dir,
                extension,
                size,
            });
        }

        // 排序：目录在前，文件在后，同类型按名称排序
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }).await?;

    tracing::info!(
        path = %path,
        entries = entries.len(),
        duration_ms = start.elapsed().as_millis(),
        "fs_list_directory: exit"
    );
    Ok(IpcResponse::ok(entries))
}

/// 创建目录
#[tauri::command]
pub async fn fs_create_directory(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<bool>, AppError> {
    let path_buf = validate_file_path(&path)?;

    if let Some(parent) = path_buf.parent() {
        if parent.exists() {
            check_workspace_authorization(&parent.to_path_buf(), &workspace_registry)?;
        }
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::CreateDir,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let created = !path_buf.exists();
    kernel.execute_blocking("fs_create_directory", &op, &ctx, move || {
        std::fs::create_dir_all(&path_buf)
            .map_err(|e| AppError::internal(format!("Failed to create directory: {}", e)))
    }).await?;

    Ok(IpcResponse::ok(created))
}

/// 删除文件/目录
#[tauri::command]
pub async fn fs_delete_file(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<bool>, AppError> {
    let path_buf = validate_file_path(&path)?;
    check_workspace_authorization(&path_buf, &workspace_registry)?;

    if !path_buf.exists() {
        return Err(AppError::not_found(format!("File not found: {}", path)));
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Delete,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    kernel.execute_blocking("fs_delete_file", &op, &ctx, move || {
        if path_buf.is_dir() {
            std::fs::remove_dir_all(&path_buf)
                .map_err(|e| AppError::internal(format!("Failed to remove directory: {}", e)))?;
        } else {
            std::fs::remove_file(&path_buf)
                .map_err(|e| AppError::internal(format!("Failed to remove file: {}", e)))?;
        }
        Ok(true)
    }).await?;

    Ok(IpcResponse::ok(true))
}

/// 检查文件是否存在
#[tauri::command]
pub async fn fs_exists(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<bool>, AppError> {
    let path_buf = validate_file_path(&path)?;
    check_workspace_authorization(&path_buf, &workspace_registry)?;

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Read,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let exists = kernel.execute_blocking("fs_exists", &op, &ctx, move || {
        Ok(path_buf.exists())
    }).await?;

    Ok(IpcResponse::ok(exists))
}

/// 复制文件
#[tauri::command]
pub async fn fs_copy_file(
    source: String,
    destination: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<bool>, AppError> {
    let src_buf = validate_file_path(&source)?;
    let dst_buf = validate_file_path(&destination)?;

    check_workspace_authorization(&src_buf, &workspace_registry)?;
    if let Some(parent) = dst_buf.parent() {
        if parent.exists() {
            check_workspace_authorization(&parent.to_path_buf(), &workspace_registry)?;
        }
    }

    if !src_buf.exists() {
        return Err(AppError::not_found(format!("Source file not found: {}", source)));
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Write,
        path: destination.clone(),
    };

    let kernel = kernel_state.kernel();
    kernel.execute_blocking("fs_copy_file", &op, &ctx, move || {
        let metadata = std::fs::metadata(&src_buf)
            .map_err(|e| AppError::internal(format!("Failed to get source metadata: {}", e)))?;
        if metadata.len() > MAX_FILE_SIZE as u64 {
            return Err(AppError::invalid_input(format!(
                "Source file too large (max {} bytes)", MAX_FILE_SIZE
            )));
        }
        std::fs::copy(&src_buf, &dst_buf)
            .map_err(|e| AppError::internal(format!("Failed to copy file: {}", e)))?;
        Ok(true)
    }).await?;

    Ok(IpcResponse::ok(true))
}

/// 编辑器读取文件
#[tauri::command]
pub async fn editor_read_file(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<EditorFileContent>, AppError> {
    let path_buf = validate_file_path(&path)?;

    if !path_buf.exists() {
        return Err(AppError::not_found(format!("File not found: {}", path)));
    }
    if path_buf.is_dir() {
        return Err(AppError::invalid_input("Path is a directory, not a file"));
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Read,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let path_str = path.clone();
    let content = kernel.execute_blocking("editor_read_file", &op, &ctx, move || {
        let metadata = std::fs::metadata(&path_buf)
            .map_err(|e| AppError::internal(format!("Failed to get file metadata: {}", e)))?;
        if metadata.len() > MAX_FILE_SIZE as u64 {
            return Err(AppError::invalid_input(format!(
                "File too large (max {} bytes)", MAX_FILE_SIZE
            )));
        }
        let content = std::fs::read_to_string(&path_buf)
            .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))?;
        let language = detect_language(&path_buf);
        Ok(EditorFileContent {
            size: metadata.len(),
            content,
            language,
            path: path_str,
        })
    }).await?;

    Ok(IpcResponse::ok(content))
}

/// 编辑器写入文件
#[tauri::command]
pub async fn editor_write_file(
    path: String,
    content: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<u64>, AppError> {
    let path_buf = validate_file_path(&path)?;

    if content.len() > MAX_FILE_SIZE {
        return Err(AppError::invalid_input(format!(
            "Content too large (max {} bytes)", MAX_FILE_SIZE
        )));
    }

    let ctx = create_operation_context(workspace_id);
    let op = Operation::Filesystem {
        scope: FsScope::Workspace,
        operation: FsOperation::Write,
        path: path.clone(),
    };

    let kernel = kernel_state.kernel();
    let bytes = content.len() as u64;
    kernel.execute_blocking("editor_write_file", &op, &ctx, move || {
        // 创建父目录
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("Failed to create parent directory: {}", e)))?;
        }

        // 原子写入：先写临时文件，再重命名
        let tmp_path = format!(
            "{}.tmp{}",
            path_buf.to_string_lossy(),
            uuid::Uuid::new_v4().simple()
        );
        let tmp_buf = PathBuf::from(&tmp_path);
        std::fs::write(&tmp_buf, &content)
            .map_err(|e| AppError::internal(format!("Failed to write temp file: {}", e)))?;
        std::fs::rename(&tmp_buf, &path_buf)
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp_buf);
                AppError::internal(format!("Failed to rename temp file: {}", e))
            })?;

        Ok(bytes)
    }).await?;

    Ok(IpcResponse::ok(bytes))
}