//! ═══════════════════════════════════════════════════════════════════════════
//! sudo_guard - sudo 命令守卫模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;

use crate::shared::error::AppError;

/// `sudo -S` 的密码环境变量名。
const SUDO_PASSWORD_ENV: &str = "SUDO_PASSWORD";

/// 检测 sudo -S 用法的密码源完整性。
///
/// - 命令含 `sudo -S` 且 `SUDO_PASSWORD` 未设置 → `Err(forbidden)`
/// - 命令含 `sudo -S` 且 `SUDO_PASSWORD` 已设置 → `Ok(())`
/// - 命令不含 `sudo -S` → `Ok(())`
pub fn check_sudo_usage(
    command: &str,
    env: &HashMap<String, String>,
) -> Result<(), AppError> {
    if command.contains("sudo -S") && !env.contains_key(SUDO_PASSWORD_ENV) {
        return Err(AppError::forbidden(
            "sudo -S requires SUDO_PASSWORD env var",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with_password() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert(SUDO_PASSWORD_ENV.to_string(), "secret".to_string());
        env
    }

    fn env_empty() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn test_sudo_s_without_password_returns_err() {
        let env = env_empty();
        let result = check_sudo_usage("sudo -S apt update", &env);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "FORBIDDEN");
        assert!(err.message.contains("SUDO_PASSWORD"));
    }

    #[test]
    fn test_sudo_s_with_password_returns_ok() {
        let env = env_with_password();
        let result = check_sudo_usage("sudo -S apt update", &env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sudo_without_s_flag_returns_ok() {
        // 普通 sudo（非 -S）不归本守卫管，由 command_patterns 标记 RequireApproval
        let env = env_empty();
        let result = check_sudo_usage("sudo apt update", &env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_sudo_returns_ok() {
        let env = env_empty();
        let result = check_sudo_usage("ls -la", &env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sudo_s_in_middle_of_command() {
        let env = env_empty();
        let result = check_sudo_usage("echo pass | sudo -S apt update", &env);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_command_returns_ok() {
        let env = env_empty();
        let result = check_sudo_usage("", &env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sudo_s_case_sensitive() {
        // sudo -s（小写 s）是登录 shell 模式，不是 -S（stdin 密码）
        let env = env_empty();
        let result = check_sudo_usage("sudo -s", &env);
        assert!(result.is_ok());
    }
}
