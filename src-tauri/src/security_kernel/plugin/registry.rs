use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::{PluginId, PluginManifest, PluginPermission, PluginRiskLevel};
use crate::shared::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantedPermissionKey {
    pub plugin_id: PluginId,
    pub permission: PluginPermission,
}

#[derive(Debug, Clone)]
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub granted_permissions: HashSet<PluginPermission>,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

impl PluginRecord {
    pub fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            granted_permissions: HashSet::new(),
            loaded_at: chrono::Utc::now(),
            last_used: None,
            enabled: true,
        }
    }

    pub fn grant_permission(&mut self, permission: PluginPermission) {
        self.granted_permissions.insert(permission);
    }

    pub fn revoke_permission(&mut self, permission: &PluginPermission) {
        self.granted_permissions.remove(permission);
    }

    pub fn is_permission_granted(&self, permission: &PluginPermission) -> bool {
        self.granted_permissions.contains(permission)
    }

    pub fn grant_all_declared(&mut self) {
        for perm in self.manifest.permissions.iter() {
            self.granted_permissions.insert(perm.clone());
        }
    }

    pub fn revoke_all(&mut self) {
        self.granted_permissions.clear();
    }

    pub fn pending_permissions(&self) -> Vec<PluginPermission> {
        self.manifest.permissions.iter()
            .filter(|p| !self.granted_permissions.contains(p))
            .cloned()
            .collect()
    }

    pub fn has_pending_permissions(&self) -> bool {
        self.manifest.permissions.iter()
            .any(|p| !self.granted_permissions.contains(p))
    }

    pub fn mark_used(&mut self) {
        self.last_used = Some(chrono::Utc::now());
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

pub struct PluginRegistry {
    plugins: HashMap<PluginId, PluginRecord>,
    pending_requests: HashMap<PluginId, Vec<PluginPermission>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            pending_requests: HashMap::new(),
        }
    }

    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), AppError> {
        if self.plugins.contains_key(&manifest.id) {
            return Err(AppError::duplicate(format!("Plugin {} already registered", manifest.id)));
        }

        let id = manifest.id;
        let record = PluginRecord::new(manifest);
        self.plugins.insert(id, record);
        
        Ok(())
    }

    pub fn unregister(&mut self, plugin_id: &PluginId) -> Result<(), AppError> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(AppError::not_found(format!("Plugin {} not found", plugin_id)));
        }

        self.plugins.remove(plugin_id);
        self.pending_requests.remove(plugin_id);
        
        Ok(())
    }

    pub fn get(&self, plugin_id: &PluginId) -> Option<&PluginRecord> {
        self.plugins.get(plugin_id)
    }

    pub fn get_mut(&mut self, plugin_id: &PluginId) -> Option<&mut PluginRecord> {
        self.plugins.get_mut(plugin_id)
    }

    pub fn contains(&self, plugin_id: &PluginId) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    pub fn is_enabled(&self, plugin_id: &PluginId) -> bool {
        self.plugins.get(plugin_id)
            .map(|r| r.enabled)
            .unwrap_or(false)
    }

    pub fn grant_permission(&mut self, plugin_id: &PluginId, permission: PluginPermission) -> Result<(), AppError> {
        let record = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        if !record.manifest.permissions.contains(&permission) {
            return Err(AppError::permission_denied(
                format!("Permission {} not declared in plugin manifest", permission)
            ));
        }

        record.grant_permission(permission);
        Ok(())
    }

    pub fn grant_all_permissions(&mut self, plugin_id: &PluginId) -> Result<(), AppError> {
        let record = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        record.grant_all_declared();
        Ok(())
    }

    pub fn revoke_permission(&mut self, plugin_id: &PluginId, permission: &PluginPermission) -> Result<(), AppError> {
        let record = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        record.revoke_permission(permission);
        Ok(())
    }

    pub fn revoke_all_permissions(&mut self, plugin_id: &PluginId) -> Result<(), AppError> {
        let record = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::not_found(format!("Plugin {} not found", plugin_id)))?;

        record.revoke_all();
        Ok(())
    }

    pub fn check_permission(&self, plugin_id: &PluginId, permission: &PluginPermission) -> PermissionCheckResult {
        let record = self.plugins.get(plugin_id);

        match record {
            None => PermissionCheckResult::PluginNotFound,
            Some(r) if !r.enabled => PermissionCheckResult::PluginDisabled,
            Some(r) if !r.manifest.permissions.contains(permission) => {
                PermissionCheckResult::NotDeclared
            }
            Some(r) if r.is_permission_granted(permission) => {
                PermissionCheckResult::Granted
            }
            Some(_) => PermissionCheckResult::Pending,
        }
    }

    pub fn list_plugins(&self) -> Vec<&PluginRecord> {
        self.plugins.values().collect()
    }

    pub fn list_enabled(&self) -> Vec<&PluginRecord> {
        self.plugins.values().filter(|r| r.enabled).collect()
    }

    pub fn list_by_risk(&self, level: PluginRiskLevel) -> Vec<&PluginRecord> {
        self.plugins.values()
            .filter(|r| r.manifest.risk_level() == level)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    pub fn clear(&mut self) {
        self.plugins.clear();
        self.pending_requests.clear();
    }

    pub fn add_pending_request(&mut self, plugin_id: PluginId, permissions: Vec<PluginPermission>) {
        self.pending_requests.insert(plugin_id, permissions);
    }

    pub fn get_pending_request(&self, plugin_id: &PluginId) -> Option<&Vec<PluginPermission>> {
        self.pending_requests.get(plugin_id)
    }

    pub fn remove_pending_request(&mut self, plugin_id: &PluginId) {
        self.pending_requests.remove(plugin_id);
    }

    pub fn has_pending_requests(&self) -> bool {
        !self.pending_requests.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionCheckResult {
    Granted,
    Pending,
    NotDeclared,
    PluginNotFound,
    PluginDisabled,
}

impl PermissionCheckResult {
    pub fn is_granted(&self) -> bool {
        matches!(self, Self::Granted)
    }

    pub fn is_denied(&self) -> bool {
        !self.is_granted()
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::Pending)
    }

    pub fn error_message(&self) -> Option<String> {
        match self {
            Self::Granted => None,
            Self::Pending => Some("Permission pending approval".to_string()),
            Self::NotDeclared => Some("Permission not declared in manifest".to_string()),
            Self::PluginNotFound => Some("Plugin not found".to_string()),
            Self::PluginDisabled => Some("Plugin is disabled".to_string()),
        }
    }
}

pub type SharedPluginRegistry = Arc<Mutex<PluginRegistry>>;

pub fn create_shared_registry() -> SharedPluginRegistry {
    Arc::new(Mutex::new(PluginRegistry::new()))
}