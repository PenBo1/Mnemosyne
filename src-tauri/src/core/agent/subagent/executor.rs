use std::path::PathBuf;
use std::sync::Arc;

use rig::client::CompletionClient;

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
    /// 可选的 skill 行为 prompt(从 loop_engine::prompts::prompt_for 解析)。
    ///
    /// 当 Some 时,会拼接到 role system_prompt 之后,作为 sub-agent 的行为约束。
    /// 用于 loop-engineering 场景:audit-revise-loop 注入 "loop-verifier" / "minimal-fix" 等 skill。
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

    /// 设置 skill 行为 prompt(从 skill name 解析得到)。
    ///
    /// 若 skill_name 未知(prompt_for 返回 None),则保持 None,不影响 sub-agent 默认行为。
    pub fn with_skill_prompt(mut self, skill_name: &str) -> Self {
        self.skill_prompt = crate::core::agent::loop_engine::prompts::prompt_for(skill_name)
            .map(|s| s.to_string());
        self
    }

    /// 直接设置 skill prompt 文本(跳过 prompt_for 解析,用于测试或自定义 skill)。
    pub fn with_skill_prompt_text(mut self, prompt: String) -> Self {
        self.skill_prompt = Some(prompt);
        self
    }
}

pub struct ExecutionResult {
    // 用 Arc<SubAgentResult> 让缓存命中路径零深拷贝：cache.get() 返回 Arc，
    // 直接透传到 ExecutionResult，最后在 Tool 边界按需 deref。
    pub result: Arc<SubAgentResult>,
    pub cached: bool,
    pub duration_ms: u64,
}

pub struct SubAgentExecutor<'a> {
    registry: &'a ProviderRegistry,
    workspace_root: PathBuf,
    cache: Arc<SubAgentCache>,
    token_counter: Arc<TokenCounter>,
    /// 并发上限（用于 `execute_parallel` 的 `buffer_unordered`）。
    max_concurrent: usize,
    /// Hook 引擎（可选，None 时跳过 SubagentStart/SubagentStop hook 派发）。
    hook_engine: OptionalHookDispatcher,
}

impl<'a> SubAgentExecutor<'a> {
    pub fn new(
        registry: &'a ProviderRegistry,
        workspace_root: PathBuf,
        cache: Arc<SubAgentCache>,
        token_counter: Arc<TokenCounter>,
        max_concurrent: usize,
        hook_engine: OptionalHookDispatcher,
    ) -> Self {
        Self {
            registry,
            workspace_root,
            cache,
            token_counter,
            max_concurrent,
            hook_engine,
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

        let role_str = task.role.as_str();

        // ── SubagentStart hook ──
        let start_payload = subagent_start_payload(None, None, role_str, &task.task);
        try_dispatch(&self.hook_engine, &start_payload).await?;

        let timer = ExecutionTimer::start();

        let subagent = SubAgent::new(task.role, self.workspace_root.clone());
        let config = self.registry.active_model_config()
            .ok_or_else(|| AppError::model_not_found("active"))?;

        let provider = config.provider.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();
        let model = config.model.clone();
        let skill_prompt = task.skill_prompt.clone();

        let execute_result = self.create_agent_and_execute(
            &provider,
            &base_url,
            &api_key,
            &model,
            &subagent,
            &task.task,
            &task.context,
            skill_prompt.as_deref(),
        ).await;

        let duration_ms = timer.elapsed_ms();

        // ── SubagentStop hook ──
        // 无论成功或失败都派发（success 字段反映执行结果）。
        let success = execute_result.is_ok();
        let stop_payload = subagent_stop_payload(None, None, role_str, success, duration_ms);
        if let Err(e) = try_dispatch(&self.hook_engine, &stop_payload).await {
            tracing::warn!(error = %e, "SubagentStop hook dispatch failed (ignored)");
        }

        let result = execute_result?;

        // TODO(token-usage): rig 的 `Prompt` trait 仅返回 `String`，不暴露 token usage。
        // 当前 record 为 0 是"未知"而非"零消耗"——待 rig 提供 usage API 或改用
        // `agent.completion(...)` 取 `Usage` 后填充真实值。
        // 现阶段显式 warn 标注，避免被误读为"本次调用 0 token"。
        tracing::warn!(
            role = role_str,
            duration_ms,
            "Sub-agent token usage unavailable (rig Prompt trait does not expose Usage); recording 0 as 'unknown'"
        );
        self.token_counter.record(
            task.role,
            task.task.clone(),
            0,
            0,
            duration_ms,
        ).await;

        self.cache.set(task.role, &task.task, &task.context, result.clone()).await;

        Ok(ExecutionResult {
            result: Arc::new(result),
            cached: false,
            duration_ms,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_agent_and_execute(
        &self,
        provider: &str,
        base_url: &str,
        api_key: &str,
        model: &str,
        subagent: &SubAgent,
        task: &str,
        context: &str,
        skill_prompt: Option<&str>,
    ) -> Result<SubAgentResult, AppError> {
        match provider.to_lowercase().as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(model);
                subagent.execute(builder, task, context, skill_prompt).await
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(model);
                subagent.execute(builder, task, context, skill_prompt).await
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;

                let builder = client.agent(model);
                subagent.execute(builder, task, context, skill_prompt).await
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();

                let builder = client.agent(model);
                subagent.execute(builder, task, context, skill_prompt).await
            }
            _ => Err(AppError::provider_not_found(provider)),
        }
    }

    /// 并行执行多个 sub-agent task。
    ///
    /// 使用 `futures::stream::iter(...).buffer_unordered(max_concurrent)` 控制并发上限，
    /// 而非顺序 await。结果顺序与完成顺序一致（不保证与输入顺序一致），
    /// 上层若需保持输入顺序应自行按 `task.role` + index 重排。
    ///
    /// 注：`buffer_unordered` 在内部用 `FuturesUnordered`，单 task 失败不影响其他 task。
    pub async fn execute_parallel(
        &self,
        tasks: Vec<SubAgentTask>,
    ) -> Vec<Result<ExecutionResult, AppError>> {
        use futures_util::stream::{self, StreamExt};

        let max_concurrent = self.max_concurrent.max(1);
        let results: Vec<Result<ExecutionResult, AppError>> = stream::iter(tasks)
            .map(|task| self.execute(task))
            .buffer_unordered(max_concurrent)
            .collect()
            .await;
        results
    }

    pub fn cache(&self) -> &Arc<SubAgentCache> {
        &self.cache
    }

    pub fn token_counter(&self) -> &Arc<TokenCounter> {
        &self.token_counter
    }
}