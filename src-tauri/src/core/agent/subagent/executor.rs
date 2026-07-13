use std::path::PathBuf;
use std::sync::Arc;

use rig::client::CompletionClient;

use crate::shared::error::AppError;
use crate::infrastructure::llm::registry::ProviderRegistry;

use super::types::{SubAgentRole, SubAgentResult};
use super::agent::SubAgent;
use super::cache::SubAgentCache;
use super::tokens::{TokenCounter, ExecutionTimer};

pub struct SubAgentTask {
    pub role: SubAgentRole,
    pub task: String,
    pub context: String,
}

impl SubAgentTask {
    pub fn new(role: SubAgentRole, task: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            role,
            task: task.into(),
            context: context.into(),
        }
    }
}

pub struct ExecutionResult {
    pub result: SubAgentResult,
    pub cached: bool,
    pub duration_ms: u64,
}

pub struct SubAgentExecutor<'a> {
    registry: &'a ProviderRegistry,
    workspace_root: PathBuf,
    cache: Arc<SubAgentCache>,
    token_counter: Arc<TokenCounter>,
    #[allow(dead_code)]
    max_concurrent: usize,
}

impl<'a> SubAgentExecutor<'a> {
    pub fn new(
        registry: &'a ProviderRegistry,
        workspace_root: PathBuf,
        cache: Arc<SubAgentCache>,
        token_counter: Arc<TokenCounter>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            registry,
            workspace_root,
            cache,
            token_counter,
            max_concurrent,
        }
    }

    pub async fn execute(&self, task: SubAgentTask) -> Result<ExecutionResult, AppError> {
        if let Some(cached_result) = self.cache.get(task.role, &task.task, &task.context).await {
            return Ok(ExecutionResult {
                result: cached_result,
                cached: true,
                duration_ms: 0,
            });
        }

        let timer = ExecutionTimer::start();

        let subagent = SubAgent::new(task.role, self.workspace_root.clone());
        let config = self.registry.active_model_config()
            .ok_or_else(|| AppError::model_not_found("active"))?;

        let provider = config.provider.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();
        let model = config.model.clone();

        let result = self.create_agent_and_execute(
            &provider,
            &base_url,
            &api_key,
            &model,
            &subagent,
            &task.task,
            &task.context,
        ).await?;

        let duration_ms = timer.elapsed_ms();

        self.token_counter.record(
            task.role,
            task.task.clone(),
            0,
            0,
            duration_ms,
        ).await;

        self.cache.set(task.role, &task.task, &task.context, result.clone()).await;

        Ok(ExecutionResult {
            result,
            cached: false,
            duration_ms,
        })
    }

    async fn create_agent_and_execute(
        &self,
        provider: &str,
        base_url: &str,
        api_key: &str,
        model: &str,
        subagent: &SubAgent,
        task: &str,
        context: &str,
    ) -> Result<SubAgentResult, AppError> {
        match provider.to_lowercase().as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(model);
                subagent.execute(builder, task, context).await
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(model);
                subagent.execute(builder, task, context).await
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(model);
                subagent.execute(builder, task, context).await
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();

                let builder = client.agent(model);
                subagent.execute(builder, task, context).await
            }
            _ => Err(AppError::provider_not_found(provider)),
        }
    }

    pub async fn execute_parallel(
        &self,
        tasks: Vec<SubAgentTask>,
    ) -> Vec<Result<ExecutionResult, AppError>> {
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            results.push(self.execute(task).await);
        }
        results
    }

    pub fn cache(&self) -> &Arc<SubAgentCache> {
        &self.cache
    }

    pub fn token_counter(&self) -> &Arc<TokenCounter> {
        &self.token_counter
    }
}