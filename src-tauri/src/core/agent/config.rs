use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Agent configuration format (P14 Agent Config Format)
/// Each agent has a structured JSON configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent: AgentMetadata,
    pub prompt: PromptConfig,
    pub tools: ToolsConfig,
    pub context: ContextConfig,
    pub output: OutputConfig,
    pub constraints: ConstraintsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub role: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub template_file: Option<String>,
    pub inline_template: Option<String>,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub token_budget: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub validation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintsConfig {
    pub must_do: Vec<String>,
    pub must_not_do: Vec<String>,
    pub style_rules: Option<Vec<String>>,
}

impl AgentConfig {
    /// Load configuration from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get prompt template, either from file or inline
    pub fn get_prompt_template(&self, base_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(ref template_file) = self.prompt.template_file {
            let path = base_dir.join(template_file);
            Ok(std::fs::read_to_string(path)?)
        } else if let Some(ref inline) = self.prompt.inline_template {
            Ok(inline.clone())
        } else {
            Err("No prompt template specified".into())
        }
    }

    /// Check if a tool is allowed for this agent
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if self.tools.denied.iter().any(|d| d == tool_name) {
            return false;
        }
        self.tools.allowed.is_empty() || self.tools.allowed.iter().any(|a| a == tool_name)
    }
}
