use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use super::types::*;
use super::openai::OpenAiProvider;
use super::ollama::OllamaProvider;
use super::agnes::AgnesProvider;
use crate::errors::AppError;
use crate::infra::data_dir::DataDir;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiSettings {
    pub models: Vec<AiModelConfig>,
    pub active_model_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub ai: AiSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ai: AiSettings {
                models: Vec::new(),
                active_model_id: None,
            },
        }
    }
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    active_model_id: Option<String>,
    model_configs: Vec<AiModelConfig>,
}

impl ProviderRegistry {
    pub fn new(data_dir: &DataDir) -> Self {
        let settings_path = data_dir.config_path();
        tracing::info!(path = %settings_path.display(), "Loading provider registry");
        let settings = Self::load_settings(&settings_path);
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        // Register ollama (always available)
        providers.insert("ollama".to_string(), Arc::new(OllamaProvider::new(None)));
        tracing::debug!("Ollama provider registered");

        // Register providers from saved model configs
        for model_config in &settings.ai.models {
            if model_config.api_key.is_empty() {
                continue;
            }
            match model_config.provider.as_str() {
                "openai" => {
                    if !providers.contains_key("openai") {
                        let base_url = if model_config.base_url.is_empty() {
                            None
                        } else {
                            Some(model_config.base_url.clone())
                        };
                        providers.insert("openai".to_string(), Arc::new(OpenAiProvider::new(
                            model_config.api_key.clone(),
                            base_url,
                        )));
                        tracing::info!("OpenAI provider registered from config");
                    }
                }
                "agnes" => {
                    if !providers.contains_key("agnes") {
                        let base_url = if model_config.base_url.is_empty() {
                            None
                        } else {
                            Some(model_config.base_url.clone())
                        };
                        providers.insert("agnes".to_string(), Arc::new(AgnesProvider::new(
                            model_config.api_key.clone(),
                            base_url,
                        )));
                        tracing::info!("Agnes provider registered from config");
                    }
                }
                _ => {
                    tracing::warn!(provider = %model_config.provider, "Unknown provider skipped");
                }
            }
        }

        // Register from env vars (override saved config)
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() && !providers.contains_key("openai") {
                let base_url = std::env::var("OPENAI_BASE_URL").ok();
                providers.insert("openai".to_string(), Arc::new(OpenAiProvider::new(api_key, base_url)));
                tracing::info!("OpenAI provider registered from env var");
            }
        }
        if let Ok(api_key) = std::env::var("AGNES_API_KEY") {
            if !api_key.is_empty() && !providers.contains_key("agnes") {
                let base_url = std::env::var("AGNES_BASE_URL").ok();
                providers.insert("agnes".to_string(), Arc::new(AgnesProvider::new(api_key, base_url)));
                tracing::info!("Agnes provider registered from env var");
            }
        }
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            if !api_key.is_empty() && !providers.contains_key("anthropic") {
                let base_url = std::env::var("ANTHROPIC_BASE_URL").ok();
                providers.insert("anthropic".to_string(), Arc::new(super::anthropic::AnthropicProvider::new(api_key, base_url)));
                tracing::info!("Anthropic provider registered from env var");
            }
        }
        if let Ok(api_key) = std::env::var("DEEPSEEK_API_KEY") {
            if !api_key.is_empty() && !providers.contains_key("deepseek") {
                providers.insert("deepseek".to_string(), Arc::new(OpenAiProvider::new(api_key, Some("https://api.deepseek.com".to_string()))));
                tracing::info!("DeepSeek provider registered from env var");
            }
        }

        tracing::info!(count = providers.len(), active_model = ?settings.ai.active_model_id, "Provider registry loaded");
        Self {
            providers,
            active_model_id: settings.ai.active_model_id,
            model_configs: settings.ai.models,
        }
    }

    fn load_settings(path: &Path) -> AppSettings {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str(&data) {
                return settings;
            }
        }
        AppSettings::default()
    }

    pub fn register(&mut self, name: String, provider: Arc<dyn Provider>) {
        self.providers.insert(name, provider);
    }

    pub fn register_openai(&mut self, api_key: String, base_url: Option<String>) {
        self.providers.insert("openai".to_string(), Arc::new(OpenAiProvider::new(api_key, base_url)));
    }

    pub fn register_agnes(&mut self, api_key: String, base_url: Option<String>) {
        self.providers.insert("agnes".to_string(), Arc::new(AgnesProvider::new(api_key, base_url)));
    }

    pub fn get(&self, name: &str) -> Result<Arc<dyn Provider>, AppError> {
        self.providers.get(name).cloned()
            .ok_or_else(|| AppError::not_found(format!("Provider '{}' not found", name)))
    }

    pub fn default(&self) -> Result<Arc<dyn Provider>, AppError> {
        // Find the provider for the active model
        if let Some(active_id) = &self.active_model_id {
            if let Some(config) = self.model_configs.iter().find(|m| m.id == *active_id) {
                return self.get(&config.provider);
            }
        }
        // Fallback to ollama
        self.get("ollama")
    }

    pub fn active_model_id(&self) -> Option<&str> {
        self.active_model_id.as_deref()
    }

    pub fn default_model(&self) -> &str {
        if let Some(config) = self.active_model_config() {
            &config.model
        } else {
            "llama3.1"
        }
    }

    pub fn active_model_config(&self) -> Option<&AiModelConfig> {
        let active_id = self.active_model_id.as_ref()?;
        self.model_configs.iter().find(|m| m.id == *active_id)
    }

    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        self.providers.iter().map(|(name, p)| ProviderInfo { name: name.clone(), models: p.models() }).collect()
    }

    pub fn all_models(&self) -> Vec<ModelInfo> {
        self.providers.values().flat_map(|p| p.models()).collect()
    }

    pub fn model_configs(&self) -> &[AiModelConfig] {
        &self.model_configs
    }

    pub fn get_model_config(&self, id: &str) -> Option<&AiModelConfig> {
        self.model_configs.iter().find(|m| m.id == id)
    }

    pub fn get_default_config(&self) -> Option<(String, String, String)> {
        let config = self.active_model_config()?;
        let _provider = self.providers.get(&config.provider)?;
        Some((config.api_key.clone(), config.base_url.clone(), config.model.clone()))
    }

    pub async fn test_connection(&self, provider_name: &str, api_key: &str, base_url: &str, _model: &str) -> Result<(), AppError> {
        use super::openai::OpenAiProvider;
        use super::ollama::OllamaProvider;
        use super::agnes::AgnesProvider;
        use super::anthropic::AnthropicProvider;
        use std::sync::Arc;

        let provider: Arc<dyn Provider> = match provider_name {
            "openai" => Arc::new(OpenAiProvider::new(
                api_key.to_string(),
                if base_url.is_empty() { None } else { Some(base_url.to_string()) },
            )),
            "ollama" => Arc::new(OllamaProvider::new(
                if base_url.is_empty() { None } else { Some(base_url.to_string()) },
            )),
            "agnes" => Arc::new(AgnesProvider::new(
                api_key.to_string(),
                if base_url.is_empty() { None } else { Some(base_url.to_string()) },
            )),
            "anthropic" => Arc::new(AnthropicProvider::new(
                api_key.to_string(),
                if base_url.is_empty() { None } else { Some(base_url.to_string()) },
            )),
            "deepseek" => Arc::new(OpenAiProvider::new(
                api_key.to_string(),
                Some(if base_url.is_empty() { "https://api.deepseek.com".to_string() } else { base_url.to_string() }),
            )),
            _ => return Err(AppError::bad_request(format!("Unknown provider: {}", provider_name))),
        };

        provider.test_connection().await
    }
}
