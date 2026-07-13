
use serde::{Deserialize, Serialize};

/// Git 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitError {
    /// 错误信息
    pub message: String,
}

/// Git 提交信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// 完整哈希值
    pub hash: String,
    /// 短哈希值（前 7 位）
    pub short_hash: String,
    /// 作者名称
    pub author: String,
    /// 作者邮箱
    pub email: String,
    /// 提交时间
    pub date: String,
    /// 提交消息
    pub message: String,
}

/// Git 状态快照
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus {
    /// 当前分支
    pub branch: String,
    /// 已暂存的文件变更
    pub staged: Vec<FileChange>,
    /// 未暂存的文件变更
    pub unstaged: Vec<FileChange>,
    /// 未跟踪的文件
    pub untracked: Vec<String>,
    /// 工作区是否干净
    pub is_clean: bool,
}

/// 文件变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// 文件路径
    pub path: String,
    /// 变更状态（added/modified/deleted）
    pub status: String,
    /// 是否已暂存
    pub staged: bool,
}

/// 差异对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    /// 文件差异列表
    pub files: Vec<FileDiff>,
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
    /// 差异补丁内容
    pub patch: String,
}

/// Git 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitConfig {
    /// 用户名
    pub user_name: Option<String>,
    /// 用户邮箱
    pub user_email: Option<String>,
    /// 是否自动暂存
    pub auto_stage: bool,
    /// 提交消息模板
    pub commit_message_template: Option<String>,
    /// 是否启用远程操作
    pub enable_remote: bool,
}

/// Git 安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// 是否成功
    pub success: bool,
    /// 结果消息
    pub message: String,
    /// 安装的版本号
    pub version: Option<String>,
}

/// 回滚模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackMode {
    /// 软回滚（保留变更）
    Soft,
    /// 硬回滚（丢弃变更）
    Hard,
}

/// Git 初始化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInitResult {
    /// 是否已初始化
    pub initialized: bool,
    /// 仓库路径
    pub path: String,
}