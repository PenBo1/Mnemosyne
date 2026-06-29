//! Effective LLM config resolution.

/// Resolve effective LLM configuration from multiple sources
pub fn resolve_effective_llm_config(
    env_config: &HashMap<String, String>,
    saved_config: Option<&LlmSavedConfig>,
    cli_overrides: Option<&LlmCliOverrides>,
) -> ResolvedLlmConfig {
    let mut config = ResolvedLlmConfig::default();

    // Apply saved config
    if let Some(saved) = saved_config {
        config.provider = saved.provider.clone().unwrap_or_default();
        config.model = saved.model.clone().unwrap_or_default();
        config.api_key = saved.api_key.clone().unwrap_or_default();
        config.base_url = saved.base_url.clone().unwrap_or_default();
    }

    // Apply env overrides
    if let Some(key) = env_config.get("OPENAI_API_KEY") {
        config.api_key = key.clone();
    }
    if let Some(url) = env_config.get("OPENAI_BASE_URL") {
        config.base_url = url.clone();
    }

    // Apply CLI overrides (highest priority)
    if let Some(cli) = cli_overrides {
        if let Some(ref p) = cli.provider { config.provider = p.clone(); }
        if let Some(ref m) = cli.model { config.model = m.clone(); }
        if let Some(ref k) = cli.api_key { config.api_key = k.clone(); }
        if let Some(ref u) = cli.base_url { config.base_url = u.clone(); }
    }

    config
}

#[derive(Debug, Clone, Default)]
pub struct LlmSavedConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LlmCliOverrides {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedLlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
}

use std::collections::HashMap;
