use std::path::PathBuf;
use std::collections::HashMap;

use rig::client::CompletionClient;

use crate::shared::error::AppError;
use crate::infrastructure::llm::registry::ProviderRegistry;

use super::types::{SubAgentRole, SubAgentResult};
use super::agent::SubAgent;

pub struct AgentOrchestrator {
    registry: ProviderRegistry,
    workspace_root: PathBuf,
    subagents: HashMap<SubAgentRole, SubAgent>,
}

impl AgentOrchestrator {
    pub fn new(registry: ProviderRegistry, workspace_root: PathBuf) -> Self {
        let mut subagents = HashMap::new();
        
        for role in [SubAgentRole::Researcher, SubAgentRole::Outliner, SubAgentRole::Critic] {
            subagents.insert(role, SubAgent::new(role, workspace_root.clone()));
        }

        Self {
            registry,
            workspace_root,
            subagents,
        }
    }

    pub async fn delegate_to_subagent(
        &self,
        role: SubAgentRole,
        task: &str,
        context: &str,
    ) -> Result<SubAgentResult, AppError> {
        let subagent = self.subagents.get(&role)
            .ok_or_else(|| AppError::internal(format!("Sub-agent {:?} not initialized", role)))?;

        let config = self.registry.active_model_config()
            .ok_or_else(|| AppError::model_not_found("active"))?;

        let provider = config.provider.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();
        let model = config.model.clone();

        match provider.to_lowercase().as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(&model);
                subagent.execute(builder, task, context, None).await
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(&model);
                subagent.execute(builder, task, context, None).await
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(&model);
                subagent.execute(builder, task, context, None).await
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();

                let builder = client.agent(&model);
                subagent.execute(builder, task, context, None).await
            }
            _ => Err(AppError::provider_not_found(&provider)),
        }
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }
}