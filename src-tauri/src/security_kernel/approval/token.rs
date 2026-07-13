use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::security_kernel::permission::Operation;
use crate::security_kernel::policy::OperationRisk;
use crate::security_kernel::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApprovalId(pub Uuid);

impl ApprovalId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ApprovalId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalToken {
    pub id: ApprovalId,
    pub workspace: WorkspaceId,
    pub action_hash: String,
    pub expire: DateTime<Utc>,
    pub risk_level: OperationRisk,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ApprovalToken {
    pub const DEFAULT_TTL_SECONDS: i64 = 30;

    pub fn new(op: &Operation, workspace: WorkspaceId, risk_level: OperationRisk) -> Self {
        let action_hash = calculate_action_hash(op);
        let now = Utc::now();
        Self {
            id: ApprovalId::new(),
            workspace,
            action_hash,
            expire: now + Duration::seconds(Self::DEFAULT_TTL_SECONDS),
            risk_level,
            approved_by: None,
            created_at: now,
        }
    }

    pub fn with_ttl(op: &Operation, workspace: WorkspaceId, risk_level: OperationRisk, ttl_seconds: i64) -> Self {
        let action_hash = calculate_action_hash(op);
        let now = Utc::now();
        Self {
            id: ApprovalId::new(),
            workspace,
            action_hash,
            expire: now + Duration::seconds(ttl_seconds),
            risk_level,
            approved_by: None,
            created_at: now,
        }
    }

    pub fn with_approved_by(mut self, approved_by: impl Into<String>) -> Self {
        self.approved_by = Some(approved_by.into());
        self
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expire
    }

    pub fn remaining_seconds(&self) -> i64 {
        let now = Utc::now();
        if now > self.expire {
            0
        } else {
            (self.expire - now).num_seconds()
        }
    }

    pub fn matches_operation(&self, op: &Operation) -> bool {
        let hash = calculate_action_hash(op);
        self.action_hash == hash
    }

    pub fn matches_workspace(&self, workspace: &WorkspaceId) -> bool {
        self.workspace == *workspace
    }
}

pub fn calculate_action_hash(op: &Operation) -> String {
    let mut hasher = Sha256::new();

    match op {
        Operation::Filesystem { scope, operation, path } => {
            hasher.update("filesystem");
            hasher.update(scope.to_string().as_bytes());
            hasher.update(operation.to_string().as_bytes());
            hasher.update(path.as_bytes());
        }
        Operation::Shell { scope, command, args } => {
            hasher.update("shell");
            hasher.update(scope.to_string().as_bytes());
            hasher.update(command.as_bytes());
            let args_json = serde_json::to_string(args).unwrap_or_default();
            hasher.update(args_json.as_bytes());
        }
        Operation::Network { scope, endpoint, method } => {
            hasher.update("network");
            hasher.update(scope.to_string().as_bytes());
            hasher.update(endpoint.as_bytes());
            hasher.update(method.as_bytes());
        }
    }

    let result = hasher.finalize();
    hex::encode(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub token: ApprovalToken,
    pub operation: Operation,
    pub workspace: WorkspaceId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResult {
    pub token_id: ApprovalId,
    pub approved: bool,
    pub approved_by: Option<String>,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_kernel::permission::{FsOperation, FsScope, Operation};
    use uuid::Uuid;

    fn make_workspace() -> WorkspaceId {
        WorkspaceId(Uuid::new_v4())
    }

    fn make_fs_op(path: &str) -> Operation {
        Operation::Filesystem {
            scope: FsScope::Workspace,
            operation: FsOperation::Write,
            path: path.to_string(),
        }
    }

    fn make_shell_op(command: &str) -> Operation {
        Operation::Shell {
            scope: crate::security_kernel::permission::ShellScope::git(
                vec![crate::security_kernel::permission::GitOperation::Commit]
            ),
            command: command.to_string(),
            args: vec!["-m".to_string(), "test".to_string()],
        }
    }

    fn make_network_op(endpoint: &str) -> Operation {
        Operation::Network {
            scope: crate::security_kernel::permission::NetworkScope::provider(
                crate::security_kernel::permission::NetworkEndpoint::https(endpoint)
            ),
            endpoint: endpoint.to_string(),
            method: "POST".to_string(),
        }
    }

    #[test]
    fn test_approval_id_new() {
        let id = ApprovalId::new();
        assert_ne!(id.0, Uuid::nil());
    }

    #[test]
    fn test_approval_id_default() {
        let id = ApprovalId::default();
        assert_ne!(id.0, Uuid::nil());
    }

    #[test]
    fn test_approval_token_new() {
        let workspace = make_workspace();
        let op = make_fs_op("/test/path");
        let token = ApprovalToken::new(&op, workspace, OperationRisk::High);

        assert_ne!(token.id.0, Uuid::nil());
        assert_eq!(token.workspace, workspace);
        assert!(!token.action_hash.is_empty());
        assert_eq!(token.risk_level, OperationRisk::High);
        assert!(token.approved_by.is_none());
    }

    #[test]
    fn test_approval_token_default_ttl() {
        let workspace = make_workspace();
        let op = make_fs_op("/test");
        let token = ApprovalToken::new(&op, workspace, OperationRisk::Medium);

        let remaining = token.remaining_seconds();
        assert!(remaining > 0);
        assert!(remaining <= ApprovalToken::DEFAULT_TTL_SECONDS);
    }

    #[test]
    fn test_approval_token_custom_ttl() {
        let workspace = make_workspace();
        let op = make_fs_op("/test");
        let custom_ttl = 60;
        let token = ApprovalToken::with_ttl(&op, workspace, OperationRisk::High, custom_ttl);

        let remaining = token.remaining_seconds();
        assert!(remaining > 0);
        assert!(remaining <= custom_ttl);
    }

    #[test]
    fn test_approval_token_is_expired() {
        let workspace = make_workspace();
        let op = make_fs_op("/test");

        let valid_token = ApprovalToken::new(&op, workspace, OperationRisk::Medium);
        assert!(!valid_token.is_expired());

        let expired_token = ApprovalToken {
            id: ApprovalId::new(),
            workspace,
            action_hash: calculate_action_hash(&op),
            expire: chrono::Utc::now() - chrono::Duration::seconds(10),
            risk_level: OperationRisk::High,
            approved_by: None,
            created_at: chrono::Utc::now() - chrono::Duration::seconds(40),
        };
        assert!(expired_token.is_expired());
        assert_eq!(expired_token.remaining_seconds(), 0);
    }

    #[test]
    fn test_approval_token_with_approved_by() {
        let workspace = make_workspace();
        let op = make_fs_op("/test");
        let token = ApprovalToken::new(&op, workspace, OperationRisk::High)
            .with_approved_by("admin_user");

        assert_eq!(token.approved_by, Some("admin_user".to_string()));
    }

    #[test]
    fn test_approval_token_matches_operation() {
        let workspace = make_workspace();
        let op1 = make_fs_op("/test/path");
        let op2 = make_fs_op("/test/path");
        let op3 = make_fs_op("/different/path");

        let token = ApprovalToken::new(&op1, workspace, OperationRisk::Medium);

        assert!(token.matches_operation(&op2));
        assert!(!token.matches_operation(&op3));
    }

    #[test]
    fn test_approval_token_matches_workspace() {
        let workspace1 = make_workspace();
        let workspace2 = make_workspace();
        let op = make_fs_op("/test");

        let token = ApprovalToken::new(&op, workspace1, OperationRisk::Medium);

        assert!(token.matches_workspace(&workspace1));
        assert!(!token.matches_workspace(&workspace2));
    }

    #[test]
    fn test_calculate_action_hash_fs_operation() {
        let op1 = make_fs_op("/test/path");
        let op2 = make_fs_op("/test/path");
        let op3 = make_fs_op("/different/path");

        let hash1 = calculate_action_hash(&op1);
        let hash2 = calculate_action_hash(&op2);
        let hash3 = calculate_action_hash(&op3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_calculate_action_hash_shell_operation() {
        let op1 = make_shell_op("git");
        let op2 = make_shell_op("git");
        let op3 = make_shell_op("npm");

        let hash1 = calculate_action_hash(&op1);
        let hash2 = calculate_action_hash(&op2);
        let hash3 = calculate_action_hash(&op3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_calculate_action_hash_network_operation() {
        let op1 = make_network_op("api.openai.com");
        let op2 = make_network_op("api.openai.com");
        let op3 = make_network_op("api.anthropic.com");

        let hash1 = calculate_action_hash(&op1);
        let hash2 = calculate_action_hash(&op2);
        let hash3 = calculate_action_hash(&op3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_calculate_action_hash_different_operations() {
        let fs_op = make_fs_op("/test");
        let shell_op = make_shell_op("git");
        let network_op = make_network_op("api.example.com");

        let fs_hash = calculate_action_hash(&fs_op);
        let shell_hash = calculate_action_hash(&shell_op);
        let network_hash = calculate_action_hash(&network_op);

        assert_ne!(fs_hash, shell_hash);
        assert_ne!(fs_hash, network_hash);
        assert_ne!(shell_hash, network_hash);
    }

    #[test]
    fn test_approval_request_creation() {
        let workspace = make_workspace();
        let op = make_fs_op("/test");
        let token = ApprovalToken::new(&op, workspace, OperationRisk::High);

        let request = ApprovalRequest {
            token,
            operation: op.clone(),
            workspace,
            reason: "Need to write file".to_string(),
        };

        assert_eq!(request.workspace, workspace);
        assert_eq!(request.reason, "Need to write file");
    }

    #[test]
    fn test_approval_result_creation() {
        let token_id = ApprovalId::new();

        let result_approved = ApprovalResult {
            token_id,
            approved: true,
            approved_by: Some("admin".to_string()),
            message: "Approved".to_string(),
        };
        assert!(result_approved.approved);

        let result_rejected = ApprovalResult {
            token_id,
            approved: false,
            approved_by: None,
            message: "Rejected".to_string(),
        };
        assert!(!result_rejected.approved);
    }
}