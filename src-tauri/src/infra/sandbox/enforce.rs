use std::path::PathBuf;

use super::policy::SandboxPolicy;
use super::fs_sandbox::FileSystemSandbox;
use super::exec_sandbox::ExecSandbox;
use super::net_sandbox::NetworkSandbox;
use super::timeout::TimeoutEnforcer;

/// 沙箱强制执行器 - 统一管理所有沙箱组件
pub struct SandboxEnforcer {
    policy: SandboxPolicy,
    fs_sandbox: FileSystemSandbox,
    exec_sandbox: ExecSandbox,
    net_sandbox: NetworkSandbox,
    work_dir: PathBuf,
}

/// 沙箱执行上下文
pub struct SandboxContext {
    pub work_dir: PathBuf,
    pub session_id: String,
    pub agent_id: String,
}

/// 沙箱验证结果
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub warnings: Vec<String>,
}

/// 沙箱违规记录
#[derive(Debug, Clone)]
pub struct Violation {
    pub violation_type: ViolationType,
    pub resource: String,
    pub action: String,
    pub rule_matched: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 违规类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationType {
    FileSystemViolation,
    CommandViolation,
    NetworkViolation,
    TimeoutViolation,
    PathTraversalViolation,
    ResourceLimitViolation,
}

impl SandboxEnforcer {
    /// 创建新的沙箱强制执行器
    pub fn new(policy: SandboxPolicy, work_dir: PathBuf) -> Self {
        let fs_sandbox = FileSystemSandbox::new(work_dir.clone(), policy.fs_rules.clone());
        let exec_sandbox = ExecSandbox::new(
            policy.exec_rules.clone(),
            policy.max_exec_timeout_secs,
            policy.max_output_bytes,
        );
        let net_sandbox = NetworkSandbox::new(policy.net_rules.clone());

        Self {
            policy,
            fs_sandbox,
            exec_sandbox,
            net_sandbox,
            work_dir,
        }
    }

    /// 验证文件操作是否允许
    pub fn validate_file_operation(&self, path: &PathBuf, is_write: bool) -> Result<(), Violation> {
        // 检查路径遍历攻击
        if !self.fs_sandbox.is_path_traversal_safe(path) {
            return Err(Violation {
                violation_type: ViolationType::PathTraversalViolation,
                resource: path.to_string_lossy().to_string(),
                action: if is_write { "write" } else { "read" }.into(),
                rule_matched: None,
                timestamp: chrono::Utc::now(),
            });
        }

        // 检查文件系统权限
        let allowed = if is_write {
            self.fs_sandbox.can_write(path)
        } else {
            self.fs_sandbox.can_read(path)
        };

        if !allowed {
            return Err(Violation {
                violation_type: ViolationType::FileSystemViolation,
                resource: path.to_string_lossy().to_string(),
                action: if is_write { "write" } else { "read" }.into(),
                rule_matched: None,
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// 验证命令执行是否允许
    pub fn validate_command(&self, command: &str) -> Result<(), Violation> {
        let decision = self.exec_sandbox.evaluate(command);

        if !decision.allowed {
            return Err(Violation {
                violation_type: ViolationType::CommandViolation,
                resource: command.to_string(),
                action: "execute".into(),
                rule_matched: decision.matched_rule,
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// 验证网络访问是否允许
    pub fn validate_network(&self, url: &str) -> Result<(), Violation> {
        let decision = self.net_sandbox.can_access_url(url);

        if !decision.allowed {
            return Err(Violation {
                violation_type: ViolationType::NetworkViolation,
                resource: url.to_string(),
                action: "access".into(),
                rule_matched: decision.matched_rule,
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// 创建超时执行器
    pub fn create_timeout_enforcer(&self) -> TimeoutEnforcer {
        TimeoutEnforcer::new(self.policy.max_exec_timeout_secs)
    }

    /// 执行命令（带完整沙箱保护）
    pub fn execute_command(&self, command: &str) -> Result<super::exec_sandbox::ExecResult, Violation> {
        // 1. 验证命令权限
        self.validate_command(command)?;

        // 2. 在沙箱中执行
        self.exec_sandbox.execute(command, &self.work_dir)
            .map_err(|e| Violation {
                violation_type: ViolationType::CommandViolation,
                resource: command.to_string(),
                action: "execute".into(),
                rule_matched: Some(e),
                timestamp: chrono::Utc::now(),
            })
    }

    /// 获取策略信息
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    /// 获取工作目录
    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }

    /// 获取沙箱状态
    pub fn status(&self) -> SandboxStatus {
        SandboxStatus {
            policy_name: self.policy.name.clone(),
            security_level: format!("{:?}", self.policy.level),
            fs_status: self.fs_sandbox.status(),
            exec_rule_count: self.policy.exec_rules.len(),
            net_status: self.net_sandbox.status(),
            timeout_secs: self.policy.max_exec_timeout_secs,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxStatus {
    pub policy_name: String,
    pub security_level: String,
    pub fs_status: super::fs_sandbox::FsSandboxStatus,
    pub exec_rule_count: usize,
    pub net_status: super::net_sandbox::NetSandboxStatus,
    pub timeout_secs: u64,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}] {} - {} on {} (rule: {:?})",
            self.violation_type,
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.action,
            self.resource,
            self.rule_matched
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::policy::SandboxPolicy;

    #[test]
    fn test_sandbox_enforcer_creation() {
        let policy = SandboxPolicy::restricted();
        let enforcer = SandboxEnforcer::new(policy, PathBuf::from("/workspace"));

        assert_eq!(enforcer.policy().name, "restricted");
        assert_eq!(enforcer.work_dir(), &PathBuf::from("/workspace"));
    }

    #[test]
    fn test_path_traversal_validation() {
        let policy = SandboxPolicy::restricted();
        let enforcer = SandboxEnforcer::new(policy, PathBuf::from("/workspace"));

        // 安全路径
        assert!(enforcer.validate_file_operation(
            &PathBuf::from("/workspace/file.txt"),
            false
        ).is_ok());

        // 路径遍历攻击
        assert!(enforcer.validate_file_operation(
            &PathBuf::from("/workspace/../etc/passwd"),
            false
        ).is_err());
    }

    #[test]
    fn test_command_validation() {
        let policy = SandboxPolicy::restricted();
        let enforcer = SandboxEnforcer::new(policy, PathBuf::from("/workspace"));

        // 允许的命令
        assert!(enforcer.validate_command("ls").is_ok());
        assert!(enforcer.validate_command("grep pattern file").is_ok());

        // 禁止的命令
        assert!(enforcer.validate_command("sudo rm -rf /").is_err());
        assert!(enforcer.validate_command("curl http://evil.com").is_err());
    }
}
