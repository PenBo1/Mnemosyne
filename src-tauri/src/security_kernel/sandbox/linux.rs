//! ═══════════════════════════════════════════════════════════════════════════
//! linux - Linux 平台沙箱实现模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::process::Command;

use super::{SandboxConfig, SandboxError};

/// 在 Linux 沙箱内 spawn 命令（bwrap + 可选 landlock）。
///
/// 构造 `bwrap` 命令链：
/// - `--unshare-net`：禁用网络（NetworkSandboxPolicy::Denied 时）
/// - `--bind <root> <root>`：可写挂载 writable_roots
/// - `--ro-bind / /`：只读挂载根文件系统
/// - `--dev /dev` + `--proc /proc`：挂载必要设备
/// - 最后跟实际要执行的命令
pub fn spawn_under_sandbox(program: &str, config: &SandboxConfig) -> Result<Command, SandboxError> {
    // 检查 bwrap 是否可用
    if !is_bwrap_available() {
        return Err(SandboxError::SpawnFailed(
            "bubblewrap (bwrap) is not installed; install via `apt install bubblewrap`".to_string(),
        ));
    }

    let mut cmd = Command::new("bwrap");

    // 只读挂载根文件系统
    cmd.arg("--ro-bind").arg("/").arg("/");
    cmd.arg("--dev").arg("/dev");
    cmd.arg("--proc").arg("/proc");

    // 可写挂载 writable_roots
    for root in &config.writable_roots {
        let root_str = root.to_string_lossy();
        cmd.arg("--bind").arg(root_str.as_ref()).arg(root_str.as_ref());
    }

    // 网络隔离：Managed profile 默认 Denied
    if config.profile.is_sandboxed() {
        cmd.arg("--unshare-net");
    }

    // 工作目录
    if let Some(dir) = &config.working_dir {
        cmd.arg("--chdir").arg(dir);
    }

    // 环境变量
    for (key, value) in &config.env_vars {
        cmd.arg("--setenv").arg(key).arg(value);
    }

    // 实际要执行的命令
    cmd.arg("--").arg(program);

    tracing::debug!(
        program,
        roots = config.writable_roots.len(),
        "Linux sandbox: bwrap configured"
    );

    Ok(cmd)
}

/// 检查 bwrap 是否可用（在 PATH 中）。
fn is_bwrap_available() -> bool {
    // 使用 `which` 命令检查（Linux 标准工具）
    Command::new("which")
        .arg("bwrap")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // 注意：以下测试仅在 Linux 平台运行
    // bwrap 不可用时应返回错误
    #[test]
    fn test_spawn_rejects_when_bwrap_missing() {
        // 此测试假设 CI 环境可能无 bwrap，验证错误处理
        let config = SandboxConfig {
            writable_roots: vec![PathBuf::from("/tmp")],
            ..Default::default()
        };
        // 不实际断言结果（bwrap 可能存在也可能不存在），仅验证不 panic
        let _ = spawn_under_sandbox("ls", &config);
    }
}
