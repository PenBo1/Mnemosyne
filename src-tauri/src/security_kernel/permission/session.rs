//! ═══════════════════════════════════════════════════════════════════════════
//! session - 权限会话管理
//! ═══════════════════════════════════════════════════════════════════════════

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::shared::error::AppError;

use super::{Capability, FsOperation, FsScope, GitOperation, NetworkScope, ShellScope};

pub type WorkspaceId = String;

/// Session 默认 TTL（30 分钟）。超时 session 在 check() 中被拒绝，强制重新创建。
const SESSION_TTL_MINUTES: i64 = 30;

// ── 权限会话 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSession {
    pub workspace: WorkspaceId,
    pub capabilities: HashSet<Capability>,
    pub opened_at: DateTime<Utc>,
}

impl PermissionSession {
    pub fn new(workspace: WorkspaceId, capabilities: HashSet<Capability>) -> Self {
        Self {
            workspace,
            capabilities,
            opened_at: Utc::now(),
        }
    }

    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn has_filesystem_scope(&self, scope: FsScope) -> bool {
        self.capabilities.iter().any(|c| {
            matches!(c, Capability::Filesystem(s, _) if *s == scope)
        })
    }

    pub fn has_shell_scope(&self) -> bool {
        self.capabilities.iter().any(|c| c.is_shell())
    }

    pub fn has_network_scope(&self) -> bool {
        self.capabilities.iter().any(|c| c.is_network())
    }
}

// ── 权限管理器 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PermissionManager {
    sessions: HashMap<WorkspaceId, PermissionSession>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self { sessions: HashMap::new() }
    }

    pub fn create_session(
        &mut self,
        workspace: WorkspaceId,
        capabilities: HashSet<Capability>,
    ) -> PermissionSession {
        let session = PermissionSession::new(workspace.clone(), capabilities);
        self.sessions.insert(workspace, session.clone());
        session
    }

    pub fn get_session(&self, workspace: &WorkspaceId) -> Option<&PermissionSession> {
        self.sessions.get(workspace)
    }

    pub fn remove_session(&mut self, workspace: &WorkspaceId) -> Option<PermissionSession> {
        self.sessions.remove(workspace)
    }

    pub fn list_sessions(&self) -> Vec<&PermissionSession> {
        self.sessions.values().collect()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn check(&self, op: &Operation, workspace: &WorkspaceId) -> Result<(), AppError> {
        // 系统工作区（nil UUID）跳过 permission check
        if workspace == &uuid::Uuid::nil().to_string() {
            return Ok(());
        }

        let session = self.sessions.get(workspace).ok_or_else(|| {
            AppError::workspace_not_found()
        })?;

        // Session TTL 检查 —— 超时 session 拒绝，强制调用方重新创建
        let now = Utc::now();
        if now > session.opened_at + chrono::Duration::minutes(SESSION_TTL_MINUTES) {
            return Err(AppError::forbidden(format!(
                "Permission session for workspace '{}' expired (opened_at: {}, ttl: {}min)",
                workspace, session.opened_at, SESSION_TTL_MINUTES
            )));
        }

        let required_capability = op.required_capability();
        if !session.capabilities.contains(&required_capability) {
            return Err(AppError::forbidden(format!(
                "Capability '{}' not allowed for workspace '{}'",
                required_capability, workspace
            )));
        }

        self.check_scope_constraints(op, session)?;

        Ok(())
    }

    fn check_scope_constraints(&self, op: &Operation, session: &PermissionSession) -> Result<(), AppError> {
        match op {
            Operation::Filesystem { scope, operation, .. } => {
                self.check_fs_scope(scope, operation, session)?;
            }
            Operation::Shell { scope, command, .. } => {
                self.check_shell_scope(scope, command, session)?;
            }
            Operation::Network { scope, endpoint, .. } => {
                self.check_network_scope(scope, endpoint, session)?;
            }
        }
        Ok(())
    }

    fn check_fs_scope(
        &self,
        scope: &FsScope,
        operation: &FsOperation,
        session: &PermissionSession,
    ) -> Result<(), AppError> {
        if operation.is_write_operation() && !scope.is_writable() {
            return Err(AppError::forbidden(format!(
                "Write operation '{}' not allowed on readonly scope '{}'",
                operation, scope
            )));
        }

        let required_cap = Capability::Filesystem(*scope, *operation);
        if !session.capabilities.contains(&required_cap) {
            return Err(AppError::forbidden(format!(
                "Filesystem operation '{}' on scope '{}' not permitted",
                operation, scope
            )));
        }

        Ok(())
    }

    fn check_shell_scope(
        &self,
        scope: &ShellScope,
        command: &str,
        session: &PermissionSession,
    ) -> Result<(), AppError> {
        let has_matching_scope = session.capabilities.iter().any(|c| {
            if let Capability::Shell(s) = c {
                self.shell_scope_matches(s, scope, command)
            } else {
                false
            }
        });

        if !has_matching_scope {
            return Err(AppError::forbidden(format!(
                "Shell command '{}' not permitted under scope '{}'",
                command, scope
            )));
        }

        Ok(())
    }

    fn shell_scope_matches(&self, granted: &ShellScope, requested: &ShellScope, _command: &str) -> bool {
        match (granted, requested) {
            (ShellScope::Git { operations: granted_ops }, ShellScope::Git { operations: req_ops }) => {
                req_ops.iter().all(|op| granted_ops.contains(op))
            }
            (ShellScope::Python { scripts: granted_scripts }, ShellScope::Python { scripts: req_scripts }) => {
                req_scripts.iter().all(|s| granted_scripts.contains(s))
            }
            (ShellScope::Node { scripts: granted_scripts }, ShellScope::Node { scripts: req_scripts }) => {
                req_scripts.iter().all(|s| granted_scripts.contains(s))
            }
            (ShellScope::Cargo { operations: granted_ops }, ShellScope::Cargo { operations: req_ops }) => {
                req_ops.iter().all(|op| granted_ops.contains(op))
            }
            _ => false,
        }
    }

    fn check_network_scope(
        &self,
        scope: &NetworkScope,
        endpoint: &str,
        session: &PermissionSession,
    ) -> Result<(), AppError> {
        let has_matching_scope = session.capabilities.iter().any(|c| {
            if let Capability::Network(s) = c {
                self.network_scope_matches(s, scope, endpoint)
            } else {
                false
            }
        });

        if !has_matching_scope {
            return Err(AppError::forbidden(format!(
                "Network endpoint '{}' not permitted under scope '{}'",
                endpoint, scope
            )));
        }

        Ok(())
    }

    fn network_scope_matches(&self, granted: &NetworkScope, requested: &NetworkScope, endpoint: &str) -> bool {
        let granted_endpoint = granted.endpoint();
        let requested_endpoint = requested.endpoint();

        if endpoint != granted_endpoint.host && endpoint != requested_endpoint.host {
            return false;
        }

        match (granted, requested) {
            (NetworkScope::Provider { .. }, NetworkScope::Provider { .. }) => true,
            (NetworkScope::MCP { allow_localhost: granted_local, .. }, NetworkScope::MCP { allow_localhost: req_local, .. }) => {
                if requested_endpoint.is_localhost() {
                    *granted_local && *req_local
                } else {
                    true
                }
            }
            (NetworkScope::Plugin { .. }, NetworkScope::Plugin { .. }) => true,
            _ => false,
        }
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── 操作枚举 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Filesystem {
        scope: FsScope,
        operation: FsOperation,
        path: String,
    },
    Shell {
        scope: ShellScope,
        command: String,
        args: Vec<String>,
    },
    Network {
        scope: NetworkScope,
        endpoint: String,
        method: String,
    },
}

impl Operation {
    pub fn fs_read(scope: FsScope, path: String) -> Self {
        Self::Filesystem { scope, operation: FsOperation::Read, path }
    }

    pub fn fs_write(scope: FsScope, path: String) -> Self {
        Self::Filesystem { scope, operation: FsOperation::Write, path }
    }

    pub fn fs_delete(scope: FsScope, path: String) -> Self {
        Self::Filesystem { scope, operation: FsOperation::Delete, path }
    }

    pub fn fs_list(scope: FsScope, path: String) -> Self {
        Self::Filesystem { scope, operation: FsOperation::List, path }
    }

    pub fn fs_create_dir(scope: FsScope, path: String) -> Self {
        Self::Filesystem { scope, operation: FsOperation::CreateDir, path }
    }

    pub fn git_status() -> Self {
        Self::Shell {
            scope: ShellScope::git(vec![GitOperation::Status]),
            command: "git".to_string(),
            args: vec!["status".to_string()],
        }
    }

    pub fn git_commit(message: String) -> Self {
        Self::Shell {
            scope: ShellScope::git(vec![GitOperation::Commit]),
            command: "git".to_string(),
            args: vec!["commit".to_string(), "-m".to_string(), message],
        }
    }

    pub fn network_request(scope: NetworkScope, endpoint: String, method: String) -> Self {
        Self::Network { scope, endpoint, method }
    }

    pub fn required_capability(&self) -> Capability {
        match self {
            Self::Filesystem { scope, operation, .. } => Capability::Filesystem(*scope, *operation),
            Self::Shell { scope, .. } => Capability::Shell(scope.clone()),
            Self::Network { scope, .. } => Capability::Network(scope.clone()),
        }
    }

    pub fn is_write_operation(&self) -> bool {
        match self {
            Self::Filesystem { operation, .. } => operation.is_write_operation(),
            Self::Shell { scope, .. } => {
                match scope {
                    ShellScope::Git { operations } => operations.iter().any(|o| o.is_write_operation()),
                    _ => true,
                }
            }
            Self::Network { method, .. } => method != "GET" && method != "HEAD",
        }
    }
}

// ── 默认能力集 ────────────────────────────────────────────────────────────────

pub fn default_workspace_capabilities() -> HashSet<Capability> {
    let mut caps = HashSet::new();

    caps.insert(Capability::filesystem_read(FsScope::Workspace));
    caps.insert(Capability::filesystem_write(FsScope::Workspace));
    caps.insert(Capability::filesystem_list(FsScope::Workspace));
    caps.insert(Capability::filesystem_create_dir(FsScope::Workspace));
    caps.insert(Capability::filesystem_read(FsScope::AppData));
    caps.insert(Capability::filesystem_read(FsScope::Resources));
    caps.insert(Capability::filesystem_read(FsScope::Templates));

    caps.insert(Capability::Shell(ShellScope::git_readonly()));

    caps.insert(Capability::Network(NetworkScope::provider(
        super::NetworkEndpoint::https("api.openai.com")
    )));
    caps.insert(Capability::Network(NetworkScope::provider(
        super::NetworkEndpoint::https("api.anthropic.com")
    )));

    caps
}

pub fn readonly_workspace_capabilities() -> HashSet<Capability> {
    let mut caps = HashSet::new();

    caps.insert(Capability::filesystem_read(FsScope::WorkspaceReadonly));
    caps.insert(Capability::filesystem_list(FsScope::WorkspaceReadonly));
    caps.insert(Capability::filesystem_read(FsScope::AppData));
    caps.insert(Capability::filesystem_read(FsScope::Resources));

    caps.insert(Capability::Shell(ShellScope::git_readonly()));

    caps.insert(Capability::Network(NetworkScope::provider(
        super::NetworkEndpoint::https("api.openai.com")
    )));

    caps
}