//! ═══════════════════════════════════════════════════════════════════════════
//! enforcer - 资源配额执行器模块
//! ═══════════════════════════════════════════════════════════════════════════

use super::quota::{ResourceQuota, ResourceUsage, ResourceType};
use crate::shared::error::AppError;

pub struct ResourceEnforcer {
    _placeholder: (),
}

impl ResourceEnforcer {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    pub fn enforce_quota(
        &self,
        quota: &ResourceQuota,
        usage: &ResourceUsage,
    ) -> Result<(), AppError> {
        if let Some(exceeded) = usage.exceeds_quota(quota) {
            tracing::error!(
                operation = "enforce_quota",
                decision = "Deny",
                reason = "Resource quota exceeded",
                resource_type = exceeded.to_string(),
                "resource_enforcer: quota exceeded"
            );
            return Err(AppError::resource_quota_exceeded(exceeded.to_string()));
        }
        tracing::warn!(
            operation = "enforce_quota",
            decision = "Allow",
            "resource_enforcer: quota check passed"
        );
        Ok(())
    }

    pub fn enforce_resource(
        &self,
        quota: &ResourceQuota,
        resource: ResourceType,
        current: f64,
    ) -> Result<(), AppError> {
        let limit = match resource {
            ResourceType::Cpu => quota.cpu,
            ResourceType::Memory => quota.memory.map(|v| v as f64),
            ResourceType::Disk => quota.disk.map(|v| v as f64),
            ResourceType::Network => quota.network.map(|v| v as f64),
            ResourceType::Token => quota.token.map(|v| v as f64),
            ResourceType::Cost => quota.cost,
        };

        if let Some(limit_value) = limit {
            if current > limit_value {
                tracing::error!(
                    operation = "enforce_resource",
                    decision = "Deny",
                    reason = "Resource limit exceeded",
                    resource_type = resource.to_string(),
                    current = current,
                    limit = limit_value,
                    "resource_enforcer: resource limit exceeded"
                );
                return Err(AppError::resource_quota_exceeded(resource.to_string()));
            }
        }

        tracing::warn!(
            operation = "enforce_resource",
            decision = "Allow",
            resource_type = resource.to_string(),
            current = current,
            limit = limit,
            "resource_enforcer: resource check passed"
        );

        Ok(())
    }

    pub fn enforce_increment(
        &self,
        quota: &ResourceQuota,
        usage: &ResourceUsage,
        resource: ResourceType,
        increment: f64,
    ) -> Result<(), AppError> {
        let projected = match resource {
            ResourceType::Cpu => usage.cpu + increment,
            ResourceType::Memory => usage.memory as f64 + increment,
            ResourceType::Disk => usage.disk as f64 + increment,
            ResourceType::Network => usage.network as f64 + increment,
            ResourceType::Token => usage.token as f64 + increment,
            ResourceType::Cost => usage.cost + increment,
        };

        self.enforce_resource(quota, resource, projected)
    }
}

impl Default for ResourceEnforcer {
    fn default() -> Self {
        Self::new()
    }
}