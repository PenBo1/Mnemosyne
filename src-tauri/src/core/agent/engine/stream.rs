//! ═══════════════════════════════════════════════════════════════════════════
//! Stream - Agent 流式消费与重试
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! Agent 流式消费：驱动 MultiTurnRunner，聚合文本/用量/tool calls，
//! 并针对临时性服务端错误应用重试/退避策略。
//!
//! 从 engine.rs 拆分，保持编排模块专注于 AgentEngine 生命周期和 IPC 入口。

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use uuid::Uuid;
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;

use crate::infrastructure::llm::tool::Tool;
use crate::infrastructure::llm::types::{Message, Provider, TokenUsage};
use crate::shared::error::AppError;

use crate::core::agent::multi_turn::{MultiTurnEvent, MultiTurnRunner, MultiTurnStream};
use crate::core::agent::thinking_scrubber::StreamingThinkScrubber;
use crate::core::agent::types::ChatEvent;

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;

/// 一次 agent 流式调用的聚合结果,用于持久化到 messages 表。
pub(super) struct StreamOutcome {
    pub(super) text: String,
    pub(super) usage: TokenUsage,
    pub(super) tool_calls: Vec<serde_json::Value>,
    pub(super) tool_results: Vec<serde_json::Value>,
}

/// 驱动 MultiTurnRunner 的流式多轮循环,聚合文本、token 用量与工具调用记录。
///
/// max_turns 由 EffortLevel 决定(Low=5, Medium=20, High=50, Ultra=100)。
/// max_tokens 由 EffortLevel 决定(Low=2048, Medium=8192, High=16384, Ultra=32768)，
/// 透传到 Provider.stream 控制单次调用输出 token 上限。
///
/// 带重试机制：针对 503 等临时性服务端错误，使用指数退避进行重试。
/// 总尝试次数为 `MAX_RETRIES`（含首次），即最多 `MAX_RETRIES - 1` 次重试。
/// 重试前向 frontend emit `Retry` 事件分隔前次不完整输出。
///
/// 取消语义：`cancel_rx` 在每次 stream `next().await` 处被 `select!` 监听
/// （在 consume_stream_with_retry_detection 内），同时 MultiTurnRunner 内部
/// 也监听 cancel_rx。收到取消信号时立即返回 `AppError::task_cancelled()`，
/// 停止后续 tool 调用与流式消费。
pub(super) async fn run_agent_stream(
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    model: String,
    system: String,
    user_message: String,
    max_turns: usize,
    max_tokens: u64,
    tx: &mpsc::Sender<ChatEvent>,
    cancel_rx: &mut watch::Receiver<bool>,
    request_id: Uuid,
) -> Result<StreamOutcome, AppError> {
    tracing::debug!(
        request_id = %request_id,
        max_turns,
        max_tokens,
        user_message_len = user_message.len(),
        "[agent] run_agent_stream: starting"
    );

    let mut last_error: Option<String> = None;

    // 0..MAX_RETRIES：总尝试次数 = MAX_RETRIES（含首次），避免 0..=MAX_RETRIES 的 off-by-one。
    // 例：MAX_RETRIES=3 → attempts 0,1,2 = 3 次（首次 + 2 次重试）。
    for attempt in 0..MAX_RETRIES {
        // 取消检查：进入下一次尝试前若已取消，立即返回
        if *cancel_rx.borrow() {
            tracing::debug!("[agent] run_agent_stream: cancelled before attempt {}", attempt);
            return Err(AppError::task_cancelled());
        }

        if attempt > 0 {
            let delay_ms = RETRY_BASE_DELAY_MS * (1 << (attempt - 1));
            tracing::warn!(
                attempt,
                max_attempts = MAX_RETRIES,
                delay_ms,
                "[agent] LLM stream returned retryable error, retrying with exponential backoff"
            );
            // 向 frontend emit Retry 事件，让 UI 能分隔前次不完整输出与重试输出
            let _ = tx
                .send(ChatEvent::Retry {
                    attempt,
                    max_attempts: MAX_RETRIES,
                })
                .await;
            // 退避期间也监听取消信号
            tokio::select! {
                _ = sleep(Duration::from_millis(delay_ms)) => {}
                _ = cancel_rx.changed() => {
                    tracing::debug!("[agent] run_agent_stream: cancelled during backoff");
                    return Err(AppError::task_cancelled());
                }
            }
        }

        tracing::debug!(attempt, max_attempts = MAX_RETRIES, "[agent] starting stream attempt");

        // MultiTurnRunner::run 消费 self，每次重试需重建 runner。
        // provider (Arc) 与 tools (Vec<Arc>) 均为廉价克隆。
        let runner = MultiTurnRunner::new(
            Arc::clone(&provider),
            tools.clone(),
            model.clone(),
            max_turns,
            max_tokens,
        );
        let messages = vec![Message {
            role: "user".to_string(),
            content: user_message.clone(),
            tool_calls: None,
            tool_call_id: None,
        }];
        // watch::Receiver 实现 Clone，runner 内部用克隆的 receiver 监听取消
        let stream = runner.run(system.clone(), messages, cancel_rx.clone());

        match consume_stream_with_retry_detection(stream, tx, cancel_rx, request_id).await {
            Ok(outcome) => {
                tracing::debug!(
                    text_len = outcome.text.len(),
                    tool_calls = outcome.tool_calls.len(),
                    tool_results = outcome.tool_results.len(),
                    "[agent] stream attempt succeeded"
                );
                return Ok(outcome);
            }
            Err(e) => {
                tracing::debug!(
                    error = %e.message,
                    is_retryable = is_retryable_error(&e.message),
                    "[agent] stream attempt failed"
                );
                if is_retryable_error(&e.message) && attempt + 1 < MAX_RETRIES {
                    last_error = Some(e.message.clone());
                    continue;
                }
                return Err(e);
            }
        }
    }

    tracing::error!(
        last_error = last_error.as_deref(),
        "[agent] run_agent_stream: max retries exceeded"
    );
    Err(AppError::stream_error(
        last_error.unwrap_or_else(|| "Max retries exceeded".to_string())
    ))
}

/// 消费 MultiTurnStream 并检测可重试错误
///
/// 将 MultiTurnEvent 转换为 ChatEvent 转发给前端，同时聚合 StreamOutcome。
///
/// 在每个 `stream.next().await` 处用 `select!` 监听 `cancel_rx.changed()`，
/// 取消时立即返回 `AppError::task_cancelled()`（已收集的部分文本被丢弃）。
///
/// 超时分为两档（替代原 60s 单一硬超时，避免大 prompt 首 token 延迟被误判）：
/// - 首字节超时（90s）：流启动后 90s 内未收到任何事件，返回 stream_error。
///   大 prompt（~14500 tokens）首字节延迟可能超过 60s，故放宽到 90s。
/// - 流间隔超时（30s）：收到首事件后，两个事件间隔超过 30s 视为流卡住，
///   返回 stream_error。两者均可被上层重试。
async fn consume_stream_with_retry_detection(
    mut stream: MultiTurnStream,
    tx: &mpsc::Sender<ChatEvent>,
    cancel_rx: &mut watch::Receiver<bool>,
    request_id: Uuid,
) -> Result<StreamOutcome, AppError> {
    tracing::debug!(request_id = %request_id, "[agent] consume_stream: starting stream consumption");

    // 首字节超时：等待第一个事件到达的最长时间（大 prompt 首 token 延迟较高）
    const FIRST_BYTE_TIMEOUT: Duration = Duration::from_secs(90);
    // 流间隔超时：两个事件之间允许的最大间隔
    const STREAM_STALL_TIMEOUT: Duration = Duration::from_secs(30);

    let mut text = String::new();
    let mut usage = TokenUsage::default();
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();
    let mut tool_results: Vec<serde_json::Value> = Vec::new();
    let mut thinking_scrubber = StreamingThinkScrubber::new();
    let mut reasoning_buffer = String::new();
    let mut tool_start_times: std::collections::HashMap<String, Instant> = std::collections::HashMap::new();
    // 累积每个 tool_call 的 args_delta（id → arguments 字符串），ToolCallEnd 时落盘
    let mut tool_call_args: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut tool_call_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 流启动时刻，用于计算首字节超时截止时间
    let stream_start = Instant::now();
    // 最近一次收到事件的时间；None 表示尚未收到任何事件（启用首字节超时）
    let mut last_event_time: Option<Instant> = None;

    let mut events_received: u64 = 0;
    let mut chat_events_sent: u64 = 0;

    loop {
        // 计算当前轮次的超时截止时间：未收到首事件用首字节超时，否则用流间隔超时
        let deadline = match last_event_time {
            None => stream_start + FIRST_BYTE_TIMEOUT,
            Some(t) => t + STREAM_STALL_TIMEOUT,
        };
        let remaining = deadline.saturating_duration_since(Instant::now());

        let next_item = tokio::select! {
            item = stream.next() => item,
            _ = cancel_rx.changed() => {
                tracing::debug!("[agent] consume_stream: cancelled during stream");
                return Err(AppError::task_cancelled());
            }
            _ = tokio::time::sleep(remaining) => {
                // 截止时间已到，按是否曾收到事件区分两种超时
                match last_event_time {
                    None => {
                        tracing::warn!(
                            timeout_ms = FIRST_BYTE_TIMEOUT.as_millis() as u64,
                            "[agent] consume_stream: first byte timeout, no data received"
                        );
                        return Err(AppError::stream_error(
                            "First byte timeout: no data for 90 seconds".to_string(),
                        ));
                    }
                    Some(_) => {
                        tracing::warn!(
                            timeout_ms = STREAM_STALL_TIMEOUT.as_millis() as u64,
                            "[agent] consume_stream: stream stalled, no data between events"
                        );
                        return Err(AppError::stream_error(
                            "Stream stalled: no data for 30 seconds".to_string(),
                        ));
                    }
                }
            }
        };

        let Some(event) = next_item else {
            tracing::debug!(
                text_len = text.len(),
                tool_calls = tool_calls.len(),
                tool_results = tool_results.len(),
                "[stream] consume_stream: stream ended"
            );
            break;
        };

        // 收到事件，刷新最近事件时间（后续轮次改用流间隔超时）
        last_event_time = Some(Instant::now());
        events_received += 1;

        match event {
            MultiTurnEvent::TextDelta { content } => {
                let delta = content;
                tracing::trace!(delta_len = delta.len(), "[agent] TextDelta received from LLM");
                tracing::debug!(delta_len = delta.len(), "[stream] MultiTurnEvent::TextDelta received");
                tracing::trace!(
                    delta_preview = %delta.chars().take(100).collect::<String>(),
                    delta_len = delta.len(),
                    "[stream] raw TextDelta from multi_turn"
                );

                // Process through thinking scrubber to extract thinking blocks
                let visible = thinking_scrubber.feed(&delta);
                let scrubber_output_len = visible.as_ref().map_or(0, |s| s.len());
                tracing::debug!(
                    input_len = delta.len(),
                    output_len = scrubber_output_len,
                    in_block = thinking_scrubber.is_in_block(),
                    "[stream] scrubber: processed TextDelta"
                );
                if let Some(visible) = visible {
                    if !visible.is_empty() {
                        tracing::trace!(visible_len = visible.len(), "[agent] sending visible TextDelta to frontend");
                        tracing::debug!(visible_len = visible.len(), "[stream] ChatEvent::TextDelta sent");
                        match tokio::time::timeout(
                            Duration::from_secs(5),
                            tx.send(ChatEvent::TextDelta {
                                content: visible.clone(),
                            }),
                        )
                        .await
                        {
                            Ok(Ok(_)) => {
                                chat_events_sent += 1;
                            }
                            Ok(Err(_)) => {
                                tracing::debug!(
                                    "[agent] Failed to send TextDelta to frontend (channel closed)"
                                );
                            }
                            Err(_) => {
                                tracing::warn!(
                                    event_kind = "textDelta",
                                    "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                                );
                            }
                        }
                        text.push_str(&visible);
                    }
                }

                // Check if we have accumulated reasoning content in the thinking block
                if let Some(block_content) = thinking_scrubber.current_block_content() {
                    if block_content.len() > reasoning_buffer.len() {
                        let new_reasoning = &block_content[reasoning_buffer.len()..];
                        if !new_reasoning.is_empty() {
                            tracing::trace!(
                                reasoning_len = new_reasoning.len(),
                                "[agent] sending ReasoningDelta to frontend"
                            );
                            match tokio::time::timeout(
                                Duration::from_secs(5),
                                tx.send(ChatEvent::ReasoningDelta {
                                    content: new_reasoning.to_string(),
                                }),
                            )
                            .await
                            {
                                Ok(Ok(_)) => {
                                    chat_events_sent += 1;
                                }
                                Ok(Err(_)) => {
                                    tracing::debug!(
                                        "[agent] Failed to send ReasoningDelta to frontend (channel closed)"
                                    );
                                }
                                Err(_) => {
                                    tracing::warn!(
                                        event_kind = "reasoningDelta",
                                        "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                                    );
                                }
                            }
                            reasoning_buffer = block_content.to_string();
                        }
                    }
                }
            }
            MultiTurnEvent::ReasoningDelta { content } => {
                tracing::trace!(
                    reasoning_len = content.len(),
                    "[agent] sending ReasoningDelta to frontend"
                );
                tracing::debug!(
                    content_len = content.len(),
                    "[stream] MultiTurnEvent::ReasoningDelta received"
                );
                match tokio::time::timeout(
                    Duration::from_secs(5),
                    tx.send(ChatEvent::ReasoningDelta { content }),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        chat_events_sent += 1;
                    }
                    Ok(Err(_)) => {
                        tracing::debug!(
                            "[agent] Failed to send ReasoningDelta to frontend (channel closed)"
                        );
                    }
                    Err(_) => {
                        tracing::warn!(
                            event_kind = "reasoningDelta",
                            "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                        );
                    }
                }
            }
            MultiTurnEvent::ToolCallStart { id, name } => {
                let tool_start = Instant::now();
                tool_start_times.insert(id.clone(), tool_start);
                tool_call_args.insert(id.clone(), String::new());
                tool_call_names.insert(id.clone(), name.clone());

                tracing::debug!(
                    request_id = %request_id,
                    tool_name = %name,
                    tool_id = %id,
                    "[agent] tool_call_start"
                );

                match tokio::time::timeout(
                    Duration::from_secs(5),
                    tx.send(ChatEvent::ToolCallStart {
                        id: id.clone(),
                        name: name.clone(),
                    }),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        chat_events_sent += 1;
                    }
                    Ok(Err(_)) => {
                        tracing::debug!(request_id = %request_id, "[agent] Failed to send ToolCallStart (channel closed)");
                    }
                    Err(_) => {
                        tracing::warn!(
                            event_kind = "toolCallStart",
                            "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                        );
                    }
                }
            }
            MultiTurnEvent::ToolCallDelta { id, args_delta } => {
                if let Some(args) = tool_call_args.get_mut(&id) {
                    args.push_str(&args_delta);
                }
                match tokio::time::timeout(
                    Duration::from_secs(5),
                    tx.send(ChatEvent::ToolCallDelta { id, args_delta }),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        chat_events_sent += 1;
                    }
                    Ok(Err(_)) => {
                        tracing::debug!(request_id = %request_id, "[agent] Failed to send ToolCallDelta (channel closed)");
                    }
                    Err(_) => {
                        tracing::warn!(
                            event_kind = "toolCallDelta",
                            "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                        );
                    }
                }
            }
            MultiTurnEvent::ToolCallEnd { id } => {
                // 把累积的 args 与 name 落盘到 tool_calls 记录
                if let (Some(name), Some(args)) = (tool_call_names.remove(&id), tool_call_args.remove(&id)) {
                    tool_calls.push(serde_json::json!({
                        "id": id,
                        "name": name,
                        "arguments": args,
                    }));
                }

                match tokio::time::timeout(
                    Duration::from_secs(5),
                    tx.send(ChatEvent::ToolCallEnd { id: id.clone() }),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        chat_events_sent += 1;
                    }
                    Ok(Err(_)) => {
                        tracing::debug!(request_id = %request_id, "[agent] Failed to send ToolCallEnd (channel closed)");
                    }
                    Err(_) => {
                        tracing::warn!(
                            event_kind = "toolCallEnd",
                            "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                        );
                    }
                }
            }
            MultiTurnEvent::ToolResult { id, name, output } => {
                let tool_duration = tool_start_times
                    .get(&id)
                    .map(|start| start.elapsed().as_millis() as u64)
                    .unwrap_or(0);

                tracing::debug!(
                    request_id = %request_id,
                    tool_id = %id,
                    tool_name = %name,
                    duration_ms = tool_duration,
                    "[agent] tool_result"
                );

                tool_results.push(serde_json::json!({
                    "id": id,
                    "name": name,
                    "output": output,
                }));
            }
            MultiTurnEvent::Finish { usage: u } => {
                usage = u;
                tracing::trace!(
                    input_tokens = usage.input_tokens,
                    output_tokens = usage.output_tokens,
                    "[agent] Finish token usage received"
                );
                tracing::debug!(
                    input_tokens = usage.input_tokens,
                    output_tokens = usage.output_tokens,
                    "[stream] MultiTurnEvent::Finish received"
                );
                // 立即转发 ChatEvent::Finish 到前端，确保前端能及时结束流式状态
                // （不等 run_agent_stream 返回后在 send_message.rs 中发送）
                match tokio::time::timeout(
                    Duration::from_secs(5),
                    tx.send(ChatEvent::Finish {
                        input_tokens: u.input_tokens,
                        output_tokens: u.output_tokens,
                    }),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        chat_events_sent += 1;
                        tracing::debug!("[stream] ChatEvent::Finish sent");
                    }
                    Ok(Err(_)) => {
                        tracing::debug!(
                            "[stream] Failed to send ChatEvent::Finish (channel closed)"
                        );
                    }
                    Err(_) => {
                        tracing::warn!(
                            event_kind = "finish",
                            "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                        );
                    }
                }
            }
            MultiTurnEvent::Error { message } => {
                tracing::error!(error = %message, "[agent] Stream error received");
                return Err(AppError::stream_error(message));
            }
        }
    }

    // Flush any remaining visible text from the thinking scrubber
    if let Some(remaining) = thinking_scrubber.flush() {
        if !remaining.is_empty() {
            tracing::trace!(
                remaining_len = remaining.len(),
                "[agent] flushing remaining visible text from scrubber"
            );
            match tokio::time::timeout(
                Duration::from_secs(5),
                tx.send(ChatEvent::TextDelta {
                    content: remaining.clone(),
                }),
            )
            .await
            {
                Ok(Ok(_)) => {
                    chat_events_sent += 1;
                }
                Ok(Err(_)) => {
                    tracing::debug!(
                        "[agent] Failed to send final TextDelta to frontend (channel closed)"
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        event_kind = "textDelta",
                        "[stream] tx.send(ChatEvent) timeout after 5s, skipping"
                    );
                }
            }
            text.push_str(&remaining);
        }
    }

    tracing::debug!(
        text_len = text.len(),
        tool_calls = tool_calls.len(),
        tool_results = tool_results.len(),
        input_tokens = usage.input_tokens,
        output_tokens = usage.output_tokens,
        "[stream] consume_stream: completed"
    );

    // 空响应回退：若全部内容被 thinking 标签包裹，scrubber 会剥离所有文本
    // 此时将 block_buffer 内容作为可见文本回退发送，避免前端显示空白
    if text.is_empty() && thinking_scrubber.is_in_block() {
        if let Some(block_content) = thinking_scrubber.take_block_buffer() {
            if !block_content.is_empty() {
                tracing::warn!(
                    block_buffer_len = block_content.len(),
                    "[stream] all content stripped by thinking scrubber, falling back to block_buffer"
                );
                match tokio::time::timeout(
                    Duration::from_secs(5),
                    tx.send(ChatEvent::TextDelta {
                        content: block_content.clone(),
                    }),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        chat_events_sent += 1;
                        tracing::debug!("[stream] fallback TextDelta sent");
                    }
                    Ok(Err(_)) => {
                        tracing::debug!("[stream] Failed to send fallback TextDelta (channel closed)");
                    }
                    Err(_) => {
                        tracing::warn!(
                            event_kind = "textDelta",
                            "[stream] tx.send(fallback TextDelta) timeout after 5s, skipping"
                        );
                    }
                }
                text.push_str(&block_content);
            }
        }
    }

    tracing::info!(
        events_received,
        chat_events_sent,
        text_len = text.len(),
        scrubber_in_block = thinking_scrubber.is_in_block(),
        "[stream] pipeline summary"
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::multi_turn::{MultiTurnEvent, MultiTurnStream};
    use tokio::sync::{mpsc, watch};

    /// 验证收到 MultiTurnEvent::Finish 时立即转发 ChatEvent::Finish 到前端
    #[tokio::test]
    async fn finish_event_forwarded_to_chat_event() {
        // 构造 MultiTurnEvent channel
        let (tx_mt, rx_mt) = mpsc::channel::<MultiTurnEvent>(8);
        let stream = MultiTurnStream::new(rx_mt);

        // 构造 ChatEvent channel
        let (tx_chat, mut rx_chat) = mpsc::channel::<ChatEvent>(8);

        // cancel channel（未取消）
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let _ = cancel_tx; // 保留 sender 避免 drop

        let request_id = uuid::Uuid::new_v4();

        // 发送 Finish 事件后关闭 MultiTurnEvent channel
        tx_mt
            .send(MultiTurnEvent::Finish {
                usage: TokenUsage {
                    input_tokens: 42,
                    output_tokens: 7,
                },
            })
            .await
            .unwrap();
        drop(tx_mt);

        // 运行 consume_stream_with_retry_detection
        let outcome = consume_stream_with_retry_detection(
            stream,
            &tx_chat,
            &mut cancel_rx,
            request_id,
        )
        .await
        .expect("consume_stream should succeed");

        // 验证 StreamOutcome.usage 正确更新
        assert_eq!(outcome.usage.input_tokens, 42);
        assert_eq!(outcome.usage.output_tokens, 7);

        // 验证 tx 收到 ChatEvent::Finish
        let event = rx_chat.recv().await.expect("should receive ChatEvent::Finish");
        match event {
            ChatEvent::Finish {
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(input_tokens, 42);
                assert_eq!(output_tokens, 7);
            }
            other => panic!("expected ChatEvent::Finish, got {:?}", other),
        }
    }

    /// 验证 TextDelta 事件被正确转发为 ChatEvent::TextDelta
    #[tokio::test]
    async fn text_delta_event_forwarded() {
        let (tx_mt, rx_mt) = mpsc::channel::<MultiTurnEvent>(8);
        let stream = MultiTurnStream::new(rx_mt);
        let (tx_chat, mut rx_chat) = mpsc::channel::<ChatEvent>(8);
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let _ = cancel_tx;
        let request_id = uuid::Uuid::new_v4();

        // 发送 TextDelta 后 Finish，然后关闭 channel
        tx_mt
            .send(MultiTurnEvent::TextDelta {
                content: "hello world".to_string(),
            })
            .await
            .unwrap();
        tx_mt
            .send(MultiTurnEvent::Finish {
                usage: TokenUsage::default(),
            })
            .await
            .unwrap();
        drop(tx_mt);

        let outcome = consume_stream_with_retry_detection(
            stream,
            &tx_chat,
            &mut cancel_rx,
            request_id,
        )
        .await
        .expect("consume_stream should succeed");

        // 关闭 tx_chat 使 rx_chat.recv() 在消费完缓冲后返回 None，结束循环
        drop(tx_chat);

        // 应收到至少一个 TextDelta 和一个 Finish
        let mut got_text = false;
        let mut got_finish = false;
        while let Some(event) = rx_chat.recv().await {
            match event {
                ChatEvent::TextDelta { content } => {
                    assert_eq!(content, "hello world");
                    got_text = true;
                }
                ChatEvent::Finish { .. } => {
                    got_finish = true;
                }
                _ => {}
            }
        }
        assert!(got_text, "should receive ChatEvent::TextDelta");
        assert!(got_finish, "should receive ChatEvent::Finish");
        assert_eq!(outcome.text, "hello world");
    }
}
