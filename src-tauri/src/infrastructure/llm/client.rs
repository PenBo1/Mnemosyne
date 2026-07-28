//! ═══════════════════════════════════════════════════════════════════════════
//! LLM 客户端 - 统一请求封装
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 封装 Provider 调用，提供：
//! - 请求日志（request_id、duration_ms）
//! - 统一错误处理
//! - 流式响应日志（chunk 计数、token 统计）

use std::sync::Arc;
use std::time::Instant;
use futures_util::StreamExt;
use uuid::Uuid;
use super::types::*;
use crate::shared::error::AppError;

pub struct LlmClient {
    provider: Arc<dyn Provider>,
}

impl LlmClient {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> &Arc<dyn Provider> {
        &self.provider
    }

    pub async fn complete(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        max_tokens: u64,
    ) -> Result<String, AppError> {
        let request_id = Uuid::new_v4();
        let start_time = Instant::now();
        let provider = self.provider.name();

        tracing::info!(
            request_id = %request_id,
            provider = %provider,
            model = %model,
            has_messages = !messages.is_empty(),
            message_count = messages.len(),
            max_tokens = max_tokens,
            "[llm] API request started"
        );

        let result = self.provider.complete(model, system, messages, max_tokens).await;

        match result {
            Ok(response) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let content_preview = if response.len() > 50 {
                    format!("{}...", &response[..50])
                } else {
                    response.clone()
                };

                tracing::info!(
                    request_id = %request_id,
                    provider = %provider,
                    model = %model,
                    status = "success",
                    duration_ms = duration_ms,
                    response_len = response.len(),
                    content_preview = %content_preview,
                    "[llm] API response received"
                );

                Ok(response)
            }
            Err(err) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;

                tracing::error!(
                    request_id = %request_id,
                    provider = %provider,
                    model = %model,
                    error = %err,
                    duration_ms = duration_ms,
                    "[llm] API request failed"
                );

                Err(err)
            }
        }
    }

    pub async fn complete_with_tools(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
        max_tokens: u64,
    ) -> Result<String, AppError> {
        let request_id = Uuid::new_v4();
        let start_time = Instant::now();
        let provider = self.provider.name();

        tracing::info!(
            request_id = %request_id,
            provider = %provider,
            model = %model,
            has_messages = !messages.is_empty(),
            message_count = messages.len(),
            tool_count = tools.len(),
            max_tokens = max_tokens,
            "[llm] API request started (with tools)"
        );

        let result = self.provider.complete_with_tools(model, system, messages, tools, max_tokens).await;

        match result {
            Ok(response) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let content_preview = if response.len() > 50 {
                    format!("{}...", &response[..50])
                } else {
                    response.clone()
                };

                tracing::info!(
                    request_id = %request_id,
                    provider = %provider,
                    model = %model,
                    status = "success",
                    duration_ms = duration_ms,
                    response_len = response.len(),
                    content_preview = %content_preview,
                    "[llm] API response received (with tools)"
                );

                Ok(response)
            }
            Err(err) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;

                tracing::error!(
                    request_id = %request_id,
                    provider = %provider,
                    model = %model,
                    error = %err,
                    duration_ms = duration_ms,
                    "[llm] API request failed (with tools)"
                );

                Err(err)
            }
        }
    }

    pub async fn stream(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
        max_tokens: u64,
    ) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>, AppError> {
        let request_id = Uuid::new_v4();
        let start_time = Instant::now();
        let provider = self.provider.name();

        tracing::info!(
            request_id = %request_id,
            provider = %provider,
            model = %model,
            has_messages = !messages.is_empty(),
            message_count = messages.len(),
            tool_count = tools.len(),
            max_tokens = max_tokens,
            "[llm] stream request started"
        );

        let stream_result = self.provider.stream(model, system, messages, tools, max_tokens).await;

        match stream_result {
            Ok(stream) => {
                let request_id_clone = request_id;
                let provider_name = provider.to_string();
                let model_name = model.to_string();
                let start_time_clone = start_time;

                let chunk_count = std::sync::atomic::AtomicU64::new(0);
                let chunk_count_clone = std::sync::Arc::new(chunk_count);

                let logged_stream = stream
                    .inspect(move |event| {
                        let chunk_num = chunk_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        match event {
                            StreamEvent::TextDelta { content } => {
                                tracing::debug!(
                                    request_id = %request_id_clone,
                                    chunk_num = chunk_num,
                                    event_type = "text_delta",
                                    chunk_size = content.len(),
                                    "[llm] stream chunk received"
                                );
                            }
                            StreamEvent::ReasoningDelta { content } => {
                                tracing::debug!(
                                    request_id = %request_id_clone,
                                    chunk_num = chunk_num,
                                    event_type = "reasoning_delta",
                                    chunk_size = content.len(),
                                    "[llm] stream chunk received"
                                );
                            }
                            StreamEvent::ToolCallStart { id, name } => {
                                tracing::debug!(
                                    request_id = %request_id_clone,
                                    chunk_num = chunk_num,
                                    event_type = "tool_call_start",
                                    tool_id = %id,
                                    tool_name = %name,
                                    "[llm] stream chunk received"
                                );
                            }
                            StreamEvent::ToolCallDelta { id, args_delta } => {
                                tracing::debug!(
                                    request_id = %request_id_clone,
                                    chunk_num = chunk_num,
                                    event_type = "tool_call_delta",
                                    tool_id = %id,
                                    delta_size = args_delta.len(),
                                    "[llm] stream chunk received"
                                );
                            }
                            StreamEvent::ToolCallEnd { id } => {
                                tracing::debug!(
                                    request_id = %request_id_clone,
                                    chunk_num = chunk_num,
                                    event_type = "tool_call_end",
                                    tool_id = %id,
                                    "[llm] stream chunk received"
                                );
                            }
                            StreamEvent::Finish { reason, usage } => {
                                let duration_ms = start_time_clone.elapsed().as_millis() as u64;
                                let total_chunks = chunk_count_clone.load(std::sync::atomic::Ordering::Relaxed);

                                tracing::info!(
                                    request_id = %request_id_clone,
                                    provider = %provider_name,
                                    model = %model_name,
                                    finish_reason = ?reason,
                                    total_chunks = total_chunks,
                                    duration_ms = duration_ms,
                                    input_tokens = usage.input_tokens,
                                    output_tokens = usage.output_tokens,
                                    "[llm] stream completed"
                                );
                            }
                            StreamEvent::Error(err) => {
                                let duration_ms = start_time_clone.elapsed().as_millis() as u64;

                                tracing::error!(
                                    request_id = %request_id_clone,
                                    provider = %provider_name,
                                    model = %model_name,
                                    error = %err,
                                    duration_ms = duration_ms,
                                    "[llm] stream error"
                                );
                            }
                        }
                    })
                    .map(|event| event);

                Ok(Box::pin(logged_stream))
            }
            Err(err) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;

                tracing::error!(
                    request_id = %request_id,
                    provider = %provider,
                    model = %model,
                    error = %err,
                    duration_ms = duration_ms,
                    "[llm] stream request failed"
                );

                Err(err)
            }
        }
    }

    pub async fn test_connection(&self) -> Result<(), AppError> {
        let provider = self.provider.name();
        tracing::info!(provider = %provider, "[llm] testing connection");
        self.provider.test_connection().await
    }

    pub fn name(&self) -> &str {
        self.provider.name()
    }

    pub fn models(&self) -> Vec<ModelInfo> {
        self.provider.models()
    }

    pub fn base_url(&self) -> &str {
        self.provider.base_url()
    }
}

impl std::fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClient")
            .field("provider", &self.provider.name())
            .finish()
    }
}