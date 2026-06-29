use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Provider family — 决定 API 格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFamily {
    OpenAi,
    Anthropic,
}

/// Service preset — 已知 LLM provider 的预配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePreset {
    pub provider_family: ProviderFamily,
    pub api: String,
    pub base_url: String,
    pub label: String,
    pub temperature_range: (f64, f64),
    pub default_temperature: f64,
    pub writing_temperature: f64,
    pub temperature_hint: Option<String>,
    pub known_models: Vec<String>,
}

/// 来自 provider 库的模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: u32,
    pub max_output: Option<u32>,
}

/// 内置 service preset
pub fn service_presets() -> HashMap<&'static str, ServicePreset> {
    let mut m = HashMap::new();

    m.insert("openai", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-responses".into(),
        base_url: "https://api.openai.com/v1".into(),
        label: "OpenAI".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 1.0,
        writing_temperature: 1.0,
        temperature_hint: None,
        known_models: vec!["gpt-4o".into(), "gpt-4o-mini".into(), "gpt-4.1".into(), "o3".into()],
    });

    m.insert("anthropic", ServicePreset {
        provider_family: ProviderFamily::Anthropic,
        api: "anthropic-messages".into(),
        base_url: "https://api.anthropic.com".into(),
        label: "Anthropic".into(),
        temperature_range: (0.0, 1.0),
        default_temperature: 1.0,
        writing_temperature: 1.0,
        temperature_hint: Some("不要同时改 temperature 和 top_p".into()),
        known_models: vec!["claude-sonnet-4-20250514".into(), "claude-3-5-haiku-20241022".into()],
    });

    m.insert("deepseek", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "https://api.deepseek.com".into(),
        label: "DeepSeek".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 1.0,
        writing_temperature: 1.5,
        temperature_hint: Some("创意写作推荐 1.5".into()),
        known_models: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
    });

    m.insert("moonshot", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "https://api.moonshot.cn/v1".into(),
        label: "Moonshot (Kimi)".into(),
        temperature_range: (0.0, 1.0),
        default_temperature: 0.3,
        writing_temperature: 1.0,
        temperature_hint: Some("kimi-k2.5 推荐 temperature=1.0".into()),
        known_models: vec!["moonshot-v1-8k".into(), "moonshot-v1-32k".into(), "moonshot-v1-128k".into()],
    });

    m.insert("minimax", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "https://api.minimaxi.com/v1".into(),
        label: "MiniMax".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 0.9,
        writing_temperature: 0.9,
        temperature_hint: None,
        known_models: vec!["MiniMax-M2.7".into(), "MiniMax-M2.5".into()],
    });

    m.insert("bailian", ServicePreset {
        provider_family: ProviderFamily::Anthropic,
        api: "anthropic-messages".into(),
        base_url: "https://dashscope.aliyuncs.com/apps/anthropic".into(),
        label: "百炼 (通义千问)".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 0.7,
        writing_temperature: 1.0,
        temperature_hint: None,
        known_models: vec!["qwen-max".into(), "qwen-plus".into(), "qwen-turbo".into()],
    });

    m.insert("zhipu", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
        label: "智谱 GLM".into(),
        temperature_range: (0.0, 1.0),
        default_temperature: 0.95,
        writing_temperature: 0.95,
        temperature_hint: None,
        known_models: vec!["glm-4-plus".into(), "glm-4-flash".into()],
    });

    m.insert("siliconflow", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "https://api.siliconflow.cn/v1".into(),
        label: "硅基流动".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 0.7,
        writing_temperature: 1.0,
        temperature_hint: None,
        known_models: vec![],
    });

    m.insert("openrouter", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-responses".into(),
        base_url: "https://openrouter.ai/api/v1".into(),
        label: "OpenRouter".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 0.7,
        writing_temperature: 1.0,
        temperature_hint: None,
        known_models: vec![],
    });

    m.insert("ollama", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "http://localhost:11434/v1".into(),
        label: "Ollama (本地)".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 0.7,
        writing_temperature: 1.0,
        temperature_hint: None,
        known_models: vec![],
    });

    m.insert("custom", ServicePreset {
        provider_family: ProviderFamily::OpenAi,
        api: "openai-completions".into(),
        base_url: "".into(),
        label: "自定义端点".into(),
        temperature_range: (0.0, 2.0),
        default_temperature: 0.7,
        writing_temperature: 1.0,
        temperature_hint: None,
        known_models: vec![],
    });

    m
}

/// 按名称解析 service preset
pub fn resolve_service_preset(service: &str) -> Option<ServicePreset> {
    service_presets().get(service).cloned()
}

/// 根据 base URL 推测 service 名称
pub fn guess_service_from_base_url(base_url: &str) -> &'static str {
    for (key, preset) in service_presets() {
        if key == "custom" || preset.base_url.is_empty() {
            continue;
        }
        if let Ok(url) = url::Url::parse(&preset.base_url) {
            if let Some(host) = url.host_str() {
                if base_url.contains(host) {
                    return key;
                }
            }
        }
    }
    "custom"
}

/// Get known models for a service
pub fn get_known_models(service: &str) -> Vec<String> {
    resolve_service_preset(service)
        .map(|p| p.known_models)
        .unwrap_or_default()
}

/// Get writing temperature for a service
pub fn get_writing_temperature(service: &str) -> f64 {
    resolve_service_preset(service)
        .map(|p| p.writing_temperature)
        .unwrap_or(1.0)
}

/// 将 temperature 限制到 service 的有效范围
pub fn clamp_temperature(service: &str, temperature: f64) -> f64 {
    let (min, max) = resolve_service_preset(service)
        .map(|p| p.temperature_range)
        .unwrap_or((0.0, 2.0));
    temperature.max(min).min(max)
}
