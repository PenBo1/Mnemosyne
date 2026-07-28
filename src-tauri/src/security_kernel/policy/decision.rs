//! ═══════════════════════════════════════════════════════════════════════════
//! decision - 策略决策定义模块
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::security_kernel::permission::{FsOperation, FsScope, Operation};

// ── 策略决策 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    RequireApproval,
    Deny,
}

impl fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::RequireApproval => write!(f, "require_approval"),
            Self::Deny => write!(f, "deny"),
        }
    }
}

// ── 操作风险等级 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for OperationRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl OperationRisk {
    pub fn default_decision(&self) -> PolicyDecision {
        match self {
            Self::Low => PolicyDecision::Allow,
            Self::Medium => PolicyDecision::RequireApproval,
            Self::High => PolicyDecision::RequireApproval,
            Self::Critical => PolicyDecision::Deny,
        }
    }

    pub fn requires_audit(&self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::Critical)
    }
}

/// 计算操作风险等级
pub fn calculate_operation_risk(op: &Operation) -> OperationRisk {
    match op {
        Operation::Filesystem { scope, operation, .. } => {
            calculate_fs_risk(scope, operation)
        }
        Operation::Shell { scope, .. } => {
            calculate_shell_risk(scope)
        }
        Operation::Network { method, .. } => {
            calculate_network_risk(method)
        }
    }
}

fn calculate_fs_risk(scope: &FsScope, operation: &FsOperation) -> OperationRisk {
    match operation {
        FsOperation::Read | FsOperation::List => OperationRisk::Low,
        FsOperation::Write | FsOperation::CreateDir => {
            if scope.is_writable() {
                OperationRisk::Medium
            } else {
                OperationRisk::Critical
            }
        }
        FsOperation::Delete => {
            if *scope == FsScope::Workspace {
                OperationRisk::High
            } else {
                OperationRisk::Critical
            }
        }
    }
}

fn calculate_shell_risk(scope: &crate::security_kernel::permission::ShellScope) -> OperationRisk {
    
    use crate::security_kernel::permission::ShellScope;

    match scope {
        ShellScope::Git { operations } => {
            let has_write = operations.iter().any(|op| op.is_write_operation());
            if has_write {
                OperationRisk::High
            } else {
                OperationRisk::Low
            }
        }
        ShellScope::Python { .. } => OperationRisk::Medium,
        ShellScope::Node { .. } => OperationRisk::Medium,
        ShellScope::Cargo { operations } => {
            let has_publish = operations.iter().any(|op| {
                matches!(op, crate::security_kernel::permission::CargoOperation::Publish)
            });
            if has_publish {
                OperationRisk::Critical
            } else {
                OperationRisk::Medium
            }
        }
    }
}

fn calculate_network_risk(method: &str) -> OperationRisk {
    match method.to_uppercase().as_str() {
        "GET" | "HEAD" | "OPTIONS" => OperationRisk::Low,
        "POST" | "PUT" | "PATCH" => OperationRisk::Medium,
        "DELETE" => OperationRisk::High,
        _ => OperationRisk::Medium,
    }
}

// ── 策略评估结果 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub decision: PolicyDecision,
    pub risk: OperationRisk,
    pub source: PolicySource,
    pub reason: String,
}

// ── 策略来源 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicySource {
    TemporaryOverride,
    UserOverride,
    WorkspaceOverride,
    GlobalPolicy,
}

impl fmt::Display for PolicySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemporaryOverride => write!(f, "temporary_override"),
            Self::UserOverride => write!(f, "user_override"),
            Self::WorkspaceOverride => write!(f, "workspace_override"),
            Self::GlobalPolicy => write!(f, "global_policy"),
        }
    }
}