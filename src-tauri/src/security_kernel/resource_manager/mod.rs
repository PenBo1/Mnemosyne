mod quota;
mod monitor;
mod enforcer;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::shared::error::AppError;
use crate::security_kernel::types::WorkspaceId;

pub use quota::{ResourceType, ResourceQuota, ResourceUsage};
pub use monitor::{ResourceMonitor, SystemInfo};
pub use enforcer::ResourceEnforcer;

pub struct ResourceManager {
    quotas: HashMap<WorkspaceId, ResourceQuota>,
    usage: HashMap<WorkspaceId, ResourceUsage>,
    monitor: ResourceMonitor,
    enforcer: ResourceEnforcer,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            quotas: HashMap::new(),
            usage: HashMap::new(),
            monitor: ResourceMonitor::current_process(),
            enforcer: ResourceEnforcer::new(),
        }
    }

    pub fn shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }

    pub fn set_quota(&mut self, workspace: WorkspaceId, quota: ResourceQuota) {
        self.quotas.insert(workspace, quota);
    }

    pub fn get_quota(&self, workspace: &WorkspaceId) -> Option<&ResourceQuota> {
        self.quotas.get(workspace)
    }

    pub fn remove_quota(&mut self, workspace: &WorkspaceId) -> Option<ResourceQuota> {
        self.quotas.remove(workspace)
    }

    pub fn get_usage(&self, workspace: &WorkspaceId) -> Option<&ResourceUsage> {
        self.usage.get(workspace)
    }

    pub fn get_or_create_usage(&mut self, workspace: WorkspaceId) -> &mut ResourceUsage {
        self.usage.entry(workspace).or_default()
    }

    pub fn check_quota(&self, workspace: &WorkspaceId) -> Result<(), AppError> {
        let quota = self.quotas.get(workspace);
        let usage = self.usage.get(workspace);

        match (quota, usage) {
            (Some(q), Some(u)) => self.enforcer.enforce_quota(q, u),
            (Some(q), None) => self.enforcer.enforce_quota(q, &ResourceUsage::new()),
            (None, _) => Ok(()),
        }
    }

    pub fn check_quota_with_increment(
        &self,
        workspace: &WorkspaceId,
        resource: ResourceType,
        increment: f64,
    ) -> Result<(), AppError> {
        let quota = self.quotas.get(workspace);
        let usage = self.usage.get(workspace);

        match (quota, usage) {
            (Some(q), Some(u)) => self.enforcer.enforce_increment(q, u, resource, increment),
            (Some(q), None) => self.enforcer.enforce_increment(q, &ResourceUsage::new(), resource, increment),
            (None, _) => Ok(()),
        }
    }

    pub fn record_usage(
        &mut self,
        workspace: WorkspaceId,
        resource: ResourceType,
        amount: f64,
    ) -> Result<(), AppError> {
        let quota = self.quotas.get(&workspace);
        let current_usage = self.usage.get(&workspace).cloned().unwrap_or_default();

        if let Some(q) = quota {
            self.enforcer.enforce_increment(q, &current_usage, resource, amount)?;
        }

        let usage = self.get_or_create_usage(workspace);
        usage.add(resource, amount);

        Ok(())
    }

    pub fn reset_usage(&mut self, workspace: &WorkspaceId) {
        if let Some(usage) = self.usage.get_mut(workspace) {
            *usage = ResourceUsage::new();
        }
    }

    pub fn clear_usage(&mut self, workspace: &WorkspaceId) {
        self.usage.remove(workspace);
    }

    pub fn clear_quota(&mut self, workspace: &WorkspaceId) {
        self.quotas.remove(workspace);
    }

    pub fn get_all_quotas(&self) -> &HashMap<WorkspaceId, ResourceQuota> {
        &self.quotas
    }

    pub fn get_all_usage(&self) -> &HashMap<WorkspaceId, ResourceUsage> {
        &self.usage
    }

    pub fn monitor(&self) -> &ResourceMonitor {
        &self.monitor
    }

    pub fn get_system_resource_usage(&self) -> Result<ResourceUsage, AppError> {
        self.monitor.get_current_usage()
    }

    pub fn check_system_resources(&self) -> Result<SystemInfo, AppError> {
        Ok(self.monitor.get_system_info())
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}