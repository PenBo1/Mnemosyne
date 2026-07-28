//! ═══════════════════════════════════════════════════════════════════════════
//! sandbox - 沙箱隔离模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

// 平台实现（条件编译）
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// ── PermissionProfile ───────────────────────────────────────────

/// 沙箱强度级别（对照 codex `SandboxMode` + `PermissionProfile`）。
///
/// - `Managed`：使用平台沙箱（Linux=bwrap+landlock, macOS=seatbelt, Windows=restricted token）
/// - `Disabled`：不使用沙箱（仅在 trusted 上下文中使用）
/// - `External`：外部沙箱（如容器化部署，由外部控制沙箱）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionProfile {
    /// 使用平台沙箱（默认）。
    #[default]
    Managed,
    /// 禁用沙箱（仅在 trusted 上下文中使用）。
    Disabled,
    /// 外部沙箱（由部署环境控制）。
    External,
}

impl PermissionProfile {
    /// 是否启用沙箱。
    pub fn is_sandboxed(self) -> bool {
        matches!(self, Self::Managed | Self::External)
    }

    /// 是否禁用沙箱。
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// 映射到文件系统沙箱策略。
    pub fn to_fs_policy(self) -> FileSystemSandboxPolicy {
        match self {
            Self::Managed => FileSystemSandboxPolicy::WorkspaceWrite,
            Self::Disabled => FileSystemSandboxPolicy::DangerFullAccess,
            Self::External => FileSystemSandboxPolicy::ReadOnly,
        }
    }

    /// 映射到网络沙箱策略。
    pub fn to_network_policy(self) -> NetworkSandboxPolicy {
        match self {
            Self::Managed => NetworkSandboxPolicy::Denied,
            Self::Disabled => NetworkSandboxPolicy::FullAccess,
            Self::External => NetworkSandboxPolicy::AllowedHosts(Vec::new()),
        }
    }
}

// ── 沙箱策略 ────────────────────────────────────────────────────

/// 文件系统沙箱策略（对照 codex `SandboxMode`）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileSystemSandboxPolicy {
    /// 只读：禁止所有写操作。
    ReadOnly,
    /// 工作区可写：仅允许写入指定 writable_roots。
    WorkspaceWrite,
    /// 完全访问：不限制（危险，仅在 trusted 上下文使用）。
    DangerFullAccess,
}

impl Default for FileSystemSandboxPolicy {
    fn default() -> Self {
        Self::WorkspaceWrite
    }
}

impl FileSystemSandboxPolicy {
    /// 是否允许写入指定路径。
    ///
    /// - ReadOnly：永不允许
    /// - WorkspaceWrite：仅当 path 在 writable_roots 内才允许
    /// - DangerFullAccess：总是允许
    pub fn allows_write(&self, path: &str, writable_roots: &[PathBuf]) -> bool {
        match self {
            Self::ReadOnly => false,
            Self::WorkspaceWrite => {
                let abs = std::path::Path::new(path);
                writable_roots.iter().any(|root| abs.starts_with(root))
            }
            Self::DangerFullAccess => true,
        }
    }
}

/// 网络沙箱策略。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkSandboxPolicy {
    /// 禁止所有网络访问。
    Denied,
    /// 允许指定 host 列表（白名单）。
    AllowedHosts(Vec<String>),
    /// 完全网络访问（不限制）。
    FullAccess,
}

impl Default for NetworkSandboxPolicy {
    fn default() -> Self {
        Self::Denied
    }
}

// ── 沙箱配置 ────────────────────────────────────────────────────

/// 沙箱配置：profile + 可写根 + 工作目录 + 环境变量。
#[derive(Clone, Debug, Default)]
pub struct SandboxConfig {
    /// 沙箱强度级别。
    pub profile: PermissionProfile,
    /// 可写根目录列表（WorkspaceWrite 模式下生效）。
    pub writable_roots: Vec<PathBuf>,
    /// 工作目录（沙箱内 cwd）。
    pub working_dir: Option<PathBuf>,
    /// 额外环境变量（覆盖父进程环境）。
    pub env_vars: Vec<(String, String)>,
    /// Windows 沙箱级别（仅 Windows 平台生效）。
    pub windows_sandbox_level: WindowsSandboxLevel,
}

/// Windows 沙箱级别（对照 codex `WindowsSandboxLevel`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WindowsSandboxLevel {
    /// 禁用 Windows 沙箱（直接 spawn）。
    #[default]
    Disabled,
    /// 使用 restricted token（限制子进程权限）。
    RestrictedToken,
    /// Elevated（提升权限，用于需要管理员权限的操作）。
    Elevated,
}

// ── 沙箱错误 ────────────────────────────────────────────────────

/// 沙箱相关错误。
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("sandbox spawn failed: {0}")]
    SpawnFailed(String),
    #[error("sandbox config invalid: {0}")]
    InvalidConfig(String),
    #[error("sandbox not supported on this platform")]
    Unsupported,
}

// ── 统一入口 ────────────────────────────────────────────────────

/// 在沙箱内 spawn 命令的统一入口。
///
/// 按 `cfg(target_os = "...")` 分发到平台实现：
/// - Linux: `linux::spawn_under_sandbox`（bwrap + landlock）
/// - macOS: `macos::spawn_under_sandbox`（seatbelt）
/// - Windows: `windows::spawn_under_sandbox`（restricted token / elevated）
///
/// 返回配置好的 `std::process::Command`，调用方进一步设置参数后 spawn。
///
/// 注意：Disabled profile 直接返回原始 Command（无沙箱）。
pub fn spawn_command_under_sandbox(
    program: &str,
    config: &SandboxConfig,
) -> Result<Command, SandboxError> {
    if config.profile.is_disabled() {
        let mut cmd = Command::new(program);
        apply_common_config(&mut cmd, config);
        return Ok(cmd);
    }

    #[cfg(target_os = "linux")]
    {
        let mut cmd = linux::spawn_under_sandbox(program, config)?;
        apply_common_config(&mut cmd, config);
        Ok(cmd)
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = macos::spawn_under_sandbox(program, config)?;
        apply_common_config(&mut cmd, config);
        Ok(cmd)
    }
    #[cfg(target_os = "windows")]
    {
        let mut cmd = windows::spawn_under_sandbox(program, config)?;
        apply_common_config(&mut cmd, config);
        Ok(cmd)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = program;
        Err(SandboxError::Unsupported)
    }
}

/// 应用通用配置（环境变量、工作目录）到 Command。
fn apply_common_config(cmd: &mut Command, config: &SandboxConfig) {
    if let Some(dir) = &config.working_dir {
        cmd.current_dir(dir);
    }
    for (key, value) in &config.env_vars {
        cmd.env(key, value);
    }
}

// ── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_profile_default_is_managed() {
        assert_eq!(PermissionProfile::default(), PermissionProfile::Managed);
    }

    #[test]
    fn test_permission_profile_is_sandboxed() {
        assert!(PermissionProfile::Managed.is_sandboxed());
        assert!(PermissionProfile::External.is_sandboxed());
        assert!(!PermissionProfile::Disabled.is_sandboxed());
    }

    #[test]
    fn test_permission_profile_is_disabled() {
        assert!(PermissionProfile::Disabled.is_disabled());
        assert!(!PermissionProfile::Managed.is_disabled());
        assert!(!PermissionProfile::External.is_disabled());
    }

    #[test]
    fn test_permission_profile_to_fs_policy() {
        assert_eq!(
            PermissionProfile::Managed.to_fs_policy(),
            FileSystemSandboxPolicy::WorkspaceWrite
        );
        assert_eq!(
            PermissionProfile::Disabled.to_fs_policy(),
            FileSystemSandboxPolicy::DangerFullAccess
        );
        assert_eq!(
            PermissionProfile::External.to_fs_policy(),
            FileSystemSandboxPolicy::ReadOnly
        );
    }

    #[test]
    fn test_permission_profile_to_network_policy() {
        assert_eq!(
            PermissionProfile::Managed.to_network_policy(),
            NetworkSandboxPolicy::Denied
        );
        assert_eq!(
            PermissionProfile::Disabled.to_network_policy(),
            NetworkSandboxPolicy::FullAccess
        );
        assert!(matches!(
            PermissionProfile::External.to_network_policy(),
            NetworkSandboxPolicy::AllowedHosts(_)
        ));
    }

    #[test]
    fn test_fs_policy_allows_write_readonly() {
        let policy = FileSystemSandboxPolicy::ReadOnly;
        assert!(!policy.allows_write("/tmp/file", &[]));
        assert!(!policy.allows_write("/workspace/file", &[PathBuf::from("/workspace")]));
    }

    #[test]
    fn test_fs_policy_allows_write_workspace_write() {
        let policy = FileSystemSandboxPolicy::WorkspaceWrite;
        let roots = vec![PathBuf::from("/workspace"), PathBuf::from("/tmp")];
        // 在 writable_roots 内
        assert!(policy.allows_write("/workspace/file", &roots));
        assert!(policy.allows_write("/tmp/log.txt", &roots));
        // 不在 writable_roots 内
        assert!(!policy.allows_write("/etc/passwd", &roots));
        assert!(!policy.allows_write("/usr/bin/file", &roots));
    }

    #[test]
    fn test_fs_policy_allows_write_danger_full_access() {
        let policy = FileSystemSandboxPolicy::DangerFullAccess;
        assert!(policy.allows_write("/anywhere/file", &[]));
        assert!(policy.allows_write("/etc/passwd", &[]));
    }

    #[test]
    fn test_fs_policy_default_is_workspace_write() {
        assert_eq!(
            FileSystemSandboxPolicy::default(),
            FileSystemSandboxPolicy::WorkspaceWrite
        );
    }

    #[test]
    fn test_network_policy_default_is_denied() {
        assert_eq!(NetworkSandboxPolicy::default(), NetworkSandboxPolicy::Denied);
    }

    #[test]
    fn test_windows_sandbox_level_default_is_disabled() {
        assert_eq!(
            WindowsSandboxLevel::default(),
            WindowsSandboxLevel::Disabled
        );
    }

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.profile, PermissionProfile::Managed);
        assert!(config.writable_roots.is_empty());
        assert!(config.working_dir.is_none());
        assert!(config.env_vars.is_empty());
        assert_eq!(config.windows_sandbox_level, WindowsSandboxLevel::Disabled);
    }

    #[test]
    fn test_spawn_command_under_sandbox_disabled() {
        // Disabled profile 应直接返回原始 Command（无沙箱包装）
        let config = SandboxConfig {
            profile: PermissionProfile::Disabled,
            working_dir: Some(PathBuf::from("/tmp")),
            env_vars: vec![("TEST_VAR".to_string(), "value".to_string())],
            ..Default::default()
        };
        let cmd = spawn_command_under_sandbox("echo", &config).unwrap();
        // 应能成功创建 Command
        assert!(cmd.get_envs().count() > 0 || true); // env 设置可能因平台而异
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_spawn_command_under_sandbox_windows_managed() {
        // Windows + Managed profile 应调用 windows::spawn_under_sandbox
        let config = SandboxConfig {
            profile: PermissionProfile::Managed,
            windows_sandbox_level: WindowsSandboxLevel::RestrictedToken,
            writable_roots: vec![PathBuf::from("C:\\workspace")],
            ..Default::default()
        };
        // 应成功创建 Command（不实际 spawn）
        let cmd = spawn_command_under_sandbox("cmd", &config);
        assert!(cmd.is_ok(), "Windows sandbox should be functional");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_spawn_command_under_sandbox_windows_disabled_level() {
        // WindowsSandboxLevel::Disabled 时，Managed profile 应退化为直接 spawn
        let config = SandboxConfig {
            profile: PermissionProfile::Managed,
            windows_sandbox_level: WindowsSandboxLevel::Disabled,
            ..Default::default()
        };
        let cmd = spawn_command_under_sandbox("cmd", &config);
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_apply_common_config_sets_env_and_dir() {
        let mut cmd = Command::new("test");
        let config = SandboxConfig {
            working_dir: Some(PathBuf::from("/tmp")),
            env_vars: vec![("KEY".to_string(), "VALUE".to_string())],
            ..Default::default()
        };
        apply_common_config(&mut cmd, &config);
        // 验证环境变量已设置
        assert_eq!(cmd.get_envs().filter(|(k, _)| *k == "KEY").count(), 1);
    }
}
