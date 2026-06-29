//! 生效 LLM 配置解析。

/// 从多个来源解析生效的 LLM 配置
pub fn resolve_effective_llm_config(
    env_config: &HashMap<String, String>,
    saved_config: Option<&LlmSavedConfig>,
    cli_overrides: Option<&LlmCliOverrides>,
) -> ResolvedLlmConfig {
    let mut config = ResolvedLlmConfig::default();

    // 应用已保存的配置
    if let Some(saved) = saved_config {
        config.provider = saved.provider.clone().unwrap_or_default();
        config.model = saved.model.clone().unwrap_or_default();
        config.api_key = saved.api_key.clone().unwrap_or_default();
        config.base_url = saved.base_url.clone().unwrap_or_default();
    }

    // 应用环境变量覆盖
    if let Some(key) = env_config.get("OPENAI_API_KEY") {
        config.api_key = key.clone();
    }
    if let Some(url) = env_config.get("OPENAI_BASE_URL") {
        config.base_url = url.clone();
    }

    // 应用 CLI 覆盖（优先级最高）
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
