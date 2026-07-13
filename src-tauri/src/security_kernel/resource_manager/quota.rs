use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
    Network,
    Token,
    Cost,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Cpu => write!(f, "cpu"),
            ResourceType::Memory => write!(f, "memory"),
            ResourceType::Disk => write!(f, "disk"),
            ResourceType::Network => write!(f, "network"),
            ResourceType::Token => write!(f, "token"),
            ResourceType::Cost => write!(f, "cost"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub cpu: Option<f64>,
    pub memory: Option<u64>,
    pub disk: Option<u64>,
    pub network: Option<u64>,
    pub token: Option<u64>,
    pub cost: Option<f64>,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu: None,
            memory: None,
            disk: None,
            network: None,
            token: Some(1_000_000),
            cost: Some(10.0),
        }
    }
}

impl ResourceQuota {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_token_limit(token: u64) -> Self {
        Self {
            cpu: None,
            memory: None,
            disk: None,
            network: None,
            token: Some(token),
            cost: Some(10.0),
        }
    }

    pub fn with_cost_limit(cost: f64) -> Self {
        Self {
            cpu: None,
            memory: None,
            disk: None,
            network: None,
            token: Some(1_000_000),
            cost: Some(cost),
        }
    }

    pub fn unlimited() -> Self {
        Self {
            cpu: None,
            memory: None,
            disk: None,
            network: None,
            token: None,
            cost: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    pub cpu: f64,
    pub memory: u64,
    pub disk: u64,
    pub network: u64,
    pub token: u64,
    pub cost: f64,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, resource: ResourceType, amount: f64) {
        match resource {
            ResourceType::Cpu => self.cpu += amount,
            ResourceType::Memory => self.memory += amount as u64,
            ResourceType::Disk => self.disk += amount as u64,
            ResourceType::Network => self.network += amount as u64,
            ResourceType::Token => self.token += amount as u64,
            ResourceType::Cost => self.cost += amount,
        }
    }

    pub fn exceeds_quota(&self, quota: &ResourceQuota) -> Option<ResourceType> {
        if let Some(cpu_limit) = quota.cpu {
            if self.cpu > cpu_limit {
                return Some(ResourceType::Cpu);
            }
        }

        if let Some(memory_limit) = quota.memory {
            if self.memory > memory_limit {
                return Some(ResourceType::Memory);
            }
        }

        if let Some(disk_limit) = quota.disk {
            if self.disk > disk_limit {
                return Some(ResourceType::Disk);
            }
        }

        if let Some(network_limit) = quota.network {
            if self.network > network_limit {
                return Some(ResourceType::Network);
            }
        }

        if let Some(token_limit) = quota.token {
            if self.token > token_limit {
                return Some(ResourceType::Token);
            }
        }

        if let Some(cost_limit) = quota.cost {
            if self.cost > cost_limit {
                return Some(ResourceType::Cost);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::Cpu.to_string(), "cpu");
        assert_eq!(ResourceType::Memory.to_string(), "memory");
        assert_eq!(ResourceType::Disk.to_string(), "disk");
        assert_eq!(ResourceType::Network.to_string(), "network");
        assert_eq!(ResourceType::Token.to_string(), "token");
        assert_eq!(ResourceType::Cost.to_string(), "cost");
    }

    #[test]
    fn test_resource_quota_default() {
        let quota = ResourceQuota::default();
        assert!(quota.cpu.is_none());
        assert!(quota.memory.is_none());
        assert!(quota.disk.is_none());
        assert!(quota.network.is_none());
        assert_eq!(quota.token, Some(1_000_000));
        assert_eq!(quota.cost, Some(10.0));
    }

    #[test]
    fn test_resource_quota_with_token_limit() {
        let quota = ResourceQuota::with_token_limit(500_000);
        assert_eq!(quota.token, Some(500_000));
        assert_eq!(quota.cost, Some(10.0));
    }

    #[test]
    fn test_resource_quota_with_cost_limit() {
        let quota = ResourceQuota::with_cost_limit(5.0);
        assert_eq!(quota.token, Some(1_000_000));
        assert_eq!(quota.cost, Some(5.0));
    }

    #[test]
    fn test_resource_quota_unlimited() {
        let quota = ResourceQuota::unlimited();
        assert!(quota.cpu.is_none());
        assert!(quota.memory.is_none());
        assert!(quota.disk.is_none());
        assert!(quota.network.is_none());
        assert!(quota.token.is_none());
        assert!(quota.cost.is_none());
    }

    #[test]
    fn test_resource_usage_new() {
        let usage = ResourceUsage::new();
        assert_eq!(usage.cpu, 0.0);
        assert_eq!(usage.memory, 0);
        assert_eq!(usage.disk, 0);
        assert_eq!(usage.network, 0);
        assert_eq!(usage.token, 0);
        assert_eq!(usage.cost, 0.0);
    }

    #[test]
    fn test_resource_usage_add() {
        let mut usage = ResourceUsage::new();
        usage.add(ResourceType::Cpu, 10.5);
        assert_eq!(usage.cpu, 10.5);

        usage.add(ResourceType::Memory, 1000.0);
        assert_eq!(usage.memory, 1000);

        usage.add(ResourceType::Token, 500.0);
        assert_eq!(usage.token, 500);

        usage.add(ResourceType::Cost, 2.5);
        assert_eq!(usage.cost, 2.5);
    }

    #[test]
    fn test_resource_usage_exceeds_quota_token() {
        let quota = ResourceQuota::with_token_limit(1000);
        let mut usage = ResourceUsage::new();
        usage.add(ResourceType::Token, 500.0);
        assert!(usage.exceeds_quota(&quota).is_none());

        usage.add(ResourceType::Token, 600.0);
        let exceeded = usage.exceeds_quota(&quota);
        assert_eq!(exceeded, Some(ResourceType::Token));
    }

    #[test]
    fn test_resource_usage_exceeds_quota_cost() {
        let quota = ResourceQuota::with_cost_limit(5.0);
        let mut usage = ResourceUsage::new();
        usage.add(ResourceType::Cost, 3.0);
        assert!(usage.exceeds_quota(&quota).is_none());

        usage.add(ResourceType::Cost, 3.0);
        let exceeded = usage.exceeds_quota(&quota);
        assert_eq!(exceeded, Some(ResourceType::Cost));
    }

    #[test]
    fn test_resource_usage_exceeds_quota_unlimited() {
        let quota = ResourceQuota::unlimited();
        let mut usage = ResourceUsage::new();
        usage.add(ResourceType::Token, 1_000_000_000.0);
        usage.add(ResourceType::Cost, 1_000_000.0);
        assert!(usage.exceeds_quota(&quota).is_none());
    }

    #[test]
    fn test_resource_usage_exceeds_quota_multiple_limits() {
        let quota = ResourceQuota {
            cpu: Some(50.0),
            memory: Some(1000),
            disk: None,
            network: None,
            token: Some(10000),
            cost: Some(5.0),
        };

        let mut usage = ResourceUsage::new();
        usage.add(ResourceType::Cpu, 30.0);
        usage.add(ResourceType::Memory, 500.0);
        usage.add(ResourceType::Token, 5000.0);
        usage.add(ResourceType::Cost, 3.0);
        assert!(usage.exceeds_quota(&quota).is_none());

        usage.add(ResourceType::Cpu, 30.0);
        let exceeded = usage.exceeds_quota(&quota);
        assert_eq!(exceeded, Some(ResourceType::Cpu));
    }

    #[test]
    fn test_resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.cpu, 0.0);
        assert_eq!(usage.memory, 0);
        assert_eq!(usage.disk, 0);
        assert_eq!(usage.network, 0);
        assert_eq!(usage.token, 0);
        assert_eq!(usage.cost, 0.0);
    }
}