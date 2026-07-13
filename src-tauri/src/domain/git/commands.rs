use std::path::PathBuf;

use crate::domain::git::detector::detect_git;
use crate::domain::git::installer::{install_git, installer_command};
use crate::domain::git::operations::GitOperations;
use crate::domain::git::types::{
    Commit, Diff, GitConfig, GitInitResult, GitStatus,
    InstallResult, RollbackMode,
};
use crate::shared::error::{AppError, IpcResponse};
use crate::security_kernel::{
    SecurityKernelState, OperationContext,
    WorkspaceId, UserId, SessionId,
};
use crate::security_kernel::permission::{Operation, ShellScope, GitOperation};
use tauri::State;

const MAX_PATH_LEN: usize = 4096;

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

fn validate_workspace_path(workspace_path: &str) -> Result<PathBuf, AppError> {
    if workspace_path.trim().is_empty() {
        return Err(AppError::invalid_input("Workspace path cannot be empty"));
    }
    if workspace_path.len() > MAX_PATH_LEN {
        return Err(AppError::invalid_input("Workspace path too long"));
    }
    if workspace_path.contains("..") {
        return Err(AppError::path_traversal());
    }
    let path_buf = PathBuf::from(workspace_path);
    if !path_buf.exists() {
        return Err(AppError::not_found("Workspace path does not exist"));
    }
    if !path_buf.is_dir() {
        return Err(AppError::invalid_input("Workspace path is not a directory"));
    }
    Ok(path_buf)
}

#[tauri::command]
pub async fn git_check_installed(
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<bool>, AppError> {
    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git_readonly(),
        command: "git".to_string(),
        args: vec!["--version".to_string()],
    };

    let kernel = kernel_state.kernel();
    let version = kernel.execute("git_check_installed", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                Ok(detect_git().await)
            })
        })
    })?;
    Ok(IpcResponse::ok(version.is_some()))
}

#[tauri::command]
pub async fn git_install(
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<InstallResult>, AppError> {
    tracing::info!("Starting git installation");

    let (program, args) = installer_command();
    let ctx = create_operation_context(None);
    // 安装 git 属于 git 相关的高风险 Shell 操作；用 Clone 语义表示“获取/建立 git”。
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Clone]),
        command: program.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
    };

    let kernel = kernel_state.kernel();
    let result = kernel.execute("git_install", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                Ok(install_git().await)
            })
        })
    })?;

    if result.success {
        tracing::info!(version = ?result.version, "Git installation succeeded");
    } else {
        tracing::warn!(message = %result.message, "Git installation failed");
    }
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn git_init(
    workspace_path: String,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<GitInitResult>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    tracing::debug!(path = %path.display(), "git_init");

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Clone]),
        command: "git".to_string(),
        args: vec!["init".to_string()],
    };

    let kernel = kernel_state.kernel();
    let result = kernel.execute("git_init", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::init(&path).await
            })
        })
    })?;

    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn git_status(
    workspace_path: String,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<GitStatus>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Status]),
        command: "git".to_string(),
        args: vec!["status".to_string()],
    };

    let kernel = kernel_state.kernel();
    let status = kernel.execute("git_status", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::status(&path).await
            })
        })
    })?;

    Ok(IpcResponse::ok(status))
}

#[tauri::command]
pub async fn git_log(
    workspace_path: String,
    limit: Option<u32>,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Vec<Commit>>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let limit = limit.unwrap_or(50);

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Log]),
        command: "git".to_string(),
        args: vec!["log".to_string()],
    };

    let kernel = kernel_state.kernel();
    let commits = kernel.execute("git_log", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::log(&path, limit).await
            })
        })
    })?;

    Ok(IpcResponse::ok(commits))
}

#[tauri::command]
pub async fn git_diff(
    workspace_path: String,
    commit_hash: Option<String>,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Diff>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let hash_ref = commit_hash.as_deref();

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Diff]),
        command: "git".to_string(),
        args: vec!["diff".to_string()],
    };

    let kernel = kernel_state.kernel();
    let diff = kernel.execute("git_diff", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::diff(&path, hash_ref).await
            })
        })
    })?;

    Ok(IpcResponse::ok(diff))
}

#[tauri::command]
pub async fn git_stage(
    workspace_path: String,
    paths: Vec<String>,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<()>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    if paths.is_empty() {
        return Err(AppError::invalid_input("paths cannot be empty"));
    }
    for p in &paths {
        if p.is_empty() {
            return Err(AppError::invalid_input("path entry cannot be empty"));
        }
        if p.contains("..") {
            return Err(AppError::path_traversal());
        }
        if p.len() > MAX_PATH_LEN {
            return Err(AppError::invalid_input("path entry too long"));
        }
    }

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Commit]),
        command: "git".to_string(),
        args: vec!["add".to_string()],
    };

    let kernel = kernel_state.kernel();
    kernel.execute("git_stage", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::stage(&path, &paths).await
            })
        })
    })?;

    Ok(IpcResponse::no_content())
}

#[tauri::command]
pub async fn git_commit(
    workspace_path: String,
    message: String,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<String>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    if message.trim().is_empty() {
        return Err(AppError::invalid_input("Commit message cannot be empty"));
    }
    if message.len() > 8192 {
        return Err(AppError::invalid_input("Commit message too long (max 8192 chars)"));
    }

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Commit]),
        command: "git".to_string(),
        args: vec!["commit".to_string()],
    };

    let kernel = kernel_state.kernel();
    let hash = kernel.execute("git_commit", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::commit(&path, &message).await
            })
        })
    })?;

    Ok(IpcResponse::ok(hash))
}

#[tauri::command]
pub async fn git_rollback(
    workspace_path: String,
    commit_hash: String,
    mode: RollbackMode,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<()>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    if matches!(mode, RollbackMode::Hard) {
        tracing::warn!(
            workspace = %path.display(),
            commit_hash,
            "Hard rollback requested — working tree changes will be discarded"
        );
    }

    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Reset]),
        command: "git".to_string(),
        args: vec!["reset".to_string()],
    };

    let kernel = kernel_state.kernel();
    kernel.execute("git_rollback", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                GitOperations::rollback(&path, &commit_hash, mode).await
            })
        })
    })?;

    Ok(IpcResponse::no_content())
}

#[tauri::command]
pub async fn git_get_config(
    workspace_path: String,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<GitConfig>, AppError> {
    // workspace_path 为空时读取全局 git config（--global），否则读仓库级 config
    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Remote]),
        command: "git".to_string(),
        args: vec!["config".to_string()],
    };
    let kernel = kernel_state.kernel();
    if workspace_path.trim().is_empty() {
        let config = kernel.execute("git_get_config", &op, &ctx, || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    GitOperations::get_global_config().await
                })
            })
        })?;
        Ok(IpcResponse::ok(config))
    } else {
        let path = validate_workspace_path(&workspace_path)?;
        let config = kernel.execute("git_get_config", &op, &ctx, || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    GitOperations::get_config(&path).await
                })
            })
        })?;
        Ok(IpcResponse::ok(config))
    }
}

#[tauri::command]
pub async fn git_set_config(
    workspace_path: String,
    config: GitConfig,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<()>, AppError> {
    let ctx = create_operation_context(None);
    let op = Operation::Shell {
        scope: ShellScope::git(vec![GitOperation::Remote]),
        command: "git".to_string(),
        args: vec!["config".to_string()],
    };
    let kernel = kernel_state.kernel();
    if workspace_path.trim().is_empty() {
        kernel.execute("git_set_config", &op, &ctx, || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    GitOperations::set_global_config(&config).await
                })
            })
        })?;
    } else {
        let path = validate_workspace_path(&workspace_path)?;
        kernel.execute("git_set_config", &op, &ctx, || {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    GitOperations::set_config(&path, &config).await
                })
            })
        })?;
    }
    Ok(IpcResponse::no_content())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn make_path(p: &str) -> &Path {
        Path::new(p)
    }

    #[test]
    fn test_validate_workspace_path_empty() {
        let result = validate_workspace_path("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_INPUT");
    }

    #[test]
    fn test_validate_workspace_path_traversal() {
        let result = validate_workspace_path("/some/../path");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "PATH_TRAVERSAL");
    }

    #[test]
    fn test_validate_workspace_path_too_long() {
        let long_path = "a".repeat(MAX_PATH_LEN + 1);
        let result = validate_workspace_path(&long_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "INVALID_INPUT");
    }

    #[test]
    fn test_validate_workspace_path_whitespace_only() {
        let result = validate_workspace_path("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workspace_path_nonexistent() {
        let nonexistent = if cfg!(target_os = "windows") {
            "Z:\\nonexistent_mnemosyne_test_path"
        } else {
            "/nonexistent_mnemosyne_test_path"
        };
        let result = validate_workspace_path(nonexistent);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
    }

    #[test]
    fn test_make_path_basic() {
        #[cfg(windows)]
        let p = "C:\\";
        #[cfg(not(windows))]
        let p = "/tmp";
        let path = make_path(p);
        assert!(path.is_absolute());
    }
}