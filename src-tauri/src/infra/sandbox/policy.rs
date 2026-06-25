use serde::{Deserialize, Serialize};

/// 文件系统权限动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsAction {
    Allow,
    Deny,
    Ask,
}

/// 命令执行权限动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecAction {
    Allow,
    Deny,
    Ask,
}

/// 网络权限动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetAction {
    Allow,
    Deny,
    Ask,
}

/// 沙箱安全级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// 无限制（仅用于完全信任的操作）
    Unrestricted = 0,
    /// 受限 - 可在工作目录内自由读写
    Restricted = 1,
    /// 严格 - 只读访问，写操作需要审批
    Strict = 2,
    /// 隔离 - 完全隔离，只能访问特定资源
    Isolated = 3,
}

/// 文件系统沙箱规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsRule {
    pub pattern: String,
    pub action: FsAction,
    pub description: Option<String>,
}

/// 命令执行沙箱规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRule {
    pub pattern: String,
    pub action: ExecAction,
    pub description: Option<String>,
}

/// 网络沙箱规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetRule {
    pub host: String,
    pub action: NetAction,
    pub description: Option<String>,
}

/// 完整的沙箱策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub name: String,
    pub description: String,
    pub level: SecurityLevel,
    pub fs_rules: Vec<FsRule>,
    pub exec_rules: Vec<ExecRule>,
    pub net_rules: Vec<NetRule>,
    pub max_exec_timeout_secs: u64,
    pub max_output_bytes: usize,
    pub env_blacklist: Vec<String>,
    pub resource_limits: ResourceLimits,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_secs: u64,
    pub max_file_size_mb: u64,
    pub max_open_files: u32,
    pub max_nesting_depth: u32,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::restricted()
    }
}

impl SandboxPolicy {
    /// 创建默认的受限策略（适用于大多数 AI Agent 操作）
    pub fn restricted() -> Self {
        Self {
            name: "restricted".into(),
            description: "受限策略 - 允许工作目录内读写，禁止危险命令和网络访问".into(),
            level: SecurityLevel::Restricted,
            fs_rules: vec![
                FsRule {
                    pattern: "/*".into(),
                    action: FsAction::Deny,
                    description: Some("默认拒绝所有路径".into()),
                },
                FsRule {
                    pattern: "${WORKDIR}/**".into(),
                    action: FsAction::Allow,
                    description: Some("工作目录内读写".into()),
                },
                FsRule {
                    pattern: "${APPDATA}/**".into(),
                    action: FsAction::Allow,
                    description: Some("应用数据目录".into()),
                },
                FsRule {
                    pattern: "${TMPDIR}/**".into(),
                    action: FsAction::Allow,
                    description: Some("临时目录".into()),
                },
            ],
            exec_rules: vec![
                ExecRule {
                    pattern: "ls".into(),
                    action: ExecAction::Allow,
                    description: Some("列出文件".into()),
                },
                ExecRule {
                    pattern: "cat".into(),
                    action: ExecAction::Allow,
                    description: Some("读取文件".into()),
                },
                ExecRule {
                    pattern: "grep".into(),
                    action: ExecAction::Allow,
                    description: Some("搜索文件内容".into()),
                },
                ExecRule {
                    pattern: "find".into(),
                    action: ExecAction::Allow,
                    description: Some("查找文件".into()),
                },
                ExecRule {
                    pattern: "git status".into(),
                    action: ExecAction::Allow,
                    description: Some("查看 git 状态".into()),
                },
                ExecRule {
                    pattern: "git diff".into(),
                    action: ExecAction::Allow,
                    description: Some("查看 git diff".into()),
                },
                ExecRule {
                    pattern: "git log".into(),
                    action: ExecAction::Allow,
                    description: Some("查看 git log".into()),
                },
                ExecRule {
                    pattern: "rm".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止删除文件".into()),
                },
                ExecRule {
                    pattern: "rmdir".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止删除目录".into()),
                },
                ExecRule {
                    pattern: "sudo".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止 sudo".into()),
                },
                ExecRule {
                    pattern: "chmod".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止修改权限".into()),
                },
                ExecRule {
                    pattern: "curl".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止网络请求".into()),
                },
                ExecRule {
                    pattern: "wget".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止网络下载".into()),
                },
                ExecRule {
                    pattern: "nc".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止 netcat".into()),
                },
                ExecRule {
                    pattern: "python -c".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止 Python 代码执行".into()),
                },
                ExecRule {
                    pattern: "node -e".into(),
                    action: ExecAction::Deny,
                    description: Some("禁止 Node.js 代码执行".into()),
                },
            ],
            net_rules: vec![
                NetRule {
                    host: "localhost".into(),
                    action: NetAction::Allow,
                    description: Some("允许本地访问".into()),
                },
                NetRule {
                    host: "127.0.0.1".into(),
                    action: NetAction::Allow,
                    description: Some("允许本地访问".into()),
                },
                NetRule {
                    host: "*".into(),
                    action: NetAction::Deny,
                    description: Some("禁止外部网络访问".into()),
                },
            ],
            max_exec_timeout_secs: 60,
            max_output_bytes: 1024 * 1024, // 1MB
            env_blacklist: vec![
                "API_KEY".into(),
                "SECRET".into(),
                "PASSWORD".into(),
                "TOKEN".into(),
                "PRIVATE_KEY".into(),
            ],
            resource_limits: ResourceLimits {
                max_memory_mb: 512,
                max_cpu_secs: 30,
                max_file_size_mb: 10,
                max_open_files: 256,
                max_nesting_depth: 10,
            },
        }
    }

    /// 创建严格策略（只读访问）
    pub fn strict() -> Self {
        let mut policy = Self::restricted();
        policy.name = "strict".into();
        policy.description = "严格策略 - 只读访问，所有写操作需要审批".into();
        policy.level = SecurityLevel::Strict;
        policy.fs_rules = vec![
            FsRule {
                pattern: "${WORKDIR}/**".into(),
                action: FsAction::Ask,
                description: Some("工作目录内写操作需要审批".into()),
            },
            FsRule {
                pattern: "${WORKDIR}/**".into(),
                action: FsAction::Allow,
                description: Some("工作目录内读取".into()),
            },
            FsRule {
                pattern: "/*".into(),
                action: FsAction::Deny,
                description: Some("禁止访问工作目录外".into()),
            },
        ];
        policy
    }

    /// 创建隔离策略（完全隔离）
    pub fn isolated() -> Self {
        let mut policy = Self::restricted();
        policy.name = "isolated".into();
        policy.description = "隔离策略 - 完全隔离，只能访问特定资源".into();
        policy.level = SecurityLevel::Isolated;
        policy.exec_rules = vec![
            ExecRule {
                pattern: "ls".into(),
                action: ExecAction::Allow,
                description: Some("列出文件".into()),
            },
            ExecRule {
                pattern: "*".into(),
                action: ExecAction::Deny,
                description: Some("禁止所有其他命令".into()),
            },
        ];
        policy.max_exec_timeout_secs = 30;
        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restricted_policy() {
        let policy = SandboxPolicy::restricted();
        assert_eq!(policy.name, "restricted");
        assert_eq!(policy.level, SecurityLevel::Restricted);
        assert!(!policy.exec_rules.is_empty());
        assert!(!policy.fs_rules.is_empty());
    }

    #[test]
    fn test_strict_policy() {
        let policy = SandboxPolicy::strict();
        assert_eq!(policy.name, "strict");
        assert_eq!(policy.level, SecurityLevel::Strict);
    }

    #[test]
    fn test_isolated_policy() {
        let policy = SandboxPolicy::isolated();
        assert_eq!(policy.name, "isolated");
        assert_eq!(policy.level, SecurityLevel::Isolated);
        assert_eq!(policy.max_exec_timeout_secs, 30);
    }

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Unrestricted < SecurityLevel::Restricted);
        assert!(SecurityLevel::Restricted < SecurityLevel::Strict);
        assert!(SecurityLevel::Strict < SecurityLevel::Isolated);
    }
}
