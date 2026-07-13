
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use super::types::*;
use super::openai::OpenAiProvider;
use super::ollama::OllamaProvider;
use super::agnes::AgnesProvider;
use crate::shared::error::AppError;
use crate::infrastructure::fs::data_dir::DataDir;

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

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    active_model_id: Option<String>,
    model_configs: Vec<AiModelConfig>,
    config_path: PathBuf,
}

impl ProviderRegistry {
    /// Create an empty registry (no providers loaded).
    pub fn empty() -> Self {
        Self {
            providers: HashMap::new(),
            active_model_id: None,
            model_configs: Vec::new(),
            config_path: PathBuf::new(),
        }
    }

    pub fn new(data_dir: &DataDir) -> Self {
        let settings_path = data_dir.config_path();
        tracing::info!(path = %settings_path.display(), "Loading provider registry");
        let settings = Self::load_settings(&settings_path);
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        providers.insert("ollama".to_string(), Arc::new(OllamaProvider::new(None)));
        tracing::debug!("Ollama provider registered");

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
            config_path: settings_path,
        }
    }

    fn load_settings(path: &PathBuf) -> AppSettings {
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
        if let Some(active_id) = &self.active_model_id {
            if let Some(config) = self.model_configs.iter().find(|m| m.id == *active_id) {
                return self.get(&config.provider);
            }
        }
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

    pub fn get_provider_for_model(&self, model_id: &str) -> Result<Arc<dyn Provider>, AppError> {
        let config = self.get_model_config(model_id)
            .ok_or_else(|| AppError::not_found(format!("Model '{}' not found in model_configs", model_id)))?;
        self.get(&config.provider)
    }

    pub fn save_settings(&self) -> Result<(), AppError> {
        let settings = AppSettings {
            ai: AiSettings {
                models: self.model_configs.clone(),
                active_model_id: self.active_model_id.clone(),
            },
        };
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| AppError::internal(format!("Failed to serialize settings: {}", e)))?;
        std::fs::write(&self.config_path, json)
            .map_err(|e| AppError::internal(format!(
                "Failed to write config to {}: {}",
                self.config_path.display(), e
            )))?;
        tracing::debug!(path = %self.config_path.display(), "Settings persisted");
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::fs::data_dir::DataDir;

    fn build_test_registry() -> (ProviderRegistry, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let settings = AppSettings {
            ai: AiSettings {
                models: vec![
                    AiModelConfig {
                        id: "model-openai".into(),
                        name: "GPT-4o".into(),
                        provider: "openai".into(),
                        model: "gpt-4o".into(),
                        api_key: "sk-test".into(),
                        base_url: String::new(),
                    },
                    AiModelConfig {
                        id: "model-ollama".into(),
                        name: "Llama 3.1".into(),
                        provider: "ollama".into(),
                        model: "llama3.1".into(),
                        api_key: String::new(),
                        base_url: String::new(),
                    },
                ],
                active_model_id: Some("model-openai".into()),
            },
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        std::fs::write(data_dir.config_path(), json).unwrap();

        let registry = ProviderRegistry::new(&data_dir);
        (registry, tmp)
    }

    #[test]
    fn ai_settings_deserializes_without_optional_fields() {
        let json = r#"{"ai":{"models":[],"active_model_id":null}}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        assert!(settings.ai.models.is_empty());
        assert!(settings.ai.active_model_id.is_none());
    }

    #[test]
    fn get_provider_for_model_returns_provider() {
        let (registry, _tmp) = build_test_registry();
        let provider = registry.get_provider_for_model("model-openai");
        assert!(provider.is_ok(), "Should find provider for model-openai");
    }

    #[test]
    fn get_provider_for_model_errors_on_unknown_id() {
        let (registry, _tmp) = build_test_registry();
        let result = registry.get_provider_for_model("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn save_settings_preserves_models_and_active_id() {
        let (registry, tmp) = build_test_registry();

        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let reloaded = ProviderRegistry::new(&data_dir);

        assert_eq!(reloaded.model_configs().len(), 2);
        assert_eq!(reloaded.active_model_id(), Some("model-openai"));
    }
}