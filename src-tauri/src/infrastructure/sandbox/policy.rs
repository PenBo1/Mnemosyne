//! ═══════════════════════════════════════════════════════════════════════════
//! 沙箱策略 - 安全策略定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_exec: bool,
    pub allow_network: bool,
    /// 命令前缀黑名单(匹配命令开头即拒绝)。
    /// 扩展自通用安全实践,覆盖:
    /// - 系统破坏:rm/del/format/fdisk/mkfs/shutdown/reboot
    /// - 权限提升:sudo/doas/su
    /// - 脚本解释器:python/python3/bash/sh/zsh/fish/node
    /// - 包管理器:npm/yarn/pnpm/bun
    /// - 网络下载:curl/wget
    /// - 文件属性:chmod/chown
    /// - 块设备:dd
    pub blocked_commands: Vec<String>,
    pub blocked_domains: Vec<String>,
    /// 受保护的元数据文件名(写入需显式批准)。
    /// .git / state.sqlite / config.json 等关键文件,
    /// 即使在 writable root 下也拒绝写入。
    pub protected_metadata_names: Vec<String>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allow_read: true,
            allow_write: true,
            allow_exec: false,
            allow_network: true,
            blocked_commands: default_blocked_commands(),
            blocked_domains: default_blocked_domains(),
            protected_metadata_names: default_protected_metadata_names(),
        }
    }
}

/// 启发式的高风险命令前缀清单。
fn default_blocked_commands() -> Vec<String> {
    vec![
        // 系统破坏
        "rm".into(), "del".into(), "rmdir".into(),
        "format".into(), "fdisk".into(), "mkfs".into(),
        "shutdown".into(), "reboot".into(), "halt".into(),
        "poweroff".into(), "init".into(), "systemctl".into(),
        // 权限提升
        "sudo".into(), "doas".into(), "su".into(),
        // 脚本解释器(可通过 -c 执行任意代码)
        "bash".into(), "sh".into(), "zsh".into(), "fish".into(),
        "python".into(), "python3".into(),
        "node".into(),
        // 包管理器(可执行 install scripts)
        "npm".into(), "yarn".into(), "pnpm".into(), "bun".into(),
        // 网络下载(可能下载并执行恶意代码)
        "curl".into(), "wget".into(),
        // 文件属性变更
        "chmod".into(), "chown".into(), "chattr".into(),
        // 块设备
        "dd".into(),
    ]
}

/// 默认拒绝访问的云元数据端点(AWS/GCP/Azure)。
fn default_blocked_domains() -> Vec<String> {
    vec![
        "metadata.google.internal".into(),
        "metadata.azure.com".into(),
        "169.254.169.254".into(),
    ]
}

/// 受保护的元数据文件/目录名清单。
/// 即使在 writable root 下,这些路径也拒绝写入,防止:
/// - 破坏版本控制(.git)
/// - 破坏应用状态(state.sqlite / feedback.sqlite / logs.sqlite)
/// - 破坏应用配置(config.json)
/// - 破坏 agent 身份(agents/)
fn default_protected_metadata_names() -> Vec<String> {
    vec![
        ".git".into(),
        "state.sqlite".into(),
        "state.sqlite-wal".into(),
        "state.sqlite-shm".into(),
        "feedback.sqlite".into(),
        "feedback.sqlite-wal".into(),
        "feedback.sqlite-shm".into(),
        "logs.sqlite".into(),
        "logs.sqlite-wal".into(),
        "logs.sqlite-shm".into(),
        "config.json".into(),
    ]
}

impl SandboxPolicy {
    /// 检查路径是否触碰受保护的元数据。
    ///
    /// 匹配规则:路径任一组件等于 protected_metadata_names 中的项即视为受保护。
    /// 例如:
    /// - `/workspace/.git/HEAD` → 命中 `.git`
    /// - `/appdata/data/state.sqlite` → 命中 `state.sqlite`
    /// - `/appdata/agents/main/SOUL.md` → 未命中(不在清单)
    pub fn is_protected_metadata_path(&self, path: &std::path::Path) -> bool {
        for component in path.components() {
            if let std::path::Component::Normal(os_str) = component {
                if let Some(name) = os_str.to_str() {
                    if self.protected_metadata_names.iter().any(|p| p == name) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_policy_has_expanded_blocklist() {
        let p = SandboxPolicy::default();
        assert!(p.blocked_commands.contains(&"python".to_string()));
        assert!(p.blocked_commands.contains(&"sudo".to_string()));
        assert!(p.blocked_commands.contains(&"curl".to_string()));
        assert!(p.blocked_commands.contains(&"rm".to_string()));
    }

    #[test]
    fn protected_metadata_detects_git_dir() {
        let p = SandboxPolicy::default();
        assert!(p.is_protected_metadata_path(Path::new("/workspace/.git/HEAD")));
        assert!(p.is_protected_metadata_path(Path::new("/workspace/.git")));
    }

    #[test]
    fn protected_metadata_detects_state_sqlite() {
        let p = SandboxPolicy::default();
        assert!(p.is_protected_metadata_path(Path::new("/appdata/data/state.sqlite")));
        assert!(p.is_protected_metadata_path(Path::new("/appdata/data/state.sqlite-wal")));
    }

    #[test]
    fn protected_metadata_detects_config_json() {
        let p = SandboxPolicy::default();
        assert!(p.is_protected_metadata_path(Path::new("/appdata/config.json")));
    }

    #[test]
    fn protected_metadata_allows_normal_files() {
        let p = SandboxPolicy::default();
        assert!(!p.is_protected_metadata_path(Path::new("/workspace/chapter1.md")));
        assert!(!p.is_protected_metadata_path(Path::new("/appdata/agents/main/SOUL.md")));
        assert!(!p.is_protected_metadata_path(Path::new("/tmp/test.txt")));
    }
}
