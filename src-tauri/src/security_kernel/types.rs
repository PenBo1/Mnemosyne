//! ═══════════════════════════════════════════════════════════════════════════
//! types - 安全内核类型定义模块
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApprovalToken(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapabilityId,
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub resource: Resource,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    FileSystem { path_pattern: String },
    Network { url_pattern: String },
    Database { table: String },
    System { command_pattern: String },
    Memory { scope: String },
    Custom { namespace: String, resource_type: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Read,
    Write,
    Execute,
    Delete,
    Admin,
}

/// 工作区信任级别（从 security_kernel/workspace/trust_level.rs 迁入）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    Unknown,
    Trusted,
    Enterprise,
    Readonly,
    Dangerous,
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Trusted => write!(f, "trusted"),
            Self::Enterprise => write!(f, "enterprise"),
            Self::Readonly => write!(f, "readonly"),
            Self::Dangerous => write!(f, "dangerous"),
        }
    }
}

impl TrustLevel {
    pub fn can_write(&self) -> bool {
        matches!(self, Self::Trusted | Self::Enterprise)
    }

    pub fn can_execute(&self) -> bool {
        matches!(self, Self::Trusted | Self::Enterprise)
    }

    pub fn can_network(&self) -> bool {
        matches!(self, Self::Trusted | Self::Enterprise | Self::Unknown)
    }

    pub fn is_restricted(&self) -> bool {
        matches!(self, Self::Dangerous | Self::Readonly | Self::Unknown)
    }
}

#[derive(Debug, Clone)]
pub struct OperationContext {
    pub workspace: WorkspaceId,
    pub user: UserId,
    pub session: SessionId,
    pub approval_token: Option<ApprovalToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub capability_id: CapabilityId,
    pub resource_pattern: String,
    pub allowed_actions: Vec<Action>,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    MaxSize(u64),
    RateLimit { requests: u32, window_secs: u64 },
    TimeWindow { start: String, end: String },
    RequireApproval,
    AuditLog,
}