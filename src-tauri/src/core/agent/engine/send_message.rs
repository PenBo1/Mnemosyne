//! ═══════════════════════════════════════════════════════════════════════════
//! SendMessage - Agent 消息发送编排
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! `send_message` 编排：AgentEngine 主入口，驱动完整的 agent turn。
//!
//! 流程：hook 派发 → prompt 构建 → tool 注册 → 流式执行 → 持久化 → 取消清理。
//! 从 engine.rs 拆分，保持生命周期/访问器方法紧凑。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use uuid::Uuid;
use tokio::sync::{mpsc, watch};

use crate::infrastructure::db::stores::session::MessageMeta;
use crate::infrastructure::telemetry::SpanKind;
use crate::security_kernel::hooks::{
    try_dispatch,
    session_start_payload, stop_payload, user_prompt_submit_payload,
};
use crate::shared::error::AppError;

use crate::core::agent::approval::ApprovalManager;
use crate::core::agent::collaboration_style::{merge_style_into_instructions, CollaborationStyle};
use crate::core::agent::user_profile::UserProfileSnapshot;
use crate::core::agent::prompts;
use crate::core::agent::types::{ChatEvent, ChatRequest};

use super::AgentEngine;
use super::DEFAULT_EFFORT;
use super::stream::{run_agent_stream, StreamOutcome};
use super::tools::build_tools;

/// RAII guard：确保 `send_message` 在任何返回路径（成功 / Err `?` 传播 / panic）
/// 都从 `cancellation_tokens` 移除本会话条目，防止 map 无限增长。
///
/// 正常路径在函数末尾将 `cleaned_up` 置 true，Drop 变为 no-op。
/// 异常路径（含 `?` 提前返回）由 Drop 兜底清理。
struct CancelTokenGuard<'a> {
    engine: &'a AgentEngine,
    session_id: String,
    cleaned_up: bool,
}

impl Drop for CancelTokenGuard<'_> {
    fn drop(&mut self) {
        if self.cleaned_up {
            return;
        }
        self.engine.cancellation_tokens.remove(&self.session_id);
        tracing::warn!(
            session_id = %self.session_id,
            "Cancellation token cleaned up via Drop guard (fallback path)"
        );
    }
}

impl AgentEngine {
    pub async fn send_message(
        &self,
        request: ChatRequest,
        workspace_root: PathBuf,
        approval: Arc<ApprovalManager>,
        tx: mpsc::Sender<ChatEvent>,
        user_profile: Option<UserProfileSnapshot>,
    ) -> Result<(), AppError> {
        // tx 直接 move 进 send_message_inner：失败路径仅通过 Err 返回，不再向 channel 发送 Error 事件。
        let result = self
            .send_message_inner(request, workspace_root, approval, tx, user_profile)
            .await;
        if let Err(ref e) = result {
            // 错误统一经由 IPC Err 转发给前端（chat_send_message → invoke reject → 前端 catch）。
            // 不再通过 tx.send(ChatEvent::Error) 发送，否则前端 onmessage 与 invoke catch 会双重处理。
            tracing::error!(error = %e, "[agent] send_message failed, error forwarded via IPC reject");
        }
        result
    }

    async fn send_message_inner(
        &self,
        request: ChatRequest,
        workspace_root: PathBuf,
        approval: Arc<ApprovalManager>,
        tx: mpsc::Sender<ChatEvent>,
        user_profile: Option<UserProfileSnapshot>,
    ) -> Result<(), AppError> {
        let request_id = Uuid::new_v4();
        let start_time = Instant::now();
        let session_id = request.session_id.clone();

        async fn inner(
            this: &AgentEngine,
            request: ChatRequest,
            workspace_root: PathBuf,
            approval: Arc<ApprovalManager>,
            tx: mpsc::Sender<ChatEvent>,
            user_profile: Option<UserProfileSnapshot>,
            request_id: Uuid,
            start_time: Instant,
            session_id: String,
        ) -> Result<(), AppError> {
            let content_preview: String = request.content.chars().take(50).collect();

            tracing::info!(
                request_id = %request_id,
                session_id = %session_id,
                content_preview = %content_preview,
                has_context = request.context_text.is_some(),
                has_custom_instructions = request.custom_instructions.is_some(),
                effort = request.effort.map(|e| e.as_str()),
                collaboration_style = request.collaboration_style.map(|s| s.as_str()),
                workspace_root = %workspace_root.display(),
                "[agent] send_message: received request"
            );

            // ── SessionStart hook ──
            // 在 agent 流程开始时派发。若 hook 返回 FailedAbort，则中止本次会话。
            tracing::debug!(request_id = %request_id, session_id = %session_id, "[agent] dispatching SessionStart hook");
            let session_start = session_start_payload(&request.session_id, None, Some(prompts::MAIN_ROLE));
            try_dispatch(&this.hook_engine, &session_start).await?;

            // Trace span: 记录本次 agent 调用的耗时与上下文（RAII Drop 自动写入 DB）
            let tracer = this.tracer();
            let mut span = tracer.start_span("agent.send_message", SpanKind::Internal);
            span.set_session(&request.session_id);

            let config = this
                .registry
                .active_model_config()
                .ok_or_else(AppError::no_active_model)?;

            let provider_name = config.provider.clone();
            let model = config.model.clone();

            // 通过 ProviderRegistry 获取激活的 Provider 实例（内部缓存）
            let provider = this.registry.active_provider()?;

            tracing::debug!(
                request_id = %request_id,
                session_id = %session_id,
                provider = %provider_name,
                model = %model,
                "[agent] active provider resolved"
            );

            // 解析 Effort:请求级覆盖 > 全局默认
            let effort = request.effort.unwrap_or(DEFAULT_EFFORT);
            let effort_params = effort.params();
            tracing::info!(
                request_id = %request_id,
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
            // 用户画像快照由 application/ 层通过 UserProfileProvider 注入（core/agent
            // 不依赖 domain::user）。None 时 system prompt 不含 user profile 段。
            tracing::debug!(request_id = %request_id, session_id = %session_id, "[agent] loading identity and building system prompt");
            // simple chat（无 context_text）不加载 extended context（pipeline_overview 等），
            // 节省 token；book/章节任务（有 context_text）加载 core + extended。
            let load_extended_context = request.context_text.is_some();
            let system_prompt = this
                .build_system_prompt(
                    prompts::MAIN_ROLE,
                    merged_instructions.as_deref(),
                    user_profile.as_ref(),
                    load_extended_context,
                )
                .await;
            tracing::debug!(
                request_id = %request_id,
                session_id = %session_id,
                system_prompt_len = system_prompt.len(),
                "[agent] system prompt built"
            );
            let user_message = build_user_message(&request);

            // ── UserPromptSubmit hook ──
            // 在用户 prompt 构造完成后、LLM 调用前派发。若 hook 返回 FailedAbort，则中止。
            let prompt_submit = user_prompt_submit_payload(&request.session_id, None, &request.content);
            try_dispatch(&this.hook_engine, &prompt_submit).await?;

            // 只 clone 一次 self 并包装为 Arc，供各工具复用（避免重复深拷贝）。
            // 注：SubAgentTool / PipelineDelegateTool / ResearchWebTool 等需要回调
            // engine 的工具持有 Arc<AgentEngine>；此处统一构造。
            let engine_arc = Arc::new(this.clone());

            // 构造工具集:按 EffortLevel 分档(Low=3/Medium=15/High+Ultra=23)
            // 并应用可选 tool_whitelist 交集筛选
            let tools = build_tools(
                &workspace_root,
                &approval,
                effort,
                request.tool_whitelist.as_deref(),
                &this.data_dir,
                &engine_arc,
            );

            // 注册取消令牌：watch::<bool>，初值 false（未取消）。
            // `stop(session_id)` 会发送 true，`run_agent_stream` 在 select! 中监听。
            let (cancel_tx, mut cancel_rx) = watch::channel(false);
            this.cancellation_tokens
                .insert(request.session_id.clone(), Arc::new(cancel_tx));
            tracing::debug!(request_id = %request_id, session_id = %session_id, "[agent] cancellation token registered");

            // RAII guard：确保 `send_message` 在任何返回路径（成功 / Err `?` 传播 / panic）
            // 都从 `cancellation_tokens` 移除本会话条目，防止 map 无限增长。
            // 正常路径在函数末尾将 `cleaned_up` 置 true，Drop 变为 no-op。
            let mut cancel_guard = CancelTokenGuard {
                engine: this,
                session_id: request.session_id.clone(),
                cleaned_up: false,
            };

            tracing::info!(
                request_id = %request_id,
                session_id = %session_id,
                provider = %provider_name,
                model = %model,
                max_tool_steps = effort_params.max_tool_steps,
                max_tokens = effort_params.max_tokens_per_call,
                allow_subagent = effort_params.allow_subagent,
                "[agent] starting stream execution"
            );

            let outcome = run_agent_stream(
                provider,
                tools,
                model.clone(),
                system_prompt,
                user_message,
                effort_params.max_tool_steps,
                effort_params.max_tokens_per_call,
                &tx,
                &mut cancel_rx,
                request_id,
            )
            .await?;

            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            let input_tokens = outcome.usage.input_tokens;
            let output_tokens = outcome.usage.output_tokens;

            tracing::info!(
                request_id = %request_id,
                session_id = %session_id,
                duration_ms = elapsed_ms,
                input_tokens,
                output_tokens,
                tool_calls = outcome.tool_calls.len(),
                "[agent] send_message: completed"
            );

            // 冗余发送：ChatEvent::Finish 已在 stream.rs 收到 MultiTurnEvent::Finish 时发送
            // 此处保留作为保障，若 channel 已关闭属正常情况（前端可能已关闭窗口）
            if tx
                .send(ChatEvent::Finish {
                    input_tokens,
                    output_tokens,
                })
                .await
                .is_err()
            {
                tracing::debug!(
                    request_id = %request_id,
                    session_id = %request.session_id,
                    "Redundant ChatEvent::Finish send failed (channel closed, already sent via stream.rs)"
                );
            }

            // 持久化到 messages 表,供仪表盘聚合统计
            tracing::debug!(request_id = %request_id, session_id = %session_id, "[agent] persisting messages to database");
            this.persist_messages(
                &request.session_id,
                &request.content,
                &outcome,
                &model,
                &provider_name,
                input_tokens,
                output_tokens,
                elapsed_ms,
            );

            // ── Stop hook（成功路径）──
            // 通知 agent 已正常停止。错误路径不触发（错误本身即停止信号）。
            tracing::debug!(request_id = %request_id, session_id = %session_id, "[agent] dispatching Stop hook");
            let stop = stop_payload(&request.session_id, None, true);
            if let Err(e) = try_dispatch(&this.hook_engine, &stop).await {
                tracing::warn!(error = %e, "Stop hook dispatch failed (ignored)");
            }

            // 标记 guard 为 no-op，避免 Drop 重复 remove（条目已不需要——会话已结束）
            cancel_guard.cleaned_up = true;
            // 显式移除取消令牌（与 guard Drop 等价，但这里语义更清晰：会话正常结束）
            this.cancellation_tokens.remove(&request.session_id);
            tracing::debug!(request_id = %request_id, session_id = %session_id, "[agent] cancellation token removed");

            Ok(())
        }

        inner(self, request, workspace_root, approval, tx, user_profile, request_id, start_time, session_id.clone())
            .await
            .map_err(|err| {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                tracing::error!(
                    request_id = %request_id,
                    session_id = %session_id,
                    error = ?err,
                    duration_ms,
                    "[agent] send_message: error"
                );
                err
            })
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
