use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::security_kernel::permission::{FsOperation, FsScope, Operation};
use crate::security_kernel::types::TrustLevel;

use super::decision::{OperationRisk, PolicyDecision};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPolicy {
    pub default_decision_by_risk: DefaultRiskDecisions,
    pub trusted_workspace_policy: WorkspacePolicy,
    pub enterprise_workspace_policy: WorkspacePolicy,
    pub readonly_workspace_policy: WorkspacePolicy,
    pub unknown_workspace_policy: WorkspacePolicy,
    pub dangerous_workspace_policy: WorkspacePolicy,
    pub blocked_operations: HashSet<String>,
    pub always_require_approval: HashSet<String>,
    pub always_deny: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultRiskDecisions {
    pub low: PolicyDecision,
    pub medium: PolicyDecision,
    pub high: PolicyDecision,
    pub critical: PolicyDecision,
}

impl Default for DefaultRiskDecisions {
    fn default() -> Self {
        Self {
            low: PolicyDecision::Allow,
            medium: PolicyDecision::RequireApproval,
            high: PolicyDecision::RequireApproval,
            critical: PolicyDecision::Deny,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePolicy {
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_delete: bool,
    pub allow_shell: bool,
    pub allow_network: bool,
    pub max_file_size_mb: u32,
    pub require_approval_for_write: bool,
    pub require_approval_for_delete: bool,
}

impl Default for WorkspacePolicy {
    fn default() -> Self {
        Self {
            allow_read: true,
            allow_write: true,
            allow_delete: true,
            allow_shell: true,
            allow_network: true,
            max_file_size_mb: 100,
            require_approval_for_write: false,
            require_approval_for_delete: true,
        }
    }
}

impl WorkspacePolicy {
    pub fn for_trusted() -> Self {
        Self {
            allow_read: true,
            allow_write: true,
            allow_delete: true,
            allow_shell: true,
            allow_network: true,
            max_file_size_mb: 500,
            require_approval_for_write: false,
            require_approval_for_delete: false,
        }
    }

    pub fn for_enterprise() -> Self {
        Self {
            allow_read: true,
            allow_write: true,
            allow_delete: false,
            allow_shell: true,
            allow_network: true,
            max_file_size_mb: 1000,
            require_approval_for_write: true,
            require_approval_for_delete: true,
        }
    }

    pub fn for_readonly() -> Self {
        Self {
            allow_read: true,
            allow_write: false,
            allow_delete: false,
            allow_shell: false,
            allow_network: true,
            max_file_size_mb: 50,
            require_approval_for_write: true,
            require_approval_for_delete: true,
        }
    }

    pub fn for_unknown() -> Self {
        Self {
            allow_read: true,
            allow_write: false,
            allow_delete: false,
            allow_shell: false,
            allow_network: false,
            max_file_size_mb: 10,
            require_approval_for_write: true,
            require_approval_for_delete: true,
        }
    }

    pub fn for_dangerous() -> Self {
        Self {
            allow_read: false,
            allow_write: false,
            allow_delete: false,
            allow_shell: false,
            allow_network: false,
            max_file_size_mb: 0,
            require_approval_for_write: true,
            require_approval_for_delete: true,
        }
    }
}

impl Default for GlobalPolicy {
    fn default() -> Self {
        let mut blocked = HashSet::new();
        // 危险 shell 命令(command-level,匹配任何 scope)
        blocked.insert("shell:rm".to_string());
        blocked.insert("shell:del".to_string());
        blocked.insert("shell:format".to_string());
        blocked.insert("shell:fdisk".to_string());
        blocked.insert("shell:shutdown".to_string());
        blocked.insert("shell:reboot".to_string());

        let mut always_approve = HashSet::new();
        always_approve.insert("fs:write:workspace".to_string());
        // git commit/push 在任何 scope 下都需审批(command-level key)
        always_approve.insert("shell:commit".to_string());
        always_approve.insert("shell:push".to_string());

        let mut always_deny = HashSet::new();
        always_deny.insert("fs:delete:cache".to_string());
        always_deny.insert("shell:mkfs".to_string());

        Self {
            default_decision_by_risk: DefaultRiskDecisions::default(),
            trusted_workspace_policy: WorkspacePolicy::for_trusted(),
            enterprise_workspace_policy: WorkspacePolicy::for_enterprise(),
            readonly_workspace_policy: WorkspacePolicy::for_readonly(),
            unknown_workspace_policy: WorkspacePolicy::for_unknown(),
            dangerous_workspace_policy: WorkspacePolicy::for_dangerous(),
            blocked_operations: blocked,
            always_require_approval: always_approve,
            always_deny,
        }
    }
}

impl GlobalPolicy {
    pub fn get_workspace_policy(&self, trust_level: TrustLevel) -> &WorkspacePolicy {
        match trust_level {
            TrustLevel::Trusted => &self.trusted_workspace_policy,
            TrustLevel::Enterprise => &self.enterprise_workspace_policy,
            TrustLevel::Readonly => &self.readonly_workspace_policy,
            TrustLevel::Unknown => &self.unknown_workspace_policy,
            TrustLevel::Dangerous => &self.dangerous_workspace_policy,
        }
    }

    pub fn decide_by_risk(&self, risk: OperationRisk) -> PolicyDecision {
        match risk {
            OperationRisk::Low => self.default_decision_by_risk.low,
            OperationRisk::Medium => self.default_decision_by_risk.medium,
            OperationRisk::High => self.default_decision_by_risk.high,
            OperationRisk::Critical => self.default_decision_by_risk.critical,
        }
    }

    pub fn is_blocked(&self, op: &Operation) -> bool {
        let op_key = operation_key(op);
        if self.blocked_operations.contains(&op_key) || self.always_deny.contains(&op_key) {
            return true;
        }
        // 对 Shell 操作额外检查 command-level key(不含 scope,匹配任何 scope 下的危险命令)
        if let crate::security_kernel::permission::Operation::Shell { command, .. } = op {
            let cmd_key = format!("shell:{}", command);
            if self.blocked_operations.contains(&cmd_key) || self.always_deny.contains(&cmd_key) {
                return true;
            }
        }
        false
    }

    pub fn requires_approval(&self, op: &Operation) -> bool {
        let op_key = operation_key(op);
        if self.always_require_approval.contains(&op_key) {
            return true;
        }
        // 对 Shell 操作额外检查 command-level key
        if let crate::security_kernel::permission::Operation::Shell { command, .. } = op {
            let cmd_key = format!("shell:{}", command);
            if self.always_require_approval.contains(&cmd_key) {
                return true;
            }
        }
        false
    }

    pub fn evaluate_fs_operation(
        &self,
        _scope: &FsScope,
        operation: &FsOperation,
        trust_level: TrustLevel,
    ) -> PolicyDecision {
        let policy = self.get_workspace_policy(trust_level);

        match operation {
            FsOperation::Read | FsOperation::List => {
                if policy.allow_read {
                    PolicyDecision::Allow
                } else {
                    PolicyDecision::Deny
                }
            }
            FsOperation::Write | FsOperation::CreateDir => {
                if !policy.allow_write {
                    PolicyDecision::Deny
                } else if policy.require_approval_for_write {
                    PolicyDecision::RequireApproval
                } else {
                    PolicyDecision::Allow
                }
            }
            FsOperation::Delete => {
                if !policy.allow_delete {
                    PolicyDecision::Deny
                } else if policy.require_approval_for_delete {
                    PolicyDecision::RequireApproval
                } else {
                    PolicyDecision::Allow
                }
            }
        }
    }
}

fn operation_key(op: &Operation) -> String {
    match op {
        Operation::Filesystem { scope, operation, .. } => {
            format!("fs:{}:{}", operation, scope)
        }
        Operation::Shell { scope, command, .. } => {
            format!("shell:{}:{}", scope, command)
        }
        Operation::Network { endpoint, method, .. } => {
            format!("network:{}:{}", method, endpoint)
        }
    }
}