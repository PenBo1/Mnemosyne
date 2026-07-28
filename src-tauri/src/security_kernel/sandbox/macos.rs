//! ═══════════════════════════════════════════════════════════════════════════
//! macos - macOS 平台沙箱实现模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::process::Command;

use super::{SandboxConfig, SandboxError};

/// 在 macOS 沙箱内 spawn 命令（sandbox-exec + seatbelt profile）。
///
/// 构造 seatbelt profile：
/// - `deny network`：禁用网络（Managed profile）
/// - `(allow file-write* subpath "<root>")`：允许写入 writable_roots
/// - `(deny file-write*)`：禁止其他写入
pub fn spawn_under_sandbox(program: &str, config: &SandboxConfig) -> Result<Command, SandboxError> {
    // 检查 sandbox-exec 是否可用
    if !is_sandbox_exec_available() {
        return Err(SandboxError::SpawnFailed(
            "/usr/bin/sandbox-exec not found; this should not happen on macOS".to_string(),
        ));
    }

    // 构造 seatbelt profile
    let profile = build_seatbelt_profile(config);

    let mut cmd = Command::new("/usr/bin/sandbox-exec");
    cmd.arg("-p").arg(&profile);

    // 工作目录
    if let Some(dir) = &config.working_dir {
        cmd.arg("-C").arg(dir);
    }

    // 实际要执行的命令
    cmd.arg("--").arg(program);

    // 环境变量（sandbox-exec 继承父进程环境，额外设置）
    for (key, value) in &config.env_vars {
        cmd.env(key, value);
    }

    tracing::debug!(
        program,
        roots = config.writable_roots.len(),
        "macOS sandbox: seatbelt profile configured"
    );

    Ok(cmd)
}

/// 构造 seatbelt profile 字符串。
///
/// 策略：
/// - 默认 deny 所有 file-write 和 network
/// - 允许写入 writable_roots
/// - 允许读取所有路径（只读访问不限制）
fn build_seatbelt_profile(config: &SandboxConfig) -> String {
    let mut rules: Vec<String> = Vec::new();

    // 版本声明
    rules.push("(version 1)".to_string());
    rules.push("(allow default)".to_string());

    // 网络隔离：Managed profile 禁用网络
    if config.profile.is_sandboxed() {
        rules.push("(deny network)".to_string());
    }

    // 文件写入：默认禁止
    rules.push("(deny file-write*)".to_string());

    // 允许写入 writable_roots
    for root in &config.writable_roots {
        let root_str = root.to_string_lossy().replace('"', "\\\"");
        rules.push(format!(
            "(allow file-write* (subpath \"{}\"))",
            root_str
        ));
    }

    // 允许写入临时目录（进程必需）
    rules.push("(allow file-write* (subpath \"/tmp\"))".to_string());
    rules.push("(allow file-write* (subpath \"/var/tmp\"))".to_string());

    rules.join("\n")
}

/// 检查 sandbox-exec 是否可用。
fn is_sandbox_exec_available() -> bool {
    std::path::Path::new("/usr/bin/sandbox-exec").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_seatbelt_profile_includes_writable_roots() {
        let config = SandboxConfig {
            writable_roots: vec![PathBuf::from("/Users/test/workspace")],
            profile: super::super::PermissionProfile::Managed,
            ..Default::default()
        };
        let profile = build_seatbelt_profile(&config);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny network)"));
        assert!(profile.contains("(deny file-write*)"));
        assert!(profile.contains("/Users/test/workspace"));
        assert!(profile.contains("(allow file-write* (subpath \"/tmp\"))"));
    }

    #[test]
    fn test_build_seatbelt_profile_disabled_no_network_deny() {
        let config = SandboxConfig {
            profile: super::super::PermissionProfile::Disabled,
            ..Default::default()
        };
        let profile = build_seatbelt_profile(&config);
        // Disabled profile 不应添加 network deny
        assert!(!profile.contains("(deny network)"));
    }
}
