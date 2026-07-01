use std::path::PathBuf;

use crate::features::git::{
    detect_git, install_git, Commit, Diff, GitConfig, GitInitResult, GitService, GitStatus,
    InstallResult, RollbackMode,
};
use crate::shared::errors::{AppError, IpcResponse};

const MAX_PATH_LEN: usize = 4096;

/// Check whether git is installed and reachable on PATH.
#[tauri::command]
pub async fn git_check_installed() -> Result<IpcResponse<bool>, AppError> {
    let installed = detect_git().await.is_some();
    Ok(IpcResponse::ok(installed))
}

/// Attempt to install git using the platform's native package manager.
#[tauri::command]
pub async fn git_install() -> Result<IpcResponse<InstallResult>, AppError> {
    tracing::info!("Starting git installation");
    let result = install_git().await;
    if result.success {
        tracing::info!(version = ?result.version, "Git installation succeeded");
    } else {
        tracing::warn!(message = %result.message, "Git installation failed");
    }
    Ok(IpcResponse::ok(result))
}

/// Initialize a git repository at the given workspace path.
#[tauri::command]
pub async fn git_init(
    workspace_path: String,
) -> Result<IpcResponse<GitInitResult>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    tracing::debug!(path = %path.display(), "git_init");
    let result = GitService::init(&path).await?;
    Ok(IpcResponse::ok(result))
}

/// Snapshot the working tree state.
#[tauri::command]
pub async fn git_status(
    workspace_path: String,
) -> Result<IpcResponse<GitStatus>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let status = GitService::status(&path).await?;
    Ok(IpcResponse::ok(status))
}

/// List recent commits. `limit` defaults to 50 and is clamped to [1, 1000].
#[tauri::command]
pub async fn git_log(
    workspace_path: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<Commit>>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let limit = limit.unwrap_or(50);
    let commits = GitService::log(&path, limit).await?;
    Ok(IpcResponse::ok(commits))
}

/// Compute a diff. `commit_hash == None` shows uncommitted changes vs HEAD.
#[tauri::command]
pub async fn git_diff(
    workspace_path: String,
    commit_hash: Option<String>,
) -> Result<IpcResponse<Diff>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let hash_ref = commit_hash.as_deref();
    let diff = GitService::diff(&path, hash_ref).await?;
    Ok(IpcResponse::ok(diff))
}

/// Stage one or more paths. Use `["."]` to stage everything.
#[tauri::command]
pub async fn git_stage(
    workspace_path: String,
    paths: Vec<String>,
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
    GitService::stage(&path, &paths).await?;
    Ok(IpcResponse::no_content())
}

/// Create a commit. Returns the new commit's full hash.
#[tauri::command]
pub async fn git_commit(
    workspace_path: String,
    message: String,
) -> Result<IpcResponse<String>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    if message.trim().is_empty() {
        return Err(AppError::invalid_input("Commit message cannot be empty"));
    }
    if message.len() > 8192 {
        return Err(AppError::invalid_input("Commit message too long (max 8192 chars)"));
    }
    let hash = GitService::commit(&path, &message).await?;
    Ok(IpcResponse::ok(hash))
}

/// Reset HEAD to `commit_hash`. `Hard` mode discards working-tree changes —
/// the frontend is responsible for confirming this with the user before
/// invoking this command.
#[tauri::command]
pub async fn git_rollback(
    workspace_path: String,
    commit_hash: String,
    mode: RollbackMode,
) -> Result<IpcResponse<()>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    if matches!(mode, RollbackMode::Hard) {
        tracing::warn!(
            workspace = %path.display(),
            commit_hash,
            "Hard rollback requested — working tree changes will be discarded"
        );
    }
    GitService::rollback(&path, &commit_hash, mode).await?;
    Ok(IpcResponse::no_content())
}

/// Read the per-repository git configuration.
#[tauri::command]
pub async fn git_get_config(
    workspace_path: String,
) -> Result<IpcResponse<GitConfig>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let config = GitService::get_config(&path).await?;
    Ok(IpcResponse::ok(config))
}

/// Persist the per-repository git configuration.
#[tauri::command]
pub async fn git_set_config(
    workspace_path: String,
    config: GitConfig,
) -> Result<IpcResponse<()>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    GitService::set_config(&path, &config).await?;
    Ok(IpcResponse::no_content())
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Validate a workspace path received from the frontend.
///
/// - Non-empty, length-bounded.
/// - Rejects `..` traversal sequences (defensive — frontend should already
///   select an absolute path via the folder picker).
/// - Verifies the path exists and is a directory.
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
        // Use a path that should never exist.
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
        // Windows 用盘符路径，Unix 用 /tmp
        #[cfg(windows)]
        let p = "C:\\";
        #[cfg(not(windows))]
        let p = "/tmp";
        let path = make_path(p);
        assert!(path.is_absolute());
    }
}
