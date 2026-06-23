use super::types::{RiskLevel, ConfirmationRequest};

/// Detects risk level of tool calls and generates confirmation requests
pub struct SafetyGate;

impl SafetyGate {
    /// Evaluate the risk level of a tool call
    pub fn evaluate_risk(tool_name: &str, args: &serde_json::Value) -> RiskLevel {
        match tool_name {
            // High risk: destructive operations
            "bash" => Self::evaluate_bash_risk(args),
            "write_file" => Self::evaluate_write_risk(args),

            // Moderate risk: memory modifications
            "archive_memory" => RiskLevel::Moderate,

            // Safe: read-only operations
            "read_file" | "list_files" | "search_memory" => RiskLevel::Safe,

            _ => RiskLevel::Moderate,
        }
    }

    fn evaluate_bash_risk(args: &serde_json::Value) -> RiskLevel {
        let command = args["command"].as_str().unwrap_or("");

        // High risk patterns
        let high_risk = ["rm ", "rmdir", "del ", "format ", "mkfs",
            "sudo", "chmod", "chown", "kill", "killall",
            "shutdown", "reboot", "init 0", "init 6",
            "git push --force", "git reset --hard", "git clean -f",
            "> /dev/", "dd if=", "wget", "curl.*|.*sh"];

        for pattern in &high_risk {
            if command.to_lowercase().contains(&pattern.to_lowercase()) {
                return RiskLevel::High;
            }
        }

        // Moderate risk: file modifications
        let moderate_risk = ["mv ", "cp ", "mkdir", "touch", "tee ",
            "echo >", "cat >", "git add", "git commit", "git rm",
            "cargo install", "npm install -g", "pip install"];

        for pattern in &moderate_risk {
            if command.to_lowercase().contains(&pattern.to_lowercase()) {
                return RiskLevel::Moderate;
            }
        }

        RiskLevel::Safe
    }

    fn evaluate_write_risk(args: &serde_json::Value) -> RiskLevel {
        let path = args["path"].as_str().unwrap_or("");

        // High risk: system files, config files
        let high_risk_paths = ["/etc/", "/usr/", "/bin/", "/sbin/",
            "C:\\Windows", "C:\\Program Files",
            ".ssh", ".env", "credentials", "secret", "token"];

        for pattern in &high_risk_paths {
            if path.to_lowercase().contains(&pattern.to_lowercase()) {
                return RiskLevel::High;
            }
        }

        // Moderate risk: any write operation
        RiskLevel::Moderate
    }

    /// Create a confirmation request for a risky step
    pub fn create_confirmation_request(
        step_id: u32,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> ConfirmationRequest {
        let risk_level = Self::evaluate_risk(tool_name, args);
        let description = format!("Execute: {}", tool_name);
        let details = Self::format_args(tool_name, args);

        ConfirmationRequest {
            step_id,
            description,
            details,
            risk_level,
        }
    }

    fn format_args(tool_name: &str, args: &serde_json::Value) -> String {
        match tool_name {
            "bash" => {
                let cmd = args["command"].as_str().unwrap_or("(unknown)");
                format!("Command: {}", cmd)
            }
            "write_file" => {
                let path = args["path"].as_str().unwrap_or("(unknown)");
                let content = args["content"].as_str().unwrap_or("");
                let preview = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.to_string()
                };
                format!("Path: {}\nContent preview:\n{}", path, preview)
            }
            "read_file" => {
                let path = args["path"].as_str().unwrap_or("(unknown)");
                format!("Read: {}", path)
            }
            "list_files" => {
                let path = args["path"].as_str().unwrap_or(".");
                format!("List: {}", path)
            }
            "search_memory" => {
                let query = args["query"].as_str().unwrap_or("(unknown)");
                format!("Search: {}", query)
            }
            "archive_memory" => {
                let content = args["content"].as_str().unwrap_or("(unknown)");
                format!("Archive: {}", content)
            }
            _ => format!("Args: {}", args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_safe_tools() {
        assert_eq!(SafetyGate::evaluate_risk("read_file", &json!({"path": "test.txt"})), RiskLevel::Safe);
        assert_eq!(SafetyGate::evaluate_risk("list_files", &json!({"path": "."})), RiskLevel::Safe);
        assert_eq!(SafetyGate::evaluate_risk("search_memory", &json!({"query": "test"})), RiskLevel::Safe);
    }

    #[test]
    fn test_high_risk_bash() {
        assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "rm -rf /"})), RiskLevel::High);
        assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "sudo apt install"})), RiskLevel::High);
        assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "git push --force"})), RiskLevel::High);
    }

    #[test]
    fn test_moderate_risk() {
        assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "git commit -m test"})), RiskLevel::Moderate);
        assert_eq!(SafetyGate::evaluate_risk("write_file", &json!({"path": "src/main.rs", "content": "test"})), RiskLevel::Moderate);
        assert_eq!(SafetyGate::evaluate_risk("archive_memory", &json!({"content": "test"})), RiskLevel::Moderate);
    }

    #[test]
    fn test_high_risk_write() {
        assert_eq!(SafetyGate::evaluate_risk("write_file", &json!({"path": "/etc/passwd", "content": "test"})), RiskLevel::High);
        assert_eq!(SafetyGate::evaluate_risk("write_file", &json!({"path": ".env", "content": "SECRET=abc"})), RiskLevel::High);
    }
}
