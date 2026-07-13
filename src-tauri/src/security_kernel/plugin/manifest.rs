use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::PluginPermission;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for PluginId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub permissions: Vec<PluginPermission>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
}

impl PluginManifest {
    pub fn new(id: PluginId, name: String, version: String) -> Self {
        Self {
            id,
            name,
            version,
            permissions: Vec::new(),
            author: None,
            description: None,
            homepage: None,
            license: None,
        }
    }

    pub fn with_permission(mut self, permission: PluginPermission) -> Self {
        self.permissions.push(permission);
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<PluginPermission>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn has_permission(&self, permission: &PluginPermission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn permission_count(&self) -> usize {
        self.permissions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }

    pub fn risk_level(&self) -> PluginRiskLevel {
        if self.permissions.is_empty() {
            return PluginRiskLevel::Low;
        }

        let has_write = self.permissions.iter().any(|p| p.is_write_operation());
        let has_shell = self.permissions.iter().any(|p| p.is_shell());
        let has_network = self.permissions.iter().any(|p| p.is_network());
        let has_critical = self.permissions.iter().any(|p| p.is_critical());

        if has_critical {
            PluginRiskLevel::Critical
        } else if has_shell || (has_write && has_network) {
            PluginRiskLevel::High
        } else if has_write || has_network {
            PluginRiskLevel::Medium
        } else {
            PluginRiskLevel::Low
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PluginRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for PluginRiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestFile {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub permissions: Vec<PluginPermissionJson>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "scope")]
pub enum PluginPermissionJson {
    Filesystem { scope: String, operations: Vec<String> },
    Shell { runtime: String, operations: Vec<String> },
    Network { hosts: Vec<String>, allow_localhost: bool },
    Clipboard { read: bool, write: bool },
    Notification,
}

impl PluginManifestFile {
    pub fn to_manifest(&self) -> Result<PluginManifest, PluginManifestError> {
        let uuid = Uuid::parse_str(&self.id)
            .map_err(|_| PluginManifestError::InvalidId(self.id.clone()))?;
        let id = PluginId::from_uuid(uuid);

        let permissions = self.permissions
            .iter()
            .map(|p| p.to_permission())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PluginManifest {
            id,
            name: self.name.clone(),
            version: self.version.clone(),
            permissions,
            author: self.author.clone(),
            description: self.description.clone(),
            homepage: self.homepage.clone(),
            license: self.license.clone(),
        })
    }
}

impl PluginPermissionJson {
    pub fn to_permission(&self) -> Result<PluginPermission, PluginManifestError> {
        use crate::security_kernel::permission::{FsScope, FsOperation, NetworkEndpoint};
        use super::{FsPermission, ShellPermission, NetworkPermission, ClipboardPermission, NotificationPermission};

        match self {
            Self::Filesystem { scope, operations } => {
                let fs_scope = FsScope::from_str_name(scope)
                    .ok_or_else(|| PluginManifestError::InvalidScope(scope.clone()))?;

                let fs_ops = operations
                    .iter()
                    .map(|op| FsOperation::from_str_name(op)
                        .ok_or_else(|| PluginManifestError::InvalidOperation(op.clone())))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(PluginPermission::Filesystem(FsPermission::new(fs_scope, fs_ops)))
            }

            Self::Shell { runtime, operations } => {
                let shell_perm = match runtime.as_str() {
                    "git" => {
                        use crate::security_kernel::permission::GitOperation;
                        let ops = operations
                            .iter()
                            .map(|op| GitOperation::from_str_name(op)
                                .ok_or_else(|| PluginManifestError::InvalidOperation(op.clone())))
                            .collect::<Result<Vec<_>, _>>()?;
                        ShellPermission::Git(ops)
                    }
                    "cargo" => {
                        use crate::security_kernel::permission::CargoOperation;
                        let ops = operations
                            .iter()
                            .map(|op| CargoOperation::from_str_name(op)
                                .ok_or_else(|| PluginManifestError::InvalidOperation(op.clone())))
                            .collect::<Result<Vec<_>, _>>()?;
                        ShellPermission::Cargo(ops)
                    }
                    "python" => ShellPermission::Python(operations.clone()),
                    "node" => ShellPermission::Node(operations.clone()),
                    _ => return Err(PluginManifestError::InvalidRuntime(runtime.clone())),
                };
                Ok(PluginPermission::Shell(shell_perm))
            }

            Self::Network { hosts, allow_localhost } => {
                let endpoints = hosts
                    .iter()
                    .map(|h| NetworkEndpoint::https(h.clone()))
                    .collect();
                Ok(PluginPermission::Network(NetworkPermission::new(endpoints, *allow_localhost)))
            }

            Self::Clipboard { read, write } => {
                Ok(PluginPermission::Clipboard(ClipboardPermission::new(*read, *write)))
            }

            Self::Notification => {
                Ok(PluginPermission::Notification(NotificationPermission::new()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum PluginManifestError {
    InvalidId(String),
    InvalidScope(String),
    InvalidOperation(String),
    InvalidRuntime(String),
    MissingField(String),
    ParseError(String),
}

impl fmt::Display for PluginManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "Invalid plugin ID: {}", id),
            Self::InvalidScope(scope) => write!(f, "Invalid filesystem scope: {}", scope),
            Self::InvalidOperation(op) => write!(f, "Invalid operation: {}", op),
            Self::InvalidRuntime(runtime) => write!(f, "Invalid shell runtime: {}", runtime),
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for PluginManifestError {}