use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::validation::validate_path;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;
use crate::security_kernel::{
    SecurityKernelState, OperationContext,
    WorkspaceId, UserId, SessionId,
};
use crate::security_kernel::permission::{Operation, FsScope, FsOperation};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
const MAX_PATH_LEN: usize = 4096;

#[derive(Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub size: u64,
}

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

    // metadata + read 一起卸载到阻塞线程池，避免 metadata 阻塞 worker
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

#[tauri::command]
pub async fn fs_list_directory(
    path: String,
    workspace_id: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
    workspace_registry: State<'_, WorkspaceRegistry>,
) -> Result<IpcResponse<Vec<FileEntry>>, AppError> {
    let path_buf = validate_file_path(&path)?;
    check_workspace_authorization(&path_buf, &workspace_registry)?;

    if !path_buf.exists() {
        return Err(AppError::not_found(format!("Directory not found: {}", path)));
    }
    if !path_buf.is_dir() {
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

        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }).await?;

    Ok(IpcResponse::ok(entries))
}

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
    // metadata + copy 一起卸载到阻塞线程池
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