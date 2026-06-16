//! LLM environment variable management.

use std::collections::HashMap;

/// Load environment variables for LLM configuration
pub fn load_llm_env() -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Check for common LLM env vars
    for key in &[
        "OPENAI_API_KEY", "OPENAI_BASE_URL",
        "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL",
        "DEEPSEEK_API_KEY",
        "MOONSHOT_API_KEY",
        "AGNES_API_KEY", "AGNES_BASE_URL",
        "TAVILY_API_KEY",
        "INKOS_AGENT_ALLOW_SYSTEM_READ",
        "LOG_FORMAT", "LOG_LEVEL",
    ] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                env.insert(key.to_string(), value);
            }
        }
    }

    env
}

/// Get a specific LLM environment variable
pub fn get_llm_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Merge multiple environment maps
pub fn merge_env_maps(base: &HashMap<String, String>, overlay: &HashMap<String, String>) -> HashMap<String, String> {
    let mut merged = base.clone();
    for (k, v) in overlay {
        merged.insert(k.clone(), v.clone());
    }
    merged
}
