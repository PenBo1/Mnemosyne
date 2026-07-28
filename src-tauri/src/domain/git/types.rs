//! ═══════════════════════════════════════════════════════════════════════════
//! Git 类型 - Git 模块类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── 提交相关 ────────────────────────────────────────────────────────────────

/// Git 提交信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// 完整哈希值
    pub id: String,
    /// 短哈希值
    pub short_id: String,
    /// 作者名称
    pub author: String,
    /// 作者邮箱
    pub author_email: String,
    /// 提交时间戳（Unix 时间戳）
    pub time: i64,
    /// 提交消息
    pub message: String,
}

// ── 状态相关 ────────────────────────────────────────────────────────────────

/// Git 状态快照
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus {
    /// 当前分支
    pub branch: String,
    /// 文件变更列表
    pub files: Vec<FileChange>,
    /// 领先远程提交数
    pub ahead: usize,
    /// 落后远程提交数
    pub behind: usize,
    /// 已暂存文件数
    pub staged: usize,
    /// 未暂存文件数
    pub unstaged: usize,
}

/// 文件状态类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatusType {
    /// 未修改
    Unmodified,
    /// 已添加
    Added,
    /// 已修改
    Modified,
    /// 已删除
    Deleted,
    /// 未跟踪
    Untracked,
    /// 已重命名
    Renamed,
    /// 已复制
    Copied,
    /// 有冲突
    Conflicted,
}

impl Default for FileStatusType {
    fn default() -> Self {
        Self::Unmodified
    }
}

impl std::fmt::Display for FileStatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unmodified => write!(f, "unmodified"),
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Untracked => write!(f, "untracked"),
            Self::Renamed => write!(f, "renamed"),
            Self::Copied => write!(f, "copied"),
            Self::Conflicted => write!(f, "conflicted"),
        }
    }
}

/// 文件变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// 文件路径
    pub path: String,
    /// 变更状态
    pub status: FileStatusType,
}

// ── 差异相关 ────────────────────────────────────────────────────────────────

/// 差异对比结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Diff {
    /// 文件差异列表
    pub files: Vec<FileDiff>,
    /// 总新增行数
    pub total_additions: u32,
    /// 总删除行数
    pub total_deletions: u32,
}

/// 单文件差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// 文件路径
    pub path: String,
    /// 新增行数
    pub additions: u32,
    /// 删除行数
    pub deletions: u32,
    /// 是否为二进制文件
    pub binary: bool,
}

// ── 配置相关 ────────────────────────────────────────────────────────────────

/// Git 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitConfig {
    /// 用户名
    pub user_name: Option<String>,
    /// 用户邮箱
    pub user_email: Option<String>,
    /// 自定义配置项
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

// ── 操作相关 ────────────────────────────────────────────────────────────────

/// 回滚模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackMode {
    /// 软回滚（保留工作区变更）
    Soft,
    /// 混合回滚（保留工作区变更，取消暂存）
    Mixed,
    /// 硬回滚（丢弃所有变更）
    Hard,
}

impl Default for RollbackMode {
    fn default() -> Self {
        Self::Mixed
    }
}

/// Git 初始化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInitResult {
    /// 是否已初始化（false 表示仓库已存在）
    pub initialized: bool,
    /// 仓库路径
    pub path: String,
}