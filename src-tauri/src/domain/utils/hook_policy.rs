//! Hook policy constants and defaults.

/// Default hook health thresholds
pub struct HookHealthDefaults;

impl HookHealthDefaults {
    pub const MAX_ACTIVE_HOOKS: u32 = 12;
    pub const STALE_AFTER_CHAPTERS: u32 = 10;
    pub const NO_ADVANCE_WINDOW: u32 = 5;
    pub const NEW_HOOK_BURST_THRESHOLD: u32 = 3;
}

/// Hook status categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookStatusCategory {
    Open,
    Pressured,
    Stale,
    Blocked,
    Resolved,
    Deferred,
}

impl HookStatusCategory {
    pub fn from_status(status: &str) -> Self {
        let lower = status.trim().to_lowercase();
        if lower == "open" || lower == "planted" {
            Self::Open
        } else if lower.contains("pressured") || lower.contains("near_payoff") {
            Self::Pressured
        } else if lower == "stale" || lower.contains("过期") {
            Self::Stale
        } else if lower.contains("blocked") || lower.contains("受阻") {
            Self::Blocked
        } else if lower == "resolved" || lower.contains("已回收") {
            Self::Resolved
        } else if lower == "deferred" || lower.contains("延后") {
            Self::Deferred
        } else {
            Self::Open
        }
    }
}
