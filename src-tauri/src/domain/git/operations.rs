//! ═══════════════════════════════════════════════════════════════════════════
//! Git 操作 - 基于 git2 库实现
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;
use std::collections::HashMap;

use git2::{
    Repository, Oid, Signature, StatusOptions, ResetType,
    BranchType,
};
use tracing::info;

use super::types::*;
use crate::shared::error::AppError;

// ── 仓库初始化 ────────────────────────────────────────────────────────────────

/// 初始化 Git 仓库
pub fn init_repository(path: &Path) -> Result<GitInitResult, AppError> {
    info!(path = %path.display(), "初始化 Git 仓库");
    
    if path.join(".git").exists() {
        return Ok(GitInitResult {
            initialized: false,
            path: path.to_string_lossy().to_string(),
        });
    }
    
    Repository::init(path)
        .map_err(|e| AppError::internal(format!("初始化仓库失败: {}", e)))?;
    
    info!(path = %path.display(), "Git 仓库初始化成功");
    Ok(GitInitResult {
        initialized: true,
        path: path.to_string_lossy().to_string(),
    })
}

/// 打开现有仓库
fn open_repository(path: &Path) -> Result<Repository, AppError> {
    Repository::discover(path)
        .map_err(|e| AppError::not_found(format!("未找到 Git 仓库: {}", e)))
}

// ── 状态查询 ────────────────────────────────────────────────────────────────

/// 获取仓库状态
pub fn get_status(path: &Path) -> Result<GitStatus, AppError> {
    let repo = open_repository(path)?;
    
    // 获取当前分支
    let branch = get_current_branch(&repo)?;
    
    // 获取文件状态
    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .include_ignored(false)
        .include_unmodified(false)
        .exclude_submodules(false)
        .recurse_untracked_dirs(true);
    
    let statuses = repo.statuses(Some(&mut status_options))
        .map_err(|e| AppError::internal(format!("获取状态失败: {}", e)))?;
    
    let mut files = Vec::new();
    for entry in statuses.iter() {
        let file_path = entry.path().unwrap_or("").to_string();
        let status = status_from_git2(entry.status());
        
        // 跳过未修改的文件
        if status == FileStatusType::Unmodified {
            continue;
        }
        
        files.push(FileChange {
            path: file_path,
            status,
        });
    }
    
    // 统计
    let staged = files.iter().filter(|f| matches!(f.status, 
        FileStatusType::Added | FileStatusType::Modified | FileStatusType::Deleted
    )).count();
    let unstaged = files.len() - staged;
    
    Ok(GitStatus {
        branch,
        files,
        ahead: 0,
        behind: 0,
        staged,
        unstaged,
    })
}

/// 获取当前分支名
fn get_current_branch(repo: &Repository) -> Result<String, AppError> {
    // 空仓库（ unborn branch ）时 head() 会失败，返回默认分支名
    match repo.head() {
        Ok(head) => {
            let branch_name = head.shorthand()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "HEAD".to_string());
            Ok(branch_name)
        }
        Err(_) => {
            // 尝试获取 HEAD 文件内容（ unborn branch 情况）
            if let Ok(head_content) = std::fs::read_to_string(repo.path().join("HEAD")) {
                if head_content.starts_with("ref: refs/heads/") {
                    return Ok(head_content.trim()
                        .strip_prefix("ref: refs/heads/")
                        .unwrap_or("master")
                        .to_string());
                }
            }
            Ok("master".to_string())
        }
    }
}

/// 将 git2 状态转换为自定义状态
fn status_from_git2(status: git2::Status) -> FileStatusType {
    if status.is_index_new() { FileStatusType::Added }
    else if status.is_index_modified() { FileStatusType::Modified }
    else if status.is_index_deleted() { FileStatusType::Deleted }
    else if status.is_wt_new() { FileStatusType::Untracked }
    else if status.is_wt_modified() { FileStatusType::Modified }
    else if status.is_wt_deleted() { FileStatusType::Deleted }
    else if status.is_conflicted() { FileStatusType::Conflicted }
    else { FileStatusType::Unmodified }
}

// ── 提交历史 ────────────────────────────────────────────────────────────────

/// 获取提交历史
pub fn get_log(path: &Path, limit: usize, skip: usize) -> Result<Vec<Commit>, AppError> {
    let repo = open_repository(path)?;
    
    let mut revwalk = repo.revwalk()
        .map_err(|e| AppError::internal(format!("创建 revwalk 失败: {}", e)))?;
    
    revwalk.push_head()
        .map_err(|e| AppError::internal(format!("推送 HEAD 失败: {}", e)))?;
    
    let mut commits = Vec::new();
    for (idx, oid_result) in revwalk.enumerate() {
        if idx >= skip + limit {
            break;
        }
        if idx < skip {
            continue;
        }
        
        let oid = oid_result
            .map_err(|e| AppError::internal(format!("获取 OID 失败: {}", e)))?;
        
        let git_commit = repo.find_commit(oid)
            .map_err(|e| AppError::internal(format!("查找提交失败: {}", e)))?;
        
        commits.push(commit_to_info(&git_commit));
    }
    
    Ok(commits)
}

/// 将 git2::Commit 转换为自定义 Commit 类型
fn commit_to_info(git_commit: &git2::Commit) -> Commit {
    let author = git_commit.author();
    let author_name = author.name().unwrap_or("Unknown").to_string();
    let author_email = author.email().unwrap_or("").to_string();
    let commit_id = git_commit.id().to_string();
    
    Commit {
        id: commit_id.clone(),
        short_id: if commit_id.len() >= 7 { commit_id[..7].to_string() } else { commit_id },
        message: git_commit.message().unwrap_or("").to_string(),
        author: author_name,
        author_email,
        time: git_commit.time().seconds(),
    }
}

// ── 差异查看 ────────────────────────────────────────────────────────────────

/// 获取工作区与暂存区的差异
pub fn get_diff(path: &Path, staged: bool) -> Result<Diff, AppError> {
    let repo = open_repository(path)?;
    
    let mut diff_options = git2::DiffOptions::new();
    
    let diff = if staged {
        // 暂存区与 HEAD 的差异
        let head = repo.head()
            .map_err(|e| AppError::internal(format!("获取 HEAD 失败: {}", e)))?;
        let head_tree = head.peel_to_tree()
            .map_err(|e| AppError::internal(format!("获取 HEAD 树失败: {}", e)))?;
        let mut index = repo.index()
            .map_err(|e| AppError::internal(format!("获取索引失败: {}", e)))?;
        let index_tree_id = index.write_tree()
            .map_err(|e| AppError::internal(format!("写入树失败: {}", e)))?;
        let index_tree = repo.find_tree(index_tree_id)
            .map_err(|e| AppError::internal(format!("查找树失败: {}", e)))?;
        
        repo.diff_tree_to_tree(Some(&head_tree), Some(&index_tree), Some(&mut diff_options))
            .map_err(|e| AppError::internal(format!("获取暂存差异失败: {}", e)))?
    } else {
        // 工作区与暂存区的差异
        repo.diff_index_to_workdir(None, Some(&mut diff_options))
            .map_err(|e| AppError::internal(format!("获取工作区差异失败: {}", e)))?
    };
    
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            let file_path = delta.new_file().path().unwrap_or(Path::new("")).to_string_lossy().to_string();
            
            files.push(FileDiff {
                path: file_path,
                additions: 0,
                deletions: 0,
                binary: delta.new_file().is_binary(),
            });
            true
        },
        None,
        None,
        None,
    ).map_err(|e| AppError::internal(format!("遍历差异失败: {}", e)))?;
    
    Ok(Diff {
        files: files.clone(),
        total_additions: files.iter().map(|f| f.additions).sum(),
        total_deletions: files.iter().map(|f| f.deletions).sum(),
    })
}

/// 获取指定提交的差异
pub fn get_commit_diff(path: &Path, commit_id: &str) -> Result<Diff, AppError> {
    let repo = open_repository(path)?;
    
    let oid = Oid::from_str(commit_id)
        .map_err(|e| AppError::bad_request(format!("无效的提交 ID: {}", e)))?;
    
    let git_commit = repo.find_commit(oid)
        .map_err(|e| AppError::not_found(format!("提交不存在: {}", e)))?;
    
    let tree = git_commit.tree()
        .map_err(|e| AppError::internal(format!("获取树失败: {}", e)))?;
    
    let parent = git_commit.parent(0).ok();
    let parent_tree = parent.as_ref()
        .map(|p| p.tree())
        .transpose()
        .map_err(|e| AppError::internal(format!("获取父树失败: {}", e)))?;
    
    let mut diff_options = git2::DiffOptions::new();
    let parent_tree_ref: Option<&git2::Tree> = parent_tree.as_ref();
    let diff = repo.diff_tree_to_tree(parent_tree_ref, Some(&tree), Some(&mut diff_options))
        .map_err(|e| AppError::internal(format!("获取提交差异失败: {}", e)))?;
    
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            let file_path = delta.new_file().path().unwrap_or(Path::new("")).to_string_lossy().to_string();
            files.push(FileDiff {
                path: file_path,
                additions: 0,
                deletions: 0,
                binary: delta.new_file().is_binary(),
            });
            true
        },
        None,
        None,
        None,
    ).map_err(|e| AppError::internal(format!("遍历差异失败: {}", e)))?;
    
    Ok(Diff {
        files,
        total_additions: 0,
        total_deletions: 0,
    })
}

// ── 暂存操作 ────────────────────────────────────────────────────────────────

/// 暂存文件
pub fn stage_files(path: &Path, files: &[String]) -> Result<(), AppError> {
    let repo = open_repository(path)?;
    let mut index = repo.index()
        .map_err(|e| AppError::internal(format!("获取索引失败: {}", e)))?;
    
    for file in files {
        index.add_path(Path::new(file))
            .map_err(|e| AppError::internal(format!("暂存文件失败 {}: {}", file, e)))?;
    }
    
    index.write()
        .map_err(|e| AppError::internal(format!("写入索引失败: {}", e)))?;
    
    info!(files = ?files, "文件暂存成功");
    Ok(())
}

/// 取消暂存文件
pub fn unstage_files(path: &Path, files: &[String]) -> Result<(), AppError> {
    let repo = open_repository(path)?;
    
    let head = repo.head()
        .map_err(|e| AppError::internal(format!("获取 HEAD 失败: {}", e)))?;
    let head_commit = head.peel_to_commit()
        .map_err(|e| AppError::internal(format!("获取 HEAD 提交失败: {}", e)))?;
    let head_tree = head_commit.tree()
        .map_err(|e| AppError::internal(format!("获取 HEAD 树失败: {}", e)))?;
    
    // 使用 reset_default 取消暂存，需要传入 Object 引用
    let head_obj = head_tree.as_object();
    repo.reset_default(Some(head_obj), files.iter().map(|s| s.as_str()))
        .map_err(|e| AppError::internal(format!("取消暂存失败: {}", e)))?;
    
    info!(files = ?files, "取消暂存成功");
    Ok(())
}

// ── 提交操作 ────────────────────────────────────────────────────────────────

/// 提交更改
pub fn commit_changes(path: &Path, message: &str) -> Result<String, AppError> {
    let repo = open_repository(path)?;
    
    // 获取签名
    let signature = get_signature(&repo)?;
    
    // 获取树
    let mut index = repo.index()
        .map_err(|e| AppError::internal(format!("获取索引失败: {}", e)))?;
    
    let tree_id = index.write_tree()
        .map_err(|e| AppError::internal(format!("写入树失败: {}", e)))?;
    let tree = repo.find_tree(tree_id)
        .map_err(|e| AppError::internal(format!("查找树失败: {}", e)))?;
    
    // 获取父提交
    let parent = repo.head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok());
    
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    
    // 创建提交
    let commit_id = repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
        .map_err(|e| AppError::internal(format!("提交失败: {}", e)))?;
    
    info!(commit_id = %commit_id, message, "提交成功");
    Ok(commit_id.to_string())
}

/// 获取签名（优先使用仓库配置，其次使用默认值）
fn get_signature(repo: &Repository) -> Result<Signature<'static>, AppError> {
    let signature = repo.signature()
        .map_err(|e| AppError::internal(format!("获取签名失败: {}", e)))?;
    
    // 复制签名数据到 'static 生命周期
    let name = signature.name().unwrap_or("Mnemosyne").to_string();
    let email = signature.email().unwrap_or("mnemosyne@local").to_string();
    
    Signature::now(&name, &email)
        .map_err(|e| AppError::internal(format!("创建签名失败: {}", e)))
}

// ── 回滚操作 ────────────────────────────────────────────────────────────────

/// 回滚到指定提交
pub fn rollback(path: &Path, commit_id: &str, mode: RollbackMode) -> Result<(), AppError> {
    let repo = open_repository(path)?;
    
    let oid = Oid::from_str(commit_id)
        .map_err(|e| AppError::bad_request(format!("无效的提交 ID: {}", e)))?;
    
    let git_commit = repo.find_commit(oid)
        .map_err(|e| AppError::not_found(format!("提交不存在: {}", e)))?;
    
    let reset_type = match mode {
        RollbackMode::Soft => ResetType::Soft,
        RollbackMode::Mixed => ResetType::Mixed,
        RollbackMode::Hard => ResetType::Hard,
    };
    
    let commit_obj = git_commit.as_object();
    repo.reset(commit_obj, reset_type, None)
        .map_err(|e| AppError::internal(format!("回滚失败: {}", e)))?;
    
    info!(commit_id, mode = ?mode, "回滚成功");
    Ok(())
}

// ── 配置操作 ────────────────────────────────────────────────────────────────

/// 获取 Git 配置
pub fn get_config(path: &Path, key: &str, global: bool) -> Result<Option<String>, AppError> {
    let config = if global {
        git2::Config::open_default()
            .map_err(|e| AppError::internal(format!("打开全局配置失败: {}", e)))?
    } else {
        let repo = open_repository(path)?;
        repo.config()
            .map_err(|e| AppError::internal(format!("打开仓库配置失败: {}", e)))?
    };
    
    let value = config.get_string(key).ok();
    Ok(value)
}

/// 设置 Git 配置
pub fn set_config(path: &Path, key: &str, value: &str, global: bool) -> Result<(), AppError> {
    if global {
        let mut config = git2::Config::open_default()
            .map_err(|e| AppError::internal(format!("打开全局配置失败: {}", e)))?;
        
        config.set_str(key, value)
            .map_err(|e| AppError::internal(format!("设置配置失败: {}", e)))?;
    } else {
        let repo = open_repository(path)?;
        let mut config = repo.config()
            .map_err(|e| AppError::internal(format!("打开仓库配置失败: {}", e)))?;
        
        config.set_str(key, value)
            .map_err(|e| AppError::internal(format!("设置配置失败: {}", e)))?;
    }
    
    info!(key, value, global, "配置设置成功");
    Ok(())
}

/// 获取所有配置
pub fn get_all_config(path: &Path) -> Result<GitConfig, AppError> {
    let repo = open_repository(path)?;
    let config = repo.config()
        .map_err(|e| AppError::internal(format!("打开仓库配置失败: {}", e)))?;
    
    let mut result = HashMap::new();
    
    // 获取常用配置项
    if let Ok(name) = config.get_string("user.name") {
        result.insert("user.name".to_string(), name);
    }
    if let Ok(email) = config.get_string("user.email") {
        result.insert("user.email".to_string(), email);
    }
    
    // 尝试获取其他配置项
    let entries = config.entries(None)
        .map_err(|e| AppError::internal(format!("获取配置条目失败: {}", e)))?;
    
    // 手动迭代 ConfigEntries
    let mut iter = entries;
    while let Some(entry_result) = iter.next() {
        match entry_result {
            Ok(entry) => {
                if let Some(name) = entry.name() {
                    if let Some(value) = entry.value() {
                        result.insert(name.to_string(), value.to_string());
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "获取配置条目时出错");
                continue;
            }
        }
    }
    
    Ok(GitConfig {
        user_name: result.get("user.name").cloned(),
        user_email: result.get("user.email").cloned(),
        custom: result,
    })
}

// ── 分支操作 ────────────────────────────────────────────────────────────────

/// 获取所有分支
pub fn get_branches(path: &Path) -> Result<Vec<String>, AppError> {
    let repo = open_repository(path)?;
    
    let branches = repo.branches(Some(BranchType::Local))
        .map_err(|e| AppError::internal(format!("获取分支失败: {}", e)))?;
    
    let mut result = Vec::new();
    for branch_result in branches {
        let (branch, _) = branch_result.map_err(|e| AppError::internal(format!("获取分支失败: {}", e)))?;
        if let Some(name) = branch.name().map_err(|e| AppError::internal(format!("获取分支名失败: {}", e)))? {
            result.push(name.to_string());
        }
    }
    
    Ok(result)
}