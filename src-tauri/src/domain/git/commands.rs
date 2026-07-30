//! ═══════════════════════════════════════════════════════════════════════════
//! Git 命令 - Git 模块 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;
use std::time::Instant;

use crate::domain::git::operations::{
    init_repository, get_status, get_log, get_diff, get_commit_diff,
    stage_files, unstage_files, commit_changes, rollback,
    get_config, set_config, get_all_config, get_branches,
};
use crate::domain::git::types::{
    Commit, Diff, GitConfig, GitInitResult, GitStatus, RollbackMode,
};
use crate::shared::error::{AppError, IpcResponse};

// ── 常量定义 ────────────────────────────────────────────────────────────────

const MAX_PATH_LEN: usize = 4096;

// ── 辅助函数 ────────────────────────────────────────────────────────────────

fn validate_workspace_path(workspace_path: &str) -> Result<PathBuf, AppError> {
    if workspace_path.trim().is_empty() {
        return Err(AppError::invalid_input("Workspace path cannot be empty"));
    }
    if workspace_path.len() > MAX_PATH_LEN {
        return Err(AppError::invalid_input("Workspace path too long"));
    }
    let path_buf = PathBuf::from(workspace_path);
    // 路径遍历防护
    if path_buf
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(AppError::path_traversal());
    }
    if !path_buf.exists() {
        return Err(AppError::not_found("Workspace path does not exist"));
    }
    if !path_buf.is_dir() {
        return Err(AppError::invalid_input("Workspace path is not a directory"));
    }
    Ok(path_buf)
}

// ── IPC 命令 ────────────────────────────────────────────────────────────────

/// 初始化 Git 仓库
#[tauri::command]
pub async fn git_init(
    workspace_path: String,
) -> Result<IpcResponse<GitInitResult>, AppError> {
    let start = Instant::now();
    let path = validate_workspace_path(&workspace_path)?;
    tracing::debug!(path = %path.display(), "git_init");

    let result = tokio::task::spawn_blocking(move || init_repository(&path))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    tracing::info!(
        initialized = result.initialized,
        duration_ms = start.elapsed().as_millis(),
        "git_init: exit"
    );
    Ok(IpcResponse::ok(result))
}

/// 获取 Git 状态
#[tauri::command]
pub async fn git_status(
    workspace_path: String,
) -> Result<IpcResponse<GitStatus>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let status = tokio::task::spawn_blocking(move || get_status(&path))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(status))
}

/// 获取提交历史
#[tauri::command]
pub async fn git_log(
    workspace_path: String,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<IpcResponse<Vec<Commit>>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let limit = limit.unwrap_or(50);
    let skip = skip.unwrap_or(0);
    let commits = tokio::task::spawn_blocking(move || get_log(&path, limit, skip))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(commits))
}

/// 获取差异
#[tauri::command]
pub async fn git_diff(
    workspace_path: String,
    staged: Option<bool>,
    commit_hash: Option<String>,
) -> Result<IpcResponse<Diff>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    
    let diff = tokio::task::spawn_blocking(move || {
        if let Some(hash) = commit_hash {
            get_commit_diff(&path, &hash)
        } else {
            get_diff(&path, staged.unwrap_or(false))
        }
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    
    Ok(IpcResponse::ok(diff))
}

/// 暂存文件
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
        if PathBuf::from(p)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(AppError::path_traversal());
        }
        if p.len() > MAX_PATH_LEN {
            return Err(AppError::invalid_input("path entry too long"));
        }
    }

    let paths_clone = paths.clone();
    tokio::task::spawn_blocking(move || stage_files(&path, &paths_clone))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::no_content())
}

/// 取消暂存文件
#[tauri::command]
pub async fn git_unstage(
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
        if PathBuf::from(p)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(AppError::path_traversal());
        }
        if p.len() > MAX_PATH_LEN {
            return Err(AppError::invalid_input("path entry too long"));
        }
    }

    let paths_clone = paths.clone();
    tokio::task::spawn_blocking(move || unstage_files(&path, &paths_clone))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::no_content())
}

/// 提交变更
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

    let hash = tokio::task::spawn_blocking(move || commit_changes(&path, &message))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(hash))
}

/// 回滚到指定提交
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

    tokio::task::spawn_blocking(move || rollback(&path, &commit_hash, mode))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::no_content())
}

/// 获取 Git 配置
#[tauri::command]
pub async fn git_get_config(
    workspace_path: String,
    key: Option<String>,
    global: Option<bool>,
) -> Result<IpcResponse<GitConfig>, AppError> {
    let is_global = global.unwrap_or(false);
    
    if let Some(k) = key {
        // 获取单个配置项
        let path = if is_global {
            std::env::current_dir().unwrap_or_default()
        } else {
            validate_workspace_path(&workspace_path)?
        };
        
        let k_clone = k.clone();
        let value = tokio::task::spawn_blocking(move || get_config(&path, &k_clone, is_global))
            .await
            .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
        
        let config = GitConfig {
            user_name: if k == "user.name" { value.clone() } else { None },
            user_email: if k == "user.email" { value } else { None },
            custom: Default::default(),
        };
        Ok(IpcResponse::ok(config))
    } else {
        // 获取所有配置
        if is_global {
            let config = GitConfig {
                user_name: None,
                user_email: None,
                custom: Default::default(),
            };
            Ok(IpcResponse::ok(config))
        } else {
            let path = validate_workspace_path(&workspace_path)?;
            let config = tokio::task::spawn_blocking(move || get_all_config(&path))
                .await
                .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
            Ok(IpcResponse::ok(config))
        }
    }
}

/// 设置 Git 配置
#[tauri::command]
pub async fn git_set_config(
    workspace_path: String,
    key: String,
    value: String,
    global: Option<bool>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_path, key, global, "git_set_config: enter");

    let is_global = global.unwrap_or(false);
    
    let path = if is_global {
        std::env::current_dir().unwrap_or_default()
    } else {
        validate_workspace_path(&workspace_path)?
    };
    
    tokio::task::spawn_blocking(move || set_config(&path, &key, &value, is_global))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "git_set_config: exit"
    );
    Ok(IpcResponse::no_content())
}

/// 获取分支列表
#[tauri::command]
pub async fn git_branches(
    workspace_path: String,
) -> Result<IpcResponse<Vec<String>>, AppError> {
    let path = validate_workspace_path(&workspace_path)?;
    let branches = tokio::task::spawn_blocking(move || get_branches(&path))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(branches))
}

// ── 测试 ────────────────────────────────────────────────────────────────────

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