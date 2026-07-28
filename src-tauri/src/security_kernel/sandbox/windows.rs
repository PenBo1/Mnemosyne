//! ═══════════════════════════════════════════════════════════════════════════
//! windows - Windows 平台沙箱实现模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::process::Command;

use super::{SandboxConfig, SandboxError, WindowsSandboxLevel};

/// 敏感环境变量名模式（匹配则剥离）。
///
/// 对照 codex `ShellEnvironmentPolicy` 默认排除模式：
/// `*KEY*` / `*SECRET*` / `*TOKEN*`。
const SENSITIVE_ENV_PATTERNS: &[&str] = &["KEY", "SECRET", "TOKEN", "PASSWORD", "CREDENTIAL"];

/// 在 Windows 沙箱内 spawn 命令。
///
/// 返回配置好的 `Command`，调用方进一步设置参数后 spawn。
pub fn spawn_under_sandbox(program: &str, config: &SandboxConfig) -> Result<Command, SandboxError> {
    match config.windows_sandbox_level {
        WindowsSandboxLevel::Disabled => {
            // Disabled 级别：直接 spawn（profile 已是 Managed，但 level 为 Disabled 时退化为直接 spawn）
            tracing::warn!(
                program,
                "Windows sandbox level is Disabled under Managed profile; spawning without sandbox"
            );
            Ok(Command::new(program))
        }
        WindowsSandboxLevel::RestrictedToken => {
            spawn_with_restricted_token(program, config)
        }
        WindowsSandboxLevel::Elevated => {
            spawn_elevated(program, config)
        }
    }
}

/// RestrictedToken 模式：剥离敏感环境变量 + 约束 cwd。
///
/// 这是"软沙箱"：不调用 `CreateRestrictedToken`，而是通过环境变量脱敏
/// 和 cwd 约束实现功能性限制。真正的 OS 级别 restricted token 需要
/// `windows-sys` 依赖（未来增强）。
fn spawn_with_restricted_token(program: &str, config: &SandboxConfig) -> Result<Command, SandboxError> {
    let mut cmd = Command::new(program);

    // 约束 cwd：若设置了 working_dir，验证是否在 writable_roots 内
    if let Some(dir) = &config.working_dir {
        if !config.writable_roots.is_empty() {
            let dir_normalized = normalize_windows_path(dir);
            let in_writable = config.writable_roots.iter().any(|root| {
                let root_normalized = normalize_windows_path(root);
                dir_normalized.starts_with(&root_normalized)
            });
            if !in_writable {
                return Err(SandboxError::InvalidConfig(format!(
                    "working_dir {:?} is not within writable_roots",
                    dir
                )));
            }
        }
        cmd.current_dir(dir);
    }

    // 剥离敏感环境变量：遍历父进程环境，过滤匹配 SENSITIVE_ENV_PATTERNS 的变量
    // cmd.env_clear() 后重新设置安全的环境变量
    let safe_env = collect_safe_env();
    cmd.env_clear();
    for (key, value) in safe_env {
        if !is_sensitive_env_key(&key) {
            cmd.env(key, value);
        }
    }

    // 覆盖额外环境变量
    for (key, value) in &config.env_vars {
        cmd.env(key, value);
    }

    tracing::debug!(
        program,
        level = "RestrictedToken",
        "Windows sandbox: restricted token mode (env sanitized, cwd constrained)"
    );

    Ok(cmd)
}

/// Elevated 模式：通过 PowerShell 请求 UAC 提升。
///
/// 使用 `powershell -Command "Start-Process -Verb RunAs -ArgumentList ..."`。
/// 注意：这会触发 UAC 弹窗，用户需手动确认。
fn spawn_elevated(program: &str, config: &SandboxConfig) -> Result<Command, SandboxError> {
    // 构造 PowerShell 命令提升权限
    // Start-Process -FilePath <program> -Verb RunAs [-ArgumentList ...] [-WorkingDirectory ...]
    let mut ps_args = vec![format!("-FilePath '{}'", program)];

    if let Some(dir) = &config.working_dir {
        ps_args.push(format!("-WorkingDirectory '{}'", dir.display()));
    }

    // 额外环境变量在提升模式下无法直接传递（UAC 会创建新进程），记录警告
    if !config.env_vars.is_empty() {
        tracing::warn!(
            "env_vars cannot be passed through UAC elevation; they will be ignored in Elevated mode"
        );
    }

    let ps_command = format!("Start-Process {}", ps_args.join(" "));

    let mut cmd = Command::new("powershell");
    cmd.arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(&ps_command);

    tracing::info!(
        program,
        level = "Elevated",
        "Windows sandbox: elevated mode (UAC prompt will appear)"
    );

    Ok(cmd)
}

/// 判断环境变量名是否敏感（匹配 KEY/SECRET/TOKEN/PASSWORD/CREDENTIAL）。
fn is_sensitive_env_key(key: &str) -> bool {
    let key_upper = key.to_ascii_uppercase();
    SENSITIVE_ENV_PATTERNS.iter().any(|pattern| key_upper.contains(pattern))
}

/// 收集当前进程的安全环境变量（PATH / SYSTEMROOT / TEMP 等）。
///
/// 在 RestrictedToken 模式下，`env_clear()` 后需要重新设置必要的系统变量，
/// 否则子进程可能无法找到可执行文件。
fn collect_safe_env() -> Vec<(String, String)> {
    let essential_keys = [
        "PATH",
        "Path",
        "SYSTEMROOT",
        "SystemRoot",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMDATA",
        "COMSPEC",
        "PATHEXT",
        "HOMEDRIVE",
        "HOMEPATH",
    ];

    let mut safe_env = Vec::new();
    for key in &essential_keys {
        if let Ok(value) = std::env::var(key) {
            safe_env.push((key.to_string(), value));
        }
    }
    safe_env
}

/// 规范化 Windows 路径（转为大写 + 统一分隔符）用于比较。
fn normalize_windows_path(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy().to_string();
    let normalized = s.replace('/', "\\").to_ascii_lowercase();
    std::path::PathBuf::from(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_sensitive_env_key() {
        assert!(is_sensitive_env_key("API_KEY"));
        assert!(is_sensitive_env_key("SECRET_TOKEN"));
        assert!(is_sensitive_env_key("PASSWORD"));
        assert!(is_sensitive_env_key("CREDENTIAL"));
        assert!(is_sensitive_env_key("MY_TOKEN"));
        assert!(!is_sensitive_env_key("PATH"));
        assert!(!is_sensitive_env_key("HOME"));
        assert!(!is_sensitive_env_key("USERNAME"));
    }

    #[test]
    fn test_normalize_windows_path() {
        let p = normalize_windows_path(&PathBuf::from("C:/Users/Test/Documents"));
        assert_eq!(p, PathBuf::from("c:\\users\\test\\documents"));

        let p2 = normalize_windows_path(&PathBuf::from("C:\\Workspace"));
        assert_eq!(p2, PathBuf::from("c:\\workspace"));
    }

    #[test]
    fn test_spawn_disabled_returns_plain_command() {
        let config = SandboxConfig {
            windows_sandbox_level: WindowsSandboxLevel::Disabled,
            ..Default::default()
        };
        let cmd = spawn_under_sandbox("cmd", &config).unwrap();
        // Disabled 应返回原始 Command
        assert_eq!(cmd.get_program(), std::path::Path::new("cmd"));
    }

    #[test]
    fn test_spawn_restricted_token_sanitizes_env() {
        // 设置一个敏感环境变量用于测试
        std::env::set_var("TEST_API_KEY", "secret_value");
        std::env::set_var("TEST_NORMAL_VAR", "normal_value");

        let config = SandboxConfig {
            windows_sandbox_level: WindowsSandboxLevel::RestrictedToken,
            ..Default::default()
        };
        let cmd = spawn_under_sandbox("cmd", &config).unwrap();

        // 验证敏感变量被剥离，普通变量保留（仅检查 PATH 等系统变量存在）
        let envs: Vec<_> = cmd.get_envs().collect();
        // env_clear 后重新设置的环境不应包含 TEST_API_KEY
        assert!(!envs
            .iter()
            .any(|(k, _)| *k == std::ffi::OsStr::new("TEST_API_KEY")));

        std::env::remove_var("TEST_API_KEY");
        std::env::remove_var("TEST_NORMAL_VAR");
    }

    #[test]
    fn test_spawn_restricted_token_rejects_cwd_outside_writable_roots() {
        let config = SandboxConfig {
            windows_sandbox_level: WindowsSandboxLevel::RestrictedToken,
            working_dir: Some(PathBuf::from("C:\\Windows\\System32")),
            writable_roots: vec![PathBuf::from("C:\\Workspace")],
            ..Default::default()
        };
        let result = spawn_under_sandbox("cmd", &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SandboxError::InvalidConfig(_)));
    }

    #[test]
    fn test_spawn_restricted_token_accepts_cwd_in_writable_roots() {
        let config = SandboxConfig {
            windows_sandbox_level: WindowsSandboxLevel::RestrictedToken,
            working_dir: Some(PathBuf::from("C:\\Workspace\\subdir")),
            writable_roots: vec![PathBuf::from("C:\\Workspace")],
            ..Default::default()
        };
        let result = spawn_under_sandbox("cmd", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_spawn_elevated_uses_powershell() {
        let config = SandboxConfig {
            windows_sandbox_level: WindowsSandboxLevel::Elevated,
            ..Default::default()
        };
        let cmd = spawn_under_sandbox("regedit", &config).unwrap();
        // Elevated 应通过 powershell 启动
        assert_eq!(cmd.get_program(), std::path::Path::new("powershell"));
    }
}
