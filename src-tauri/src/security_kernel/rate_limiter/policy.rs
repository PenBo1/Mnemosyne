//! ═══════════════════════════════════════════════════════════════════════════
//! policy - 速率限制策略模块
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatePolicy {
    pub operation: String,
    pub max_per_minute: u32,
    pub max_per_hour: u32,
    pub max_per_day: u32,
}

impl RatePolicy {
    pub fn new(
        operation: impl Into<String>,
        max_per_minute: u32,
        max_per_hour: u32,
        max_per_day: u32,
    ) -> Self {
        Self {
            operation: operation.into(),
            max_per_minute,
            max_per_hour,
            max_per_day,
        }
    }

    pub fn unlimited(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            max_per_minute: u32::MAX,
            max_per_hour: u32::MAX,
            max_per_day: u32::MAX,
        }
    }
}

impl Default for RatePolicy {
    fn default() -> Self {
        Self::unlimited("default")
    }
}

pub struct DefaultPolicies;

impl DefaultPolicies {
    pub fn read_file() -> RatePolicy {
        RatePolicy::new("ReadFile", 10000, 600_000, 10_000_000)
    }

    pub fn write_file() -> RatePolicy {
        RatePolicy::new("WriteFile", 1000, 60_000, 1_000_000)
    }

    pub fn delete_file() -> RatePolicy {
        RatePolicy::new("DeleteFile", 100, 6_000, 100_000)
    }

    pub fn all() -> Vec<RatePolicy> {
        vec![
            Self::read_file(),
            Self::write_file(),
            Self::delete_file(),
        ]
    }
}