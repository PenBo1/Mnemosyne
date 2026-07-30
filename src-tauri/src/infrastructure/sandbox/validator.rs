//! ═══════════════════════════════════════════════════════════════════════════
//! Sandbox 验证器 - 路径/命令/URL 验证业务逻辑
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 从 commands.rs 和 state.rs 中提取的验证逻辑，遵循分层架构原则。

use std::path::{Path, PathBuf};
use crate::shared::error::AppError;
use crate::infrastructure::validation::validate_path;
use super::constants::{MAX_PATH_LEN, MAX_COMMAND_LEN, MAX_URL_LEN};
use super::policy::SandboxPolicy;
use super::heuristics;
use super::execpolicy::{self, evaluator::Evaluation, ExecPolicy, NetworkProtocol};

// ── 路径验证 ────────────────────────────────────────────────────────────────

/// 验证沙箱路径参数
///
/// 检查：
/// - 非空
/// - 长度限制
/// - 路径遍历
/// - 规范化
pub fn validate_sandbox_path(path: &str) -> Result<PathBuf, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > MAX_PATH_LEN {
        return Err(AppError::invalid_input(format!(
            "Path too long (max {} chars)", MAX_PATH_LEN
        )));
    }
    validate_path(path).map_err(AppError::invalid_input)?;

    let path_buf = PathBuf::from(path);
    if path_buf.components().any(|c| c.as_os_str() == "..") {
        return Err(AppError::invalid_input("Path traversal denied"));
    }
    Ok(path_buf)
}

/// 沙箱路径验证器
pub struct PathValidator<'a> {
    root: &'a Path,
    policy: &'a SandboxPolicy,
}

impl<'a> PathValidator<'a> {
    pub fn new(root: &'a Path, policy: &'a SandboxPolicy) -> Self {
        Self { root, policy }
    }

    /// 验证路径是否允许访问
    ///
    /// 检查：
    /// - 受保护元数据路径
    /// - 路径规范化
    /// - 写权限
    pub fn validate(&self, path: &PathBuf, is_write: bool) -> Result<bool, AppError> {
        // 受保护元数据检查
        if is_write && self.policy.is_protected_metadata_path(path) {
            tracing::warn!(
                path = %path.display(),
                "Blocked write to protected metadata path"
            );
            return Ok(false);
        }

        // 路径规范化
        let canonical = self.canonicalize_path(path)?;

        // 检查是否在 root 下
        if !canonical.starts_with(self.root) {
            return Ok(false);
        }

        // 写权限检查
        if is_write && !self.policy.allow_write {
            return Ok(false);
        }

        Ok(true)
    }

    /// 规范化路径
    ///
    /// - 存在的路径：直接 canonicalize
    /// - 不存在的路径：canonicalize 父目录再拼接文件名
    fn canonicalize_path(&self, path: &PathBuf) -> Result<PathBuf, AppError> {
        if path.exists() {
            std::fs::canonicalize(path).map_err(|e| {
                AppError::internal(format!("Failed to canonicalize: {}", e))
            })
        } else {
            let parent = path.parent().ok_or_else(|| {
                AppError::invalid_input("path has no parent directory")
            })?;
            let canonical_parent = std::fs::canonicalize(parent).map_err(|e| {
                AppError::internal(format!("Failed to canonicalize parent: {}", e))
            })?;
            let file_name = path.file_name().ok_or_else(|| {
                AppError::invalid_input("path has no file name")
            })?;
            Ok(canonical_parent.join(file_name))
        }
    }
}

// ── 命令验证 ────────────────────────────────────────────────────────────────

/// 沙箱命令验证器
pub struct CommandValidator<'a> {
    policy: &'a SandboxPolicy,
    exec_policy: &'a ExecPolicy,
}

impl<'a> CommandValidator<'a> {
    pub fn new(policy: &'a SandboxPolicy, exec_policy: &'a ExecPolicy) -> Self {
        Self { policy, exec_policy }
    }

    /// 验证命令是否允许执行
    pub fn validate(&self, command: &str) -> Result<bool, AppError> {
        // 拒绝含控制字符的命令
        if command.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
            return Ok(false);
        }

        // ExecPolicy 精细化评估（优先）
        let eval = self.exec_policy.evaluate_command(command);
        if eval.matched_index.is_some() {
            return Ok(eval.decision.is_allowed_sync());
        }

        // 黑名单 token 前缀检查
        if self.is_blocked_command(command) {
            return Ok(false);
        }

        // 启发式危险模式检查
        if let Some(reason) = heuristics::command_might_be_dangerous(command) {
            tracing::warn!(
                command = %command,
                reason = %reason,
                "Blocked command by heuristic"
            );
            return Ok(false);
        }

        // 已知安全命令
        if heuristics::is_known_safe_command(command) {
            return Ok(true);
        }

        // 默认行为
        Ok(self.policy.allow_exec)
    }

    /// 检查命令是否在黑名单中
    fn is_blocked_command(&self, command: &str) -> bool {
        let cmd_tokens: Vec<&str> = command.split_whitespace().collect();
        for blocked in &self.policy.blocked_commands {
            let blocked_tokens: Vec<&str> = blocked.split_whitespace().collect();
            if blocked_tokens.is_empty() {
                continue;
            }
            if cmd_tokens.len() >= blocked_tokens.len()
                && blocked_tokens
                    .iter()
                    .zip(cmd_tokens.iter())
                    .all(|(b, c)| b.eq_ignore_ascii_case(c))
            {
                return true;
            }
        }
        false
    }

    /// 评估命令（返回完整 PolicyDecision）
    pub fn evaluate(&self, command: &str) -> Evaluation {
        self.exec_policy.evaluate_command(command)
    }
}

// ── URL 验证 ─────────────────────────────────────────────────────────────────

/// 沙箱 URL 验证器
pub struct UrlValidator<'a> {
    policy: &'a SandboxPolicy,
    exec_policy: &'a ExecPolicy,
}

impl<'a> UrlValidator<'a> {
    pub fn new(policy: &'a SandboxPolicy, exec_policy: &'a ExecPolicy) -> Self {
        Self { policy, exec_policy }
    }

    /// 验证 URL 是否允许访问
    pub fn validate(&self, url: &str) -> Result<bool, AppError> {
        // ExecPolicy 网络规则评估
        let (host, protocol) = execpolicy::extract_host_and_protocol(url);
        if !host.is_empty() {
            let eval = self.exec_policy.evaluate_network(&host, protocol);
            if eval.matched_index.is_some() {
                return Ok(eval.decision.is_allowed_sync());
            }
        }

        // 域名黑名单检查
        for blocked in &self.policy.blocked_domains {
            if url.contains(blocked) {
                return Ok(false);
            }
        }

        Ok(self.policy.allow_network)
    }

    /// 评估网络请求（返回完整 PolicyDecision）
    pub fn evaluate(&self, host: &str, protocol: NetworkProtocol) -> Evaluation {
        self.exec_policy.evaluate_network(host, protocol)
    }
}

// ── 辅助验证函数 ────────────────────────────────────────────────────────────

/// 验证命令参数
pub fn validate_command_param(command: &str) -> Result<(), AppError> {
    if command.trim().is_empty() {
        return Err(AppError::invalid_input("Command cannot be empty"));
    }
    if command.len() > MAX_COMMAND_LEN {
        return Err(AppError::invalid_input(format!(
            "Command too long (max {} chars)", MAX_COMMAND_LEN
        )));
    }
    Ok(())
}

/// 验证路径参数
pub fn validate_path_param(path: &str) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > MAX_PATH_LEN {
        return Err(AppError::invalid_input(format!(
            "Path too long (max {} chars)", MAX_PATH_LEN
        )));
    }
    Ok(())
}

/// 验证 URL 参数
pub fn validate_url_param(url: &str) -> Result<(), AppError> {
    if url.trim().is_empty() {
        return Err(AppError::invalid_input("URL cannot be empty"));
    }
    if url.len() > MAX_URL_LEN {
        return Err(AppError::invalid_input(format!(
            "URL too long (max {} chars)", MAX_URL_LEN
        )));
    }
    Ok(())
}

/// 验证主机参数
pub fn validate_host_param(host: &str) -> Result<(), AppError> {
    if host.trim().is_empty() {
        return Err(AppError::invalid_input("Host cannot be empty"));
    }
    if host.len() > MAX_URL_LEN {
        return Err(AppError::invalid_input(format!(
            "Host too long (max {} chars)", MAX_URL_LEN
        )));
    }
    Ok(())
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_command_param() {
        assert!(validate_command_param("ls -la").is_ok());
        assert!(validate_command_param("").is_err());
        assert!(validate_command_param("   ").is_err());
    }

    #[test]
    fn test_validate_path_param() {
        assert!(validate_path_param("/tmp/test").is_ok());
        assert!(validate_path_param("").is_err());
    }

    #[test]
    fn test_validate_url_param() {
        assert!(validate_url_param("https://example.com").is_ok());
        assert!(validate_url_param("").is_err());
    }
}