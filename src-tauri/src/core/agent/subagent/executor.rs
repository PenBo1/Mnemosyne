//! ═══════════════════════════════════════════════════════════════════════════
//! SubAgentExecutor - 子代理执行器
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use crate::shared::error::AppError;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::security_kernel::hooks::{
    OptionalHookDispatcher, try_dispatch,
    subagent_start_payload, subagent_stop_payload,
};

use super::types::{SubAgentRole, SubAgentResult};
use super::agent::SubAgent;
use super::cache::SubAgentCache;
use super::tokens::{TokenCounter, ExecutionTimer};

pub struct SubAgentTask {
    pub role: SubAgentRole,
    pub task: String,
    pub context: String,
    pub skill_prompt: Option<String>,
}

impl SubAgentTask {
    pub fn new(role: SubAgentRole, task: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            role,
            task: task.into(),
            context: context.into(),
            skill_prompt: None,
        }
    }

    pub fn with_skill_prompt(mut self, skill_name: &str) -> Self {
        self.skill_prompt = crate::core::agent::loop_engine::prompts::prompt_for(skill_name)
            .map(|s| s.to_string());
        self
    }

    pub fn with_skill_prompt_text(mut self, prompt: String) -> Self {
        self.skill_prompt = Some(prompt);
        self
    }
}

pub struct ExecutionResult {
    pub result: Arc<SubAgentResult>,
    pub cached: bool,
    pub duration_ms: u64,
}

pub struct SubAgentExecutor<'a> {
    registry: &'a ProviderRegistry,
    cache: Arc<SubAgentCache>,
    token_counter: Arc<TokenCounter>,
    max_concurrent: usize,
    hook_engine: OptionalHookDispatcher,
}

impl<'a> SubAgentExecutor<'a> {
    pub fn new(
        registry: &'a ProviderRegistry,
        cache: Arc<SubAgentCache>,
        token_counter: Arc<TokenCounter>,
        max_concurrent: usize,
        hook_engine: OptionalHookDispatcher,
    ) -> Self {
        Self {
            registry,
            cache,
            token_counter,
            max_concurrent,
            hook_engine,
        }
    }

    pub async fn execute(&self, task: SubAgentTask) -> Result<ExecutionResult, AppError> {
        let task_preview = if task.task.len() > 100 {
            format!("{}...", &task.task[..100])
        } else {
            task.task.clone()
        };

        tracing::info!(
            role = %task.role.as_str(),
            task_preview = %task_preview,
            has_skill_prompt = task.skill_prompt.is_some(),
            "[subagent] execute: starting task"
        );

        if let Some(cached_result) = self.cache.get(task.role, &task.task, &task.context).await {
            tracing::info!(
                role = %task.role.as_str(),
                "[subagent] execute: cache hit, returning cached result"
            );
            return Ok(ExecutionResult {
                result: cached_result,
                cached: true,
                duration_ms: 0,
            });
        }

        tracing::debug!(
            role = %task.role.as_str(),
            "[subagent] execute: cache miss, proceeding with execution"
        );

        let role_str = task.role.as_str();

        tracing::debug!(
            role = %role_str,
            "[subagent] dispatching SubagentStart hook"
        );
        let start_payload = subagent_start_payload(None, None, role_str, &task.task);
        try_dispatch(&self.hook_engine, &start_payload).await?;

        let timer = ExecutionTimer::start();

        let subagent = SubAgent::new(task.role);
        let config = self.registry.active_model_config()
            .ok_or_else(AppError::no_active_model)?;

        let provider_name = config.provider.clone();
        let model = config.model.clone();
        let provider = self.registry.active_provider()?;
        let skill_prompt = task.skill_prompt.clone();

        tracing::debug!(
            role = %role_str,
            provider = %provider_name,
            model = %model,
            "[subagent] model config resolved"
        );

        let execute_result = subagent
            .execute(&provider, &model, &task.task, &task.context, skill_prompt.as_deref())
            .await;

        let duration_ms = timer.elapsed_ms();

        let success = execute_result.is_ok();
        tracing::debug!(
            role = %role_str,
            success = success,
            duration_ms,
            "[subagent] dispatching SubagentStop hook"
        );
        let stop_payload = subagent_stop_payload(None, None, role_str, success, duration_ms);
        if let Err(e) = try_dispatch(&self.hook_engine, &stop_payload).await {
            tracing::warn!(error = %e, "[subagent] SubagentStop hook dispatch failed (ignored)");
        }

        let result = execute_result?;

        tracing::debug!(
            role = %role_str,
            duration_ms,
            "[subagent] recording token usage"
        );
        self.token_counter.record(
            task.role,
            task.task.clone(),
            0,
            0,
            duration_ms,
        ).await;

        tracing::debug!(
            role = %role_str,
            "[subagent] caching result"
        );
        self.cache.set(task.role, &task.task, &task.context, result.clone()).await;

        tracing::info!(
            role = %role_str,
            duration_ms,
            cached = false,
            "[subagent] execute: completed"
        );

        Ok(ExecutionResult {
            result: Arc::new(result),
            cached: false,
            duration_ms,
        })
    }

    pub async fn execute_parallel(
        &self,
        tasks: Vec<SubAgentTask>,
    ) -> Vec<Result<ExecutionResult, AppError>> {
        use futures_util::stream::{self, StreamExt};

        tracing::info!(
            task_count = tasks.len(),
            max_concurrent = self.max_concurrent,
            "[subagent] execute_parallel: starting parallel execution"
        );

        let max_concurrent = self.max_concurrent.max(1);
        let results: Vec<Result<ExecutionResult, AppError>> = stream::iter(tasks)
            .map(|task| self.execute(task))
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        tracing::info!(
            total = results.len(),
            success = success_count,
            failed = results.len() - success_count,
            "[subagent] execute_parallel: completed"
        );

        results
    }

    pub fn cache(&self) -> &Arc<SubAgentCache> {
        &self.cache
    }

    pub fn token_counter(&self) -> &Arc<TokenCounter> {
        &self.token_counter
    }
}
