use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use rig::agent::{Agent, MultiTurnStreamItem, StreamingError};
use rig::client::CompletionClient;
use rig::completion::{GetTokenUsage, Prompt, Usage};
use rig::streaming::{StreamingPrompt, StreamedAssistantContent, StreamedUserContent};
use tokio::sync::mpsc;

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;

use crate::domain::user::store::UserProfileStore;
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::session::MessageMeta;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::infrastructure::telemetry::{SpanKind, Tracer};
use crate::security_kernel::hooks::{
    OptionalHookDispatcher, try_dispatch,
    session_start_payload, stop_payload, user_prompt_submit_payload,
};
use crate::shared::error::AppError;

use super::approval::ApprovalManager;
use super::collaboration_style::{merge_style_into_instructions, CollaborationStyle};
use super::effort::{EffortLevel, EffortParams};
use super::identity;
use super::prompts;
use super::subagent::{SubAgentCache, SubAgentExecutor, SubAgentTool, TokenCounter};
use super::tools::todo_tools::TodoWriteTool;
use super::tools::{
    CreateDirectoryTool, EditTool, ListDirectoryTool, MultiEditTool, ReadFileTool, WriteFileTool,
};
use super::tools::{
    GenerateCoverTool, GrepTool, ImportChaptersTool, IngestMaterialTool, LsTool,
    PatchChapterTextTool, PipelineDelegateTool, ProposeActionTool, RenameEntityTool,
    ReplaceChapterTextTool, ResearchWebTool, RetrieveMaterialTool, WriteTruthFileTool,
};
use super::types::{ChatEvent, ChatRequest};

/// 默认 Effort 级别(当 ChatRequest 未指定时使用)
const DEFAULT_EFFORT: EffortLevel = EffortLevel::Medium;
const MAX_CONCURRENT_SUBAGENTS: usize = 3;

#[derive(Clone)]
pub struct AgentEngine {
    registry: ProviderRegistry,
    db: Database,
    data_dir: DataDir,
    subagent_cache: Arc<SubAgentCache>,
    token_counter: Arc<TokenCounter>,
    /// Hook 引擎（可选，None 时跳过所有 hook 派发）。
    /// 从 SecurityKernel.hook_engine() 注入，用于在 send_message 前后派发
    /// SessionStart / UserPromptSubmit / Stop 等生命周期 hook。
    hook_engine: OptionalHookDispatcher,
}

/// 一次 agent 流式调用的聚合结果,用于持久化到 messages 表。
struct StreamOutcome {
    text: String,
    usage: Usage,
    tool_calls: Vec<serde_json::Value>,
    tool_results: Vec<serde_json::Value>,
}

impl AgentEngine {
    pub fn new(
        registry: ProviderRegistry,
        db: Database,
        data_dir: DataDir,
        _workspace_root: PathBuf,
        hook_engine: OptionalHookDispatcher,
    ) -> Self {
        let cache = Arc::new(SubAgentCache::new(100, 3600));
        let token_counter = Arc::new(TokenCounter::new(1000));

        Self {
            registry,
            db,
            data_dir,
            subagent_cache: cache,
            token_counter,
            hook_engine,
        }
    }

    /// 访问 hook 引擎（供 SubAgentExecutor 借用）。
    pub fn hook_engine(&self) -> &OptionalHookDispatcher {
        &self.hook_engine
    }

    pub fn subagent_executor(&self, workspace_root: PathBuf) -> SubAgentExecutor<'_> {
        SubAgentExecutor::new(
            &self.registry,
            workspace_root,
            Arc::clone(&self.subagent_cache),
            Arc::clone(&self.token_counter),
            MAX_CONCURRENT_SUBAGENTS,
            self.hook_engine.clone(),
        )
    }

    /// 返回一个 Tracer，用于在 agent 流程中创建 trace span。
    ///
    /// Tracer 持有 Database 的廉价克隆（Arc<Connection>），可自由创建。
    /// Span 实现 RAII Drop，自动写入 DB。
    pub fn tracer(&self) -> Tracer {
        Tracer::new(self.db.clone())
    }

    pub async fn send_message(
        &self,
        request: ChatRequest,
        workspace_root: PathBuf,
        approval: Arc<ApprovalManager>,
        tx: mpsc::Sender<ChatEvent>,
    ) -> Result<(), AppError> {
        // ── SessionStart hook ──
        // 在 agent 流程开始时派发。若 hook 返回 FailedAbort，则中止本次会话。
        let session_start = session_start_payload(&request.session_id, None, Some(prompts::MAIN_ROLE));
        try_dispatch(&self.hook_engine, &session_start).await?;

        // Trace span: 记录本次 agent 调用的耗时与上下文（RAII Drop 自动写入 DB）
        let tracer = self.tracer();
        let mut span = tracer.start_span("agent.send_message", SpanKind::Internal);
        span.set_session(&request.session_id);

        let config = self
            .registry
            .active_model_config()
            .ok_or_else(|| AppError::model_not_found("active"))?;

        let provider = config.provider.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();
        let model = config.model.clone();

        // 解析 Effort:请求级覆盖 > 全局默认
        let effort = request.effort.unwrap_or(DEFAULT_EFFORT);
        let effort_params = effort.params();
        tracing::info!(
            effort = %effort.as_str(),
            max_tool_steps = effort_params.max_tool_steps,
            max_tokens = effort_params.max_tokens_per_call,
            "Agent effort resolved"
        );

        // 合并协作风格 prompt 片段到 custom_instructions
        // Style 与 Effort 正交:Effort 控制"做多少",Style 控制"怎么做"
        let merged_instructions = merge_style_into_instructions(
            request.custom_instructions.as_deref(),
            request.collaboration_style.or(Some(CollaborationStyle::default())),
        );
        // 加载用户画像(磁盘优先,缺失则用默认 profile),注入 system prompt
        let user_profile_store = UserProfileStore::new(self.data_dir.root());
        let system_prompt = identity::build_system_prompt(
            &self.data_dir,
            prompts::MAIN_ROLE,
            merged_instructions.as_deref(),
            Some(user_profile_store.get()),
        );
        let user_message = build_user_message(&request);

        // ── UserPromptSubmit hook ──
        // 在用户 prompt 构造完成后、LLM 调用前派发。若 hook 返回 FailedAbort，则中止。
        let prompt_submit = user_prompt_submit_payload(&request.session_id, None, &request.content);
        try_dispatch(&self.hook_engine, &prompt_submit).await?;

        // TODO(PreCompact): 当 Rust 侧实现上下文压缩逻辑后，在此处派发 PreCompact hook：
        //   let pre_compact = pre_compact_payload(&request.session_id, None);
        //   try_dispatch(&self.hook_engine, &pre_compact).await?;
        // TODO(PostCompact): 压缩完成后派发 PostCompact hook：
        //   let post_compact = post_compact_payload(&request.session_id, None, original_tokens, compacted_tokens);
        //   try_dispatch(&self.hook_engine, &post_compact).await?;
        // 当前 Rust 侧无 compact 逻辑（前端通过 extractive 摘要处理），暂不触发。

        let started = Instant::now();
        let outcome = match provider.to_lowercase().as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval, &effort_params, self.data_dir.clone(), Arc::new(self.clone()));
                run_agent_stream(agent, &user_message, effort_params.max_tool_steps, &tx).await?
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval, &effort_params, self.data_dir.clone(), Arc::new(self.clone()));
                run_agent_stream(agent, &user_message, effort_params.max_tool_steps, &tx).await?
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval, &effort_params, self.data_dir.clone(), Arc::new(self.clone()));
                run_agent_stream(agent, &user_message, effort_params.max_tool_steps, &tx).await?
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval, &effort_params, self.data_dir.clone(), Arc::new(self.clone()));
                run_agent_stream(agent, &user_message, effort_params.max_tool_steps, &tx).await?
            }
            _ => return Err(AppError::provider_not_found(&provider)),
        };

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let input_tokens = outcome.usage.input_tokens as u32;
        let output_tokens = outcome.usage.output_tokens as u32;

        // 通知前端流结束(携带真实 token 用量)
        let _ = tx
            .send(ChatEvent::Finish {
                input_tokens,
                output_tokens,
            })
            .await;

        // 持久化到 messages 表,供仪表盘聚合统计
        self.persist_messages(
            &request.session_id,
            &request.content,
            &outcome,
            &model,
            &provider,
            input_tokens,
            output_tokens,
            elapsed_ms,
        );

        // ── Stop hook（成功路径）──
        // 通知 agent 已正常停止。错误路径不触发（错误本身即停止信号）。
        let stop = stop_payload(&request.session_id, None, true);
        if let Err(e) = try_dispatch(&self.hook_engine, &stop).await {
            tracing::warn!(error = %e, "Stop hook dispatch failed (ignored)");
        }

        Ok(())
    }

    /// 写入 user 消息与 assistant 消息(含 LLM 指标)。指标记录为 best-effort,
    /// 失败时记录警告但不阻断主流程——主流程的成败由 LLM 响应本身决定。
    fn persist_messages(
        &self,
        session_id: &str,
        user_content: &str,
        outcome: &StreamOutcome,
        model: &str,
        provider: &str,
        input_tokens: u32,
        output_tokens: u32,
        latency_ms: u64,
    ) {
        let tool_calls_json = if outcome.tool_calls.is_empty() {
            None
        } else {
            serde_json::to_string(&outcome.tool_calls).ok()
        };
        let tool_results_json = if outcome.tool_results.is_empty() {
            None
        } else {
            serde_json::to_string(&outcome.tool_results).ok()
        };

        if let Err(e) = self.db.create_message(session_id, "user", user_content, None, None) {
            tracing::warn!(error = %e, session_id, "Failed to persist user message");
        }

        let meta = MessageMeta {
            thinking_content: None,
            model: Some(model),
            provider: Some(provider),
            input_tokens,
            output_tokens,
            latency_ms: Some(latency_ms),
        };
        if let Err(e) = self.db.create_message_with_meta(
            session_id,
            "assistant",
            &outcome.text,
            tool_calls_json.as_deref(),
            tool_results_json.as_deref(),
            Some(meta),
        ) {
            tracing::warn!(error = %e, session_id, "Failed to persist assistant message");
        }
    }

    pub async fn stop(&self, _session_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    /// 一次性 LLM 调用(非流式、无工具),返回纯文本。
    /// 供雷达扫描等不需要工具链的简单场景使用。
    pub async fn prompt_once(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AppError> {
        let config = self
            .registry
            .active_model_config()
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
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            _ => Err(AppError::provider_not_found(&provider)),
        }
    }

    /// 为 session 生成短期记忆摘要并 upsert 到 memory_short_term 表。
    ///
    /// 触发时机:
    /// - session commit(用户手动 / 自动保存)
    /// - session 关闭
    /// - 用户主动请求"重新生成摘要"
    ///
    /// 流程:
    /// 1. 拉取 session 的最近 N 条消息(默认 20)
    /// 2. 拼装成 LLM 输入(角色 + 内容摘要)
    /// 3. 调用 LLM 生成 2-3 句话摘要 + 关键 topics(JSON 数组)
    /// 4. upsert 到 memory_short_term(UNIQUE session_id+date → 当天覆盖)
    ///
    /// 失败策略(对齐 "no silent fallback"):
    /// - 拉取消息失败:返回 Err
    /// - LLM 调用失败:返回 Err(不写空摘要)
    /// - JSON 解析失败:用整个响应作为 summary,key_topics 留空数组
    pub async fn summarize_session(
        &self,
        session_id: &str,
        book_id: Option<&str>,
        agent_role: Option<&str>,
    ) -> Result<(), AppError> {
        use crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow;

        // 1. 拉取消息(全量后取最近 20 条)
        let mut messages = self.db.list_messages(session_id)?;
        if messages.is_empty() {
            tracing::debug!(session_id, "skip summarize: no messages");
            return Ok(());
        }
        let total = messages.len();
        if total > 20 {
            messages = messages.split_off(total - 20);
        }
        let message_count = messages.len() as u32;
        let total_tokens: u64 = messages.iter().map(|m| m.token_count.unwrap_or(0) as u64).sum();

        // 2. 拼装 LLM 输入
        let transcript = messages
            .iter()
            .map(|m| format!("[{}] {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let system_prompt = "你是会话摘要助手。请把下面的对话压缩成 2-3 句话摘要,并提取 1-5 个关键主题。输出 JSON 格式:{\"summary\":\"...\",\"key_topics\":[\"...\"]}";
        let user_message = format!(
            "Session ID: {}\n消息数: {}\nToken 总量: {}\n\n对话内容:\n{}",
            session_id, message_count, total_tokens, transcript
        );

        // 3. 调用 LLM
        let raw = self.prompt_once(system_prompt, &user_message).await?;

        // 4. 解析 JSON(失败则用原始响应作为 summary)
        let (summary, key_topics) = parse_summary_response(&raw);

        // 5. upsert 到 DB
        let now = chrono::Utc::now();
        let row = ShortTermMemoryRow {
            id: format!("stm-{}-{}", session_id, now.timestamp_millis()),
            session_id: session_id.to_string(),
            book_id: book_id.map(|s| s.to_string()),
            entry_date: now.format("%Y-%m-%d").to_string(),
            summary,
            key_topics,
            agent_role: agent_role.map(|s| s.to_string()),
            token_count: total_tokens,
            message_count,
            created_at: now.to_rfc3339(),
        };
        self.db.upsert_short_term_memory(&row)?;
        tracing::info!(
            session_id,
            entry_date = %row.entry_date,
            "Session summary persisted"
        );
        Ok(())
    }

    /// 从 session 对话中提取并记录用户偏好到 learned_preferences 表。
    ///
    /// 触发时机:
    /// - 每次 send_message 完成后(异步,不阻塞响应)
    /// - 用户主动请求"分析我的偏好"
    ///
    /// 流程:
    /// 1. 拉取 session 最近 10 条用户消息(role=user)
    /// 2. 调用 LLM 分析:提取偏好键值对(code_style/tone/work_hours/...)
    /// 3. 对每个提取到的偏好调用 upsert_learned_preference
    /// 4. 返回提取的偏好数量
    ///
    /// 失败策略:
    /// - 拉取消息失败:返回 Err
    /// - LLM 调用失败:返回 Err(不静默跳过)
    /// - JSON 解析失败:返回 0(可能 LLM 没识别到偏好,不算错误)
    pub async fn analyze_user_preferences(
        &self,
        session_id: &str,
    ) -> Result<usize, AppError> {
        // 1. 拉取最近 10 条用户消息
        let messages = self.db.list_messages(session_id)?;
        let user_msgs: Vec<_> = messages
            .iter()
            .filter(|m| m.role == "user")
            .rev()
            .take(10)
            .collect::<Vec<_>>();
        if user_msgs.is_empty() {
            tracing::debug!(session_id, "skip preference analysis: no user messages");
            return Ok(0);
        }

        let transcript = user_msgs
            .iter()
            .rev()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n---\n");

        // 2. 调用 LLM 提取偏好
        let system_prompt = "你是用户偏好分析助手。从下面的用户消息中提取可识别的偏好。输出 JSON 数组,每个元素形如 {\"key\":\"code_style\",\"value\":\"concise\"}。如果没有识别到偏好,输出空数组 []。常见 key:code_style, tone, work_hours, favorite_tools, response_length, language, framework.";
        let user_message = format!("用户消息:\n{}", transcript);

        let raw = self.prompt_once(system_prompt, &user_message).await?;

        // 3. 解析 JSON 数组
        let prefs = parse_preferences_response(&raw);
        if prefs.is_empty() {
            tracing::debug!(session_id, "no preferences extracted from session");
            return Ok(0);
        }

        // 4. upsert 每个偏好
        let source = format!("session:{}", session_id);
        for (key, value) in &prefs {
            self.db
                .upsert_learned_preference(key, value, Some(&source))?;
        }
        tracing::info!(
            session_id,
            extracted = prefs.len(),
            "User preferences recorded"
        );
        Ok(prefs.len())
    }
}

/// 构造无工具的简单 agent 并执行一次性 prompt,返回纯文本。
///
/// 使用 Medium Effort 的 max_tokens(8192)作为默认上限。
/// 复杂一次性调用(如雷达分析)可改用 send_message + High Effort 获得更大 token。
async fn run_simple_prompt<M>(
    builder: rig::agent::AgentBuilder<M>,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, AppError>
where
    M: rig::completion::CompletionModel + 'static,
{
    let agent = builder
        .preamble(system_prompt)
        .max_tokens(EffortLevel::Medium.params().max_tokens_per_call)
        .build();
    let response = agent
        .prompt(user_message)
        .await
        .map_err(|e| AppError::stream_error(e.to_string()))?;
    tracing::debug!(
        response_len = response.len(),
        response_preview = &response[..response.len().min(500)],
        "prompt_once response received"
    );
    Ok(response)
}

/// 解析 LLM 摘要响应(容错:JSON 失败则退化为 raw text summary + 空 topics)
///
/// 支持三种 LLM 输出:
/// 1. 纯 JSON:`{"summary":"...","key_topics":["a","b"]}`
/// 2. JSON in code block:```json\n{...}\n```
/// 3. 混杂文本 + JSON(提取第一个 { 到最后 })
fn parse_summary_response(raw: &str) -> (String, String) {
    if let Some(json_str) = extract_json_block(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
            let summary = v
                .get("summary")
                .and_then(|s| s.as_str())
                .unwrap_or(raw)
                .to_string();
            let topics_arr = v
                .get("key_topics")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            return (
                summary,
                serde_json::to_string(&topics_arr).unwrap_or_else(|_| "[]".to_string()),
            );
        }
    }
    // 解析失败:用 raw 作为 summary,topics 留空
    (raw.to_string(), "[]".to_string())
}

/// 解析 LLM 偏好提取响应,返回 (key, value) 元组列表。
///
/// 期望格式:`[{"key":"code_style","value":"concise"}, ...]`
/// 容错:
/// - JSON 解析失败 → 返回空 Vec
/// - 数组中元素缺 key 或 value → 跳过该元素
fn parse_preferences_response(raw: &str) -> Vec<(String, String)> {
    let json_str = match extract_json_array(raw) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let key = item.get("key")?.as_str()?.to_string();
            let value = item.get("value")?.as_str()?.to_string();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

/// 从可能含 code block 的字符串中提取 JSON 数组部分
fn extract_json_array(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with('[') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        if end > start {
            return Some(trimmed[start..=end].to_string());
        }
    }
    None
}

/// 从可能含 code block 的字符串中提取 JSON 部分
fn extract_json_block(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            return Some(trimmed[start..=end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_summary_pure_json() {
        let raw = r#"{"summary":"讨论了章节 3 的剧情","key_topics":["chapter-3","plot"]}"#;
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "讨论了章节 3 的剧情");
        assert!(t.contains("chapter-3"));
        assert!(t.contains("plot"));
    }

    #[test]
    fn parse_summary_code_block() {
        let raw = "这是摘要:\n```json\n{\"summary\":\"很好\",\"key_topics\":[\"a\"]}\n```\n";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "很好");
        assert!(t.contains("\"a\""));
    }

    #[test]
    fn parse_summary_plain_code_block() {
        let raw = "```\n{\"summary\":\"纯文本块\",\"key_topics\":[\"x\",\"y\"]}\n```";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "纯文本块");
        assert!(t.contains("\"x\""));
        assert!(t.contains("\"y\""));
    }

    #[test]
    fn parse_summary_invalid_falls_back_to_raw() {
        let raw = "这不是 JSON,只是普通文本";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, raw);
        assert_eq!(t, "[]");
    }

    #[test]
    fn parse_summary_embedded_json() {
        let raw = "好的,我来总结:\n{\"summary\":\"嵌入 JSON\",\"key_topics\":[\"z\"]}\n以上是总结";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "嵌入 JSON");
        assert!(t.contains("\"z\""));
    }

    #[test]
    fn extract_json_block_handles_no_brace() {
        assert_eq!(extract_json_block("no json here"), None);
    }

    #[test]
    fn parse_preferences_pure_array() {
        let raw = r#"[{"key":"code_style","value":"concise"},{"key":"tone","value":"friendly"}]"#;
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 2);
        assert_eq!(prefs[0], ("code_style".to_string(), "concise".to_string()));
        assert_eq!(prefs[1], ("tone".to_string(), "friendly".to_string()));
    }

    #[test]
    fn parse_preferences_code_block_array() {
        let raw = "好的,我识别到:\n```json\n[{\"key\":\"work_hours\",\"value\":\"09-18\"}]\n```";
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0], ("work_hours".to_string(), "09-18".to_string()));
    }

    #[test]
    fn parse_preferences_empty_array() {
        let raw = "[]";
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 0);
    }

    #[test]
    fn parse_preferences_skips_invalid_items() {
        // 第二项缺 value,第三项缺 key,应当被跳过
        let raw = r#"[
            {"key":"ok","value":"yes"},
            {"key":"bad"},
            {"value":"bad"}
        ]"#;
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0], ("ok".to_string(), "yes".to_string()));
    }

    #[test]
    fn parse_preferences_empty_keys_filtered() {
        let raw = r#"[{"key":"","value":"bad"},{"key":"ok","value":"good"}]"#;
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 1);
    }

    #[test]
    fn parse_preferences_invalid_json_returns_empty() {
        let prefs = parse_preferences_response("no json here");
        assert_eq!(prefs.len(), 0);
    }

    #[test]
    fn extract_json_array_handles_no_bracket() {
        assert_eq!(extract_json_array("no array"), None);
    }
}

/// 统一构造 agent:注入系统提示、token 上限、工具集。
///
/// effort_params 控制投入程度:EffortLevel 越高,max_tokens / max_turns 越大。
///
/// 工具注入策略：
/// - 只读工具无条件注入（fs + 搜索 + 研究 + 确认闸门）
/// - 写/重操作工具按 allow_subagent 条件注入（与 SubAgentTool 同门控）
fn build_agent<M>(
    builder: rig::agent::AgentBuilder<M>,
    system_prompt: &str,
    workspace_root: PathBuf,
    approval: Arc<ApprovalManager>,
    effort_params: &EffortParams,
    data_dir: DataDir,
    engine: Arc<AgentEngine>,
) -> Agent<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    let agent_builder = builder
        .preamble(system_prompt)
        .max_tokens(effort_params.max_tokens_per_call)
        .default_max_turns(effort_params.max_tool_steps)
        .tool(ReadFileTool {
            workspace_root: workspace_root.clone(),
        })
        .tool(ListDirectoryTool {
            workspace_root: workspace_root.clone(),
        })
        .tool(WriteFileTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(CreateDirectoryTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(EditTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(MultiEditTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(TodoWriteTool)
        // 只读工具：书籍目录搜索（无条件）
        .tool(GrepTool { data_dir: data_dir.clone() })
        .tool(LsTool { data_dir: data_dir.clone() })
        // 只读工具：研究与材料（无条件）
        .tool(ResearchWebTool { engine: engine.clone() })
        .tool(IngestMaterialTool { data_dir: data_dir.clone() })
        .tool(RetrieveMaterialTool { data_dir: data_dir.clone() })
        // 确认闸门：纯 JSON，无副作用（无条件）
        .tool(ProposeActionTool);

    // 仅当 Effort 允许 sub-agent 时注入 SubAgentTool 与重操作书籍工具
    // (Low Effort 不允许 spawn sub-agent,避免长任务链开销)
    if effort_params.allow_subagent {
        agent_builder
            .tool(SubAgentTool { workspace_root })
            // 重操作书籍工具：pipeline 委托 + 真相文件 + 实体重命名 + 章节编辑 + 导入 + 封面
            .tool(PipelineDelegateTool {
                engine: engine.clone(),
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            })
            .tool(WriteTruthFileTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            })
            .tool(RenameEntityTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            })
            .tool(PatchChapterTextTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            })
            .tool(ReplaceChapterTextTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            })
            .tool(ImportChaptersTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            })
            .tool(GenerateCoverTool {
                data_dir,
                approval,
            })
            .build()
    } else {
        agent_builder.build()
    }
}

/// 驱动 agent 的流式多轮循环,聚合文本、token 用量与工具调用记录。
///
/// max_tool_steps 由 EffortLevel 决定(Low=5, Medium=20, High=50, Ultra=100)。
/// 
/// 带重试机制：针对 503 等临时性服务端错误，使用指数退避进行最多 3 次重试。
async fn run_agent_stream<M>(
    agent: Agent<M>,
    user_message: &str,
    max_tool_steps: usize,
    tx: &mpsc::Sender<ChatEvent>,
) -> Result<StreamOutcome, AppError>
where
    M: rig::completion::CompletionModel + 'static,
    <M as rig::completion::CompletionModel>::StreamingResponse: GetTokenUsage + Clone + Unpin,
{
    let mut last_error: Option<String> = None;
    
    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay_ms = RETRY_BASE_DELAY_MS * (1 << (attempt - 1));
            tracing::warn!(
                attempt,
                delay_ms,
                "LLM stream returned retryable error, retrying with exponential backoff"
            );
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        
        let stream = agent
            .stream_prompt(user_message.to_string())
            .multi_turn(max_tool_steps)
            .await;
        
        match consume_stream_with_retry_detection(stream, tx).await {
            Ok(outcome) => return Ok(outcome),
            Err(e) => {
                if is_retryable_error(&e.message) && attempt < MAX_RETRIES {
                    last_error = Some(e.message.clone());
                    continue;
                }
                return Err(e);
            }
        }
    }
    
    Err(AppError::stream_error(
        last_error.unwrap_or_else(|| "Max retries exceeded".to_string())
    ))
}

/// 消费流并检测可重试错误
async fn consume_stream_with_retry_detection<S, R>(
    mut stream: S,
    tx: &mpsc::Sender<ChatEvent>,
) -> Result<StreamOutcome, AppError>
where
    S: futures_util::stream::Stream<Item = Result<MultiTurnStreamItem<R>, StreamingError>> + Unpin,
    R: GetTokenUsage + Clone + Unpin,
{
    let mut text = String::new();
    let mut usage = Usage::new();
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();
    let mut tool_results: Vec<serde_json::Value> = Vec::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(content)) => match content {
                StreamedAssistantContent::Text(t) => {
                    let delta = t.text.to_string();
                    let _ = tx
                        .send(ChatEvent::TextDelta {
                            content: delta.clone(),
                        })
                        .await;
                    text.push_str(&delta);
                }
                StreamedAssistantContent::ToolCall {
                    tool_call, ..
                } => {
                    let _ = tx
                        .send(ChatEvent::ToolCallStart {
                            id: tool_call.id.clone(),
                            name: tool_call.function.name.clone(),
                        })
                        .await;
                    let _ = tx
                        .send(ChatEvent::ToolCallEnd {
                            id: tool_call.id.clone(),
                        })
                        .await;
                    tool_calls.push(serde_json::json!({
                        "id": tool_call.id,
                        "name": tool_call.function.name,
                        "arguments": tool_call.function.arguments,
                    }));
                }
                StreamedAssistantContent::Final(r) => {
                    let u = r.token_usage();
                    if u.has_values() {
                        usage = u;
                    }
                }
                _ => {}
            },
            Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult {
                tool_result, ..
            })) => {
                tool_results.push(serde_json::json!({
                    "id": tool_result.id,
                    "content": format!("{:?}", tool_result.content),
                }));
            }
            Ok(MultiTurnStreamItem::CompletionCall(cc)) => {
                if cc.usage.has_values() {
                    usage = cc.usage;
                }
            }
            Ok(MultiTurnStreamItem::FinalResponse(fr)) => {
                let final_text = fr.response().to_string();
                if !final_text.is_empty() {
                    text = final_text;
                }
                let final_usage = fr.usage();
                if final_usage.has_values() {
                    usage = final_usage;
                }
            }
            Ok(_) => {}
            Err(e) => {
                let err_str = e.to_string();
                return Err(AppError::stream_error(err_str));
            }
        }
    }

    Ok(StreamOutcome {
        text,
        usage,
        tool_calls,
        tool_results,
    })
}

/// 检查错误是否可重试（临时性服务端错误）
fn is_retryable_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("503")
        || lower.contains("502")
        || lower.contains("429")
        || lower.contains("overloaded")
        || lower.contains("unavailable")
        || lower.contains("rate limit")
        || lower.contains("timeout")
}

fn build_user_message(request: &ChatRequest) -> String {
    let mut message = String::new();
    if let Some(ref context) = request.context_text {
        message.push_str("Context:\n");
        message.push_str(context);
        message.push_str("\n\n");
    }
    message.push_str(&request.content);
    message
}
