use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use super::types::*;
use super::openai::OpenAiProvider;
use super::ollama::OllamaProvider;
use super::agnes::AgnesProvider;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;

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
    /// S9: per-agent model 路由 —— agent_name -> model_id（指向 `models` 中某条 AiModelConfig.id）
    #[serde(default)]
    pub agent_model_overrides: HashMap<String, String>,
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
                agent_model_overrides: HashMap::new(),
            },
        }
    }
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    active_model_id: Option<String>,
    model_configs: Vec<AiModelConfig>,
    /// S9: agent_name -> model_id 路由表（与 `model_configs` 中的 id 对应）
    agent_model_overrides: HashMap<String, String>,
    /// config.json 路径，用于持久化 agent_model_overrides
    config_path: PathBuf,
}

impl ProviderRegistry {
    pub fn new(data_dir: &DataDir) -> Self {
        let settings_path = data_dir.config_path();
        tracing::info!(path = %settings_path.display(), "Loading provider registry");
        let settings = Self::load_settings(&settings_path);
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        // 注册 ollama（始终可用）
        providers.insert("ollama".to_string(), Arc::new(OllamaProvider::new(None)));
        tracing::debug!("Ollama provider registered");

        // 从已保存的模型配置注册 provider
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

        // 从环境变量注册（覆盖已保存的配置）
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
            agent_model_overrides: settings.ai.agent_model_overrides,
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
        // 查找当前激活模型对应的 provider
        if let Some(active_id) = &self.active_model_id {
            if let Some(config) = self.model_configs.iter().find(|m| m.id == *active_id) {
                return self.get(&config.provider);
            }
        }
        // 回退到 ollama
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

    // ── S9: per-agent 模型路由 ─────────────────────────────────

    /// 只读访问 agent_model_overrides 路由表（agent_name -> model_id）。
    pub fn agent_model_overrides(&self) -> &HashMap<String, String> {
        &self.agent_model_overrides
    }

    /// 按 model_id 查找对应的 provider 实例。
    ///
    /// 查找链：model_id → AiModelConfig → config.provider name → providers map。
    /// 若 model_id 不存在或对应 provider 未注册，返回 `AppError::not_found`。
    pub fn get_provider_for_model(&self, model_id: &str) -> Result<Arc<dyn Provider>, AppError> {
        let config = self.get_model_config(model_id)
            .ok_or_else(|| AppError::not_found(format!("Model '{}' not found in model_configs", model_id)))?;
        self.get(&config.provider)
    }

    /// 构建完整的 per-agent 路由：返回 (model_overrides, agent_providers)。
    ///
    /// - `model_overrides`: agent_name -> model_name_string（用于 `provider.complete(&model, ...)`）
    /// - `agent_providers`: agent_name -> provider Arc（用于构造 AgentContext.provider）
    ///
    /// 对每条 (agent_name, model_id) 配置：
    /// - 若 model_id 在 `model_configs` 中存在，且对应 provider 已注册 → 写入两张表
    /// - 否则跳过并记录 warn（不阻断 pipeline 启动）
    pub fn build_agent_routing(
        &self,
    ) -> (HashMap<String, String>, HashMap<String, Arc<dyn Provider>>) {
        let mut model_overrides: HashMap<String, String> = HashMap::new();
        let mut agent_providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        for (agent_name, model_id) in &self.agent_model_overrides {
            match self.get_model_config(model_id) {
                Some(config) => {
                    match self.get(&config.provider) {
                        Ok(provider) => {
                            model_overrides.insert(agent_name.clone(), config.model.clone());
                            agent_providers.insert(agent_name.clone(), provider);
                        }
                        Err(e) => {
                            tracing::warn!(
                                agent = %agent_name,
                                model_id = %model_id,
                                provider = %config.provider,
                                error = %e,
                                "Provider not registered for agent override; skipping"
                            );
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        agent = %agent_name,
                        model_id = %model_id,
                        "Model id not found in model_configs; skipping agent override"
                    );
                }
            }
        }

        (model_overrides, agent_providers)
    }

    /// 设置（或清除）某个 agent 的 model 覆盖，并持久化到 config.json。
    ///
    /// - `model_id = Some(id)` → 设置覆盖（id 必须在 `model_configs` 中存在）
    /// - `model_id = None` → 清除覆盖
    pub fn set_agent_model_override(
        &mut self,
        agent_name: &str,
        model_id: Option<String>,
    ) -> Result<(), AppError> {
        if let Some(ref id) = model_id {
            if self.get_model_config(id).is_none() {
                return Err(AppError::bad_request(format!(
                    "Model id '{}' not found in configured models", id
                )));
            }
            self.agent_model_overrides.insert(agent_name.to_string(), id.clone());
            tracing::info!(agent = %agent_name, model_id = %id, "Agent model override set");
        } else {
            self.agent_model_overrides.remove(agent_name);
            tracing::info!(agent = %agent_name, "Agent model override cleared");
        }
        self.save_settings()
    }

    /// 将当前配置（含 agent_model_overrides）持久化到 config.json。
    pub fn save_settings(&self) -> Result<(), AppError> {
        let settings = AppSettings {
            ai: AiSettings {
                models: self.model_configs.clone(),
                active_model_id: self.active_model_id.clone(),
                agent_model_overrides: self.agent_model_overrides.clone(),
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

// ════════════════════════════════════════════════════════════════════
// S9 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::file_storage::data_dir::DataDir;

    /// 构造一个临时 registry，含两个 model_configs 和指定的 agent_model_overrides。
    fn build_test_registry(
        overrides: HashMap<String, String>,
    ) -> (ProviderRegistry, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        // 写入 config.json，含两个 model 配置
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
                agent_model_overrides: overrides,
            },
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        std::fs::write(data_dir.config_path(), json).unwrap();

        let registry = ProviderRegistry::new(&data_dir);
        (registry, tmp)
    }

    #[test]
    fn ai_settings_deserializes_without_agent_model_overrides() {
        // 旧版 config.json（无 agent_model_overrides 字段）应能正常反序列化
        let json = r#"{"ai":{"models":[],"active_model_id":null}}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        assert!(settings.ai.agent_model_overrides.is_empty());
    }

    #[test]
    fn build_agent_routing_empty_when_no_overrides() {
        let (registry, _tmp) = build_test_registry(HashMap::new());
        let (model_overrides, agent_providers) = registry.build_agent_routing();
        assert!(model_overrides.is_empty());
        assert!(agent_providers.is_empty());
    }

    #[test]
    fn build_agent_routing_populates_both_maps() {
        let mut overrides = HashMap::new();
        overrides.insert("writer".into(), "model-openai".into());
        overrides.insert("auditor".into(), "model-ollama".into());
        let (registry, _tmp) = build_test_registry(overrides);

        let (model_overrides, agent_providers) = registry.build_agent_routing();

        // writer → gpt-4o
        assert_eq!(model_overrides.get("writer").map(|s| s.as_str()), Some("gpt-4o"));
        // auditor → llama3.1
        assert_eq!(model_overrides.get("auditor").map(|s| s.as_str()), Some("llama3.1"));
        // 两个 agent 都有 provider
        assert!(agent_providers.contains_key("writer"));
        assert!(agent_providers.contains_key("auditor"));
    }

    #[test]
    fn build_agent_routing_skips_invalid_model_id() {
        let mut overrides = HashMap::new();
        overrides.insert("writer".into(), "model-openai".into());
        overrides.insert("auditor".into(), "nonexistent-model".into());
        let (registry, _tmp) = build_test_registry(overrides);

        let (model_overrides, agent_providers) = registry.build_agent_routing();

        // writer 正常填充
        assert_eq!(model_overrides.get("writer").map(|s| s.as_str()), Some("gpt-4o"));
        // auditor 因 model_id 不存在被跳过
        assert!(!model_overrides.contains_key("auditor"));
        assert!(!agent_providers.contains_key("auditor"));
    }

    #[test]
    fn get_provider_for_model_returns_provider() {
        let (registry, _tmp) = build_test_registry(HashMap::new());
        let provider = registry.get_provider_for_model("model-openai");
        assert!(provider.is_ok(), "Should find provider for model-openai");
    }

    #[test]
    fn get_provider_for_model_errors_on_unknown_id() {
        let (registry, _tmp) = build_test_registry(HashMap::new());
        let result = registry.get_provider_for_model("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn set_agent_model_override_rejects_unknown_model_id() {
        let (mut registry, _tmp) = build_test_registry(HashMap::new());
        let result = registry.set_agent_model_override("writer", Some("nonexistent".into()));
        assert!(result.is_err(), "Should reject unknown model_id");
    }

    #[test]
    fn set_agent_model_override_persists_and_updates() {
        let (mut registry, tmp) = build_test_registry(HashMap::new());

        // 设置覆盖
        registry.set_agent_model_override("writer", Some("model-ollama".into()))
            .expect("set override should succeed");

        // 内存中已更新
        assert_eq!(
            registry.agent_model_overrides().get("writer").map(|s| s.as_str()),
            Some("model-ollama")
        );

        // config.json 已持久化 —— 重新加载验证
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let reloaded = ProviderRegistry::new(&data_dir);
        assert_eq!(
            reloaded.agent_model_overrides().get("writer").map(|s| s.as_str()),
            Some("model-ollama")
        );
    }

    #[test]
    fn set_agent_model_override_clear_removes_entry() {
        let mut initial = HashMap::new();
        initial.insert("writer".into(), "model-openai".into());
        let (mut registry, _tmp) = build_test_registry(initial);

        // 清除覆盖
        registry.set_agent_model_override("writer", None)
            .expect("clear override should succeed");

        assert!(!registry.agent_model_overrides().contains_key("writer"));
    }

    #[test]
    fn save_settings_preserves_all_fields() {
        let (mut registry, tmp) = build_test_registry(HashMap::new());
        registry.set_agent_model_override("auditor", Some("model-ollama".into()))
            .expect("set override");

        // 重新加载验证所有字段
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let reloaded = ProviderRegistry::new(&data_dir);

        assert_eq!(reloaded.model_configs().len(), 2);
        assert_eq!(reloaded.active_model_id(), Some("model-openai"));
        assert_eq!(
            reloaded.agent_model_overrides().get("auditor").map(|s| s.as_str()),
            Some("model-ollama")
        );
    }
}
