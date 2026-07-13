use std::path::Path;
use std::sync::{Arc, Mutex};

use super::{
    PluginId, PluginManifest, PluginManifestFile,
    PluginPermission, PluginRegistry, PluginRiskLevel,
    PermissionCheckResult,
};
use crate::shared::error::AppError;
use crate::security_kernel::permission::{FsScope, FsOperation};

pub struct PluginSecurity {
    registry: Arc<Mutex<PluginRegistry>>,
    approval_callback: Option<ApprovalCallback>,
}

type ApprovalCallback = Arc<dyn Fn(&PluginManifest, &[PluginPermission]) -> bool + Send + Sync>;

impl PluginSecurity {
    pub fn new(registry: Arc<Mutex<PluginRegistry>>) -> Self {
        Self {
            registry,
            approval_callback: None,
        }
    }

    pub fn with_approval_callback(
        mut self,
        callback: impl Fn(&PluginManifest, &[PluginPermission]) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.approval_callback = Some(Arc::new(callback));
        self
    }

    pub fn load_manifest(&self, plugin_path: &Path) -> Result<PluginManifest, PluginLoadError> {
        if !plugin_path.exists() {
            return Err(PluginLoadError::NotFound(plugin_path.to_path_buf()));
        }

        let manifest_path = plugin_path.join("manifest.json");
        if !manifest_path.exists() {
            return Err(PluginLoadError::MissingManifest(plugin_path.to_path_buf()));
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::ReadError(manifest_path.clone(), e.to_string()))?;

        let manifest_file: PluginManifestFile = serde_json::from_str(&content)
            .map_err(|e| PluginLoadError::ParseError(manifest_path.clone(), e.to_string()))?;

        manifest_file.to_manifest()
            .map_err(|e| PluginLoadError::InvalidManifest(manifest_path.clone(), e.to_string()))
    }

    pub fn register_plugin(&self, manifest: PluginManifest) -> Result<(), AppError> {
        let mut registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        registry.register(manifest)
    }

    pub fn unregister_plugin(&self, plugin_id: &PluginId) -> Result<(), AppError> {
        let mut registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        registry.unregister(plugin_id)
    }

    pub fn request_permission(
        &self,
        plugin_id: &PluginId,
        permissions: &[PluginPermission],
    ) -> Result<PermissionApprovalResult, AppError> {
        let registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        let record = registry.get(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        let manifest = &record.manifest;

        let undeclared = permissions.iter()
            .filter(|p| !manifest.permissions.contains(p))
            .cloned()
            .collect::<Vec<_>>();

        if !undeclared.is_empty() {
            return Err(AppError::permission_denied(
                format!("Permissions not declared: {}", 
                    undeclared.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "))
            ));
        }

        let risk_level = manifest.risk_level();
        let requires_approval = risk_level >= PluginRiskLevel::Medium || 
            permissions.iter().any(|p| p.is_write_operation());

        if !requires_approval {
            return Ok(PermissionApprovalResult::AutoApproved);
        }

        if let Some(callback) = &self.approval_callback {
            let approved = callback(manifest, permissions);
            if approved {
                Ok(PermissionApprovalResult::UserApproved)
            } else {
                Ok(PermissionApprovalResult::UserRejected)
            }
        } else {
            Ok(PermissionApprovalResult::PendingUserApproval)
        }
    }

    pub fn grant_permissions(&self, plugin_id: &PluginId, permissions: &[PluginPermission]) -> Result<(), AppError> {
        let mut registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        for permission in permissions {
            registry.grant_permission(plugin_id, permission.clone())?;
        }

        Ok(())
    }

    pub fn check_permission(&self, plugin_id: &PluginId, operation: &PluginOperation) -> Result<(), AppError> {
        let registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        let permission = operation.to_permission();

        let result = registry.check_permission(plugin_id, &permission);

        match result {
            PermissionCheckResult::Granted => Ok(()),
            PermissionCheckResult::Pending => Err(AppError::permission_denied(
                "Permission pending approval"
            )),
            PermissionCheckResult::NotDeclared => Err(AppError::permission_denied(
                format!("Permission {} not declared in manifest", permission)
            )),
            PermissionCheckResult::PluginNotFound => Err(AppError::not_found(
                format!("Plugin {} not found", plugin_id)
            )),
            PermissionCheckResult::PluginDisabled => Err(AppError::permission_denied(
                "Plugin is disabled"
            )),
        }
    }

    pub fn check_fs_operation(
        &self,
        plugin_id: &PluginId,
        scope: FsScope,
        operation: FsOperation,
    ) -> Result<(), AppError> {
        let _perm = PluginPermission::filesystem(scope, vec![operation]);
        self.check_permission(plugin_id, &PluginOperation::Filesystem { scope, operation })
    }

    pub fn check_network_access(
        &self,
        plugin_id: &PluginId,
        host: &str,
    ) -> Result<(), AppError> {
        let registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        let record = registry.get(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        if !record.enabled {
            return Err(AppError::permission_denied("Plugin is disabled"));
        }

        let has_network_perm = record.manifest.permissions.iter()
            .filter_map(|p| if let PluginPermission::Network(net) = p { Some(net) } else { None })
            .any(|net| net.can_access_host(host));

        if !has_network_perm {
            return Err(AppError::permission_denied(
                format!("Network access to {} not permitted", host)
            ));
        }

        if !record.granted_permissions.iter()
            .filter_map(|p| if let PluginPermission::Network(net) = p { Some(net) } else { None })
            .any(|net| net.can_access_host(host)) {
            return Err(AppError::permission_denied(
                "Network permission pending approval"
            ));
        }

        Ok(())
    }

    pub fn get_plugin_info(&self, plugin_id: &PluginId) -> Result<PluginInfo, AppError> {
        let registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        let record = registry.get(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        Ok(PluginInfo {
            id: record.manifest.id,
            name: record.manifest.name.clone(),
            version: record.manifest.version.clone(),
            risk_level: record.manifest.risk_level(),
            declared_permissions: record.manifest.permissions.clone(),
            granted_permissions: record.granted_permissions.iter().cloned().collect(),
            pending_permissions: record.pending_permissions(),
            enabled: record.enabled,
        })
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>, AppError> {
        let registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        Ok(registry.list_plugins().iter().map(|r| PluginInfo {
            id: r.manifest.id,
            name: r.manifest.name.clone(),
            version: r.manifest.version.clone(),
            risk_level: r.manifest.risk_level(),
            declared_permissions: r.manifest.permissions.clone(),
            granted_permissions: r.granted_permissions.iter().cloned().collect(),
            pending_permissions: r.pending_permissions(),
            enabled: r.enabled,
        }).collect())
    }

    pub fn enable_plugin(&self, plugin_id: &PluginId) -> Result<(), AppError> {
        let mut registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        let record = registry.get_mut(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        record.enable();
        Ok(())
    }

    pub fn disable_plugin(&self, plugin_id: &PluginId) -> Result<(), AppError> {
        let mut registry = self.registry.lock()
            .map_err(|_| AppError::internal("Failed to lock registry"))?;

        let record = registry.get_mut(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        record.disable();
        Ok(())
    }
}

impl Default for PluginSecurity {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(PluginRegistry::new())))
    }
}

#[derive(Debug, Clone)]
pub enum PluginOperation {
    Filesystem { scope: FsScope, operation: FsOperation },
    Shell { command: String },
    Network { host: String },
    ClipboardRead,
    ClipboardWrite,
    Notification,
}

impl PluginOperation {
    pub fn to_permission(&self) -> PluginPermission {
        match self {
            Self::Filesystem { scope, operation } => {
                PluginPermission::filesystem(*scope, vec![*operation])
            }
            Self::Shell { command: _ } => {
                PluginPermission::shell_git(vec![])
            }
            Self::Network { host } => {
                PluginPermission::network(vec![host.clone()], false)
            }
            Self::ClipboardRead => PluginPermission::clipboard(true, false),
            Self::ClipboardWrite => PluginPermission::clipboard(false, true),
            Self::Notification => PluginPermission::notification(),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Filesystem { scope, operation } => {
                format!("fs:{}:{}", scope, operation)
            }
            Self::Shell { command } => format!("shell:{}", command),
            Self::Network { host } => format!("network:{}", host),
            Self::ClipboardRead => "clipboard:read".to_string(),
            Self::ClipboardWrite => "clipboard:write".to_string(),
            Self::Notification => "notification".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub risk_level: PluginRiskLevel,
    pub declared_permissions: Vec<PluginPermission>,
    pub granted_permissions: Vec<PluginPermission>,
    pub pending_permissions: Vec<PluginPermission>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionApprovalResult {
    AutoApproved,
    UserApproved,
    UserRejected,
    PendingUserApproval,
}

impl PermissionApprovalResult {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::AutoApproved | Self::UserApproved)
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::UserRejected)
    }

    pub fn requires_ui(&self) -> bool {
        matches!(self, Self::PendingUserApproval)
    }
}

#[derive(Debug, Clone)]
pub enum PluginLoadError {
    NotFound(std::path::PathBuf),
    MissingManifest(std::path::PathBuf),
    ReadError(std::path::PathBuf, String),
    ParseError(std::path::PathBuf, String),
    InvalidManifest(std::path::PathBuf, String),
}

impl std::fmt::Display for PluginLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "Plugin directory not found: {}", path.display()),
            Self::MissingManifest(path) => write!(f, "Missing manifest.json in {}", path.display()),
            Self::ReadError(path, err) => write!(f, "Failed to read manifest at {}: {}", path.display(), err),
            Self::ParseError(path, err) => write!(f, "Failed to parse manifest at {}: {}", path.display(), err),
            Self::InvalidManifest(path, err) => write!(f, "Invalid manifest at {}: {}", path.display(), err),
        }
    }
}

impl std::error::Error for PluginLoadError {}

impl From<PluginLoadError> for AppError {
    fn from(err: PluginLoadError) -> Self {
        AppError::internal(err.to_string())
    }
}