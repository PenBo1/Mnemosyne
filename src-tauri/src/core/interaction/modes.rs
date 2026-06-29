use serde::{Deserialize, Serialize};

/// Automation mode for pipeline operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutomationMode {
    Auto,
    Semi,
    Manual,
}

impl Default for AutomationMode {
    fn default() -> Self {
        Self::Semi
    }
}

/// Normalize an automation mode string to a valid enum value
pub fn normalize_automation_mode(mode: Option<&str>, fallback: AutomationMode) -> AutomationMode {
    match mode {
        Some("auto") => AutomationMode::Auto,
        Some("semi") => AutomationMode::Semi,
        Some("manual") => AutomationMode::Manual,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_mode() {
        assert_eq!(normalize_automation_mode(Some("auto"), AutomationMode::Semi), AutomationMode::Auto);
        assert_eq!(normalize_automation_mode(Some("manual"), AutomationMode::Semi), AutomationMode::Manual);
        assert_eq!(normalize_automation_mode(None, AutomationMode::Semi), AutomationMode::Semi);
        assert_eq!(normalize_automation_mode(Some("invalid"), AutomationMode::Auto), AutomationMode::Auto);
    }
}
