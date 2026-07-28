use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};

use crate::core::agent::memory::scrubber::MemoryContextScrubber;
use crate::infrastructure::llm::tool::{tools_to_specs, Tool};
use crate::infrastructure::llm::types::{
    FinishReason, Message, Provider, StreamEvent, TokenUsage, ToolCallRequest, ToolSpec,
};
use crate::shared::error::AppError;

/// MultiTurnRunner 向外发出的流式事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MultiTurnEvent {
    TextDelta { content: String },
    ReasoningDelta { content: String },
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, args_delta: String },
    ToolCallEnd { id: String },
    /// 工具执行完成（含输出或错误信息）
    ToolResult { id: String, name: String, output: String },
    /// 整个多轮循环结束，携带累计 token 用量
    Finish { usage: TokenUsage },
    /// 错误（max_turns / 工具执行异常 / 取消等）
    Error { message: String },
}

/// 多轮工具调用循环的运行结果（供调用方持久化）
#[derive(Debug, Clone, Default)]
pub struct MultiTurnOutcome {
    pub text: String,
    pub usage: TokenUsage,
    pub tool_calls: Vec<serde_json::Value>,
    pub tool_results: Vec<serde_json::Value>,
}

/// 多轮工具调用循环的运行器，替代 rig 的 `agent.stream_prompt().multi_turn(N)`。
pub struct MultiTurnRunner {
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    model: String,
    max_turns: usize,
    /// 单次 LLM 调用输出 token 上限（由 EffortLevel 决定，透传到 Provider.stream）
    max_tokens: u64,
}

impl MultiTurnRunner {
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        model: String,
        max_turns: usize,
        max_tokens: u64,
    ) -> Self {
        Self {
            provider,
            tools,
            model,
            max_turns,
            max_tokens,
        }
    }

    /// 启动多轮循环，返回事件流。
    ///
    /// 内部 spawn 一个 task 驱动循环，事件通过 mpsc channel 转发。
    /// 调用方消费 stream 时背压自动传到 channel。
    ///
    /// 错误事件统一由本方法的 spawned task 发送（基于 run_inner 返回的 `AppError.message`），
    /// 避免在 run_inner 内部多处发送造成重复。
    pub fn run(
        self,
        system: String,
        messages: Vec<Message>,
        cancel_rx: watch::Receiver<bool>,
    ) -> MultiTurnStream {
        let (tx, rx) = mpsc::channel::<MultiTurnEvent>(128);
        let provider = self.provider;
        let tools = self.tools;
        let model = self.model;
        let max_turns = self.max_turns;
        let max_tokens = self.max_tokens;

        tokio::spawn(async move {
            let outcome = Self::run_inner(
                &provider,
                &tools,
                &model,
                system,
                messages,
                max_turns,
                max_tokens,
                tx.clone(),
                cancel_rx,
            )
            .await;

            match outcome {
                Ok(_) => {
                    // run_inner 内部已发 Finish，无需再发
                }
                Err(e) => {
                    let _ = tx
                        .send(MultiTurnEvent::Error {
                            message: e.message,
                        })
                        .await;
                }
            }
        });

        MultiTurnStream::new(rx)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_inner(
        provider: &Arc<dyn Provider>,
        tools: &[Arc<dyn Tool>],
        model: &str,
        system: String,
        mut messages: Vec<Message>,
        max_turns: usize,
        max_tokens: u64,
        tx: mpsc::Sender<MultiTurnEvent>,
        mut cancel_rx: watch::Receiver<bool>,
    ) -> Result<MultiTurnOutcome, AppError> {
        let mut total_usage = TokenUsage::default();
        let mut accumulated_text = String::new();
        let mut all_tool_calls: Vec<serde_json::Value> = Vec::new();
        let mut all_tool_results: Vec<serde_json::Value> = Vec::new();

        // 首字节超时：流启动后 90s 内未收到任何事件视为卡死（大 prompt 首 token 延迟较高）
        const FIRST_BYTE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);
        // 流间隔超时：两个事件之间允许的最大间隔（30s），收到首事件后启用
        const STREAM_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

        // 预计算 ToolSpec 列表（每轮调用 provider.stream 时传入）
        let tool_specs: Vec<ToolSpec> = tools_to_specs(tools).await;

        for _turn in 0..max_turns {
            // 取消检查
            if *cancel_rx.borrow() {
                return Err(AppError::task_cancelled());
            }

            // 调用 provider.stream（透传 max_tokens 到 Provider，按 EffortLevel 控制输出上限）
            let stream = tokio::select! {
                r = provider.stream(model, &system, &messages, &tool_specs, max_tokens) => r?,
                _ = cancel_rx.changed() => return Err(AppError::task_cancelled()),
            };

            // 消费流，收集本轮 tool_calls
            let mut turn_text = String::new();
            let mut turn_tool_calls: Vec<ToolCallRequest> = Vec::new();
            // 用于累积每个 tool_call 的 args_delta（id → arguments 字符串）
            let mut tool_call_args: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            let mut turn_finish_reason: Option<FinishReason> = None;
            let mut turn_usage = TokenUsage::default();
            // memory-context scrubber：剥离模型复述的 <memory-context>...</memory-context> 块。
            // 每轮独立实例，避免跨轮状态污染。
            let mut memory_scrubber = MemoryContextScrubber::new();

            let mut stream = stream;

            // 流启动时刻，用于计算首字节超时截止时间
            let stream_start = std::time::Instant::now();
            // 最近一次收到事件的时间；None 表示尚未收到任何事件（启用首字节超时）
            let mut last_event_time: Option<std::time::Instant> = None;

            loop {
                // 计算当前轮次的超时截止时间：未收到首事件用首字节超时，否则用流间隔超时
                let deadline = match last_event_time {
                    None => stream_start + FIRST_BYTE_TIMEOUT,
                    Some(t) => t + STREAM_STALL_TIMEOUT,
                };
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());

                let next_event = tokio::select! {
                    ev = stream.next() => ev,
                    _ = cancel_rx.changed() => return Err(AppError::task_cancelled()),
                    _ = tokio::time::sleep(remaining) => {
                        // 截止时间已到，按是否曾收到事件区分两种超时
                        match last_event_time {
                            None => {
                                tracing::warn!(
                                    timeout_ms = FIRST_BYTE_TIMEOUT.as_millis() as u64,
                                    "[multi_turn] LLM stream first byte timeout"
                                );
                                return Err(AppError::stream_error(
                                    "LLM stream first byte timeout: no data for 90 seconds".to_string()
                                ));
                            }
                            Some(_) => {
                                tracing::warn!(
                                    timeout_ms = STREAM_STALL_TIMEOUT.as_millis() as u64,
                                    "[multi_turn] LLM stream stalled"
                                );
                                return Err(AppError::stream_error(
                                    "LLM stream stalled: no data for 30 seconds".to_string()
                                ));
                            }
                        }
                    }
                };

                let Some(event) = next_event else { break; };
                // 收到事件，刷新最近事件时间（后续轮次改用流间隔超时）
                last_event_time = Some(std::time::Instant::now());

                match event {
                    StreamEvent::TextDelta { content } => {
                        // 经 scrubber 剥离 memory-context 块后再累积/转发
                        if let Some(visible) = memory_scrubber.feed(&content) {
                            if !visible.is_empty() {
                                turn_text.push_str(&visible);
                                let _ = tx
                                    .send(MultiTurnEvent::TextDelta { content: visible })
                                    .await;
                            }
                        }
                    }
                    StreamEvent::ReasoningDelta { content } => {
                        let _ = tx
                            .send(MultiTurnEvent::ReasoningDelta { content })
                            .await;
                    }
                    StreamEvent::ToolCallStart { id, name } => {
                        turn_tool_calls.push(ToolCallRequest {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: String::new(),
                        });
                        tool_call_args.insert(id.clone(), String::new());
                        let _ = tx
                            .send(MultiTurnEvent::ToolCallStart { id, name })
                            .await;
                    }
                    StreamEvent::ToolCallDelta { id, args_delta } => {
                        if let Some(args) = tool_call_args.get_mut(&id) {
                            args.push_str(&args_delta);
                        }
                        let _ = tx
                            .send(MultiTurnEvent::ToolCallDelta { id, args_delta })
                            .await;
                    }
                    StreamEvent::ToolCallEnd { id } => {
                        let _ = tx.send(MultiTurnEvent::ToolCallEnd { id }).await;
                    }
                    StreamEvent::Finish { reason, usage } => {
                        turn_finish_reason = Some(reason);
                        turn_usage = usage;
                        // 不在这里发 Finish，等本轮决策（Stop/ToolCalls）后再决定
                    }
                    StreamEvent::Error(msg) => {
                        return Err(AppError::stream_error(msg));
                    }
                }
            }

            // 流结束（stream.next() 返回 None）
            tracing::debug!(
                turn_text_len = turn_text.len(),
                turn_tool_calls_count = turn_tool_calls.len(),
                finish_reason = ?turn_finish_reason,
                "[multi_turn] LLM stream ended"
            );

            // 流结束后 flush memory scrubber，输出缓冲区中的剩余可见内容
            // （未闭合的 <memory-context> 块会被丢弃，见 scrubber::flush 文档）
            if let Some(remaining) = memory_scrubber.flush() {
                if !remaining.is_empty() {
                    turn_text.push_str(&remaining);
                    let _ = tx
                        .send(MultiTurnEvent::TextDelta { content: remaining })
                        .await;
                }
            }

            // 累积本轮 text 和 usage
            accumulated_text.push_str(&turn_text);
            total_usage.input_tokens = total_usage
                .input_tokens
                .saturating_add(turn_usage.input_tokens);
            total_usage.output_tokens = total_usage
                .output_tokens
                .saturating_add(turn_usage.output_tokens);

            // 把累积的 args 填回 turn_tool_calls
            for tc in &mut turn_tool_calls {
                if let Some(args) = tool_call_args.remove(&tc.id) {
                    tc.arguments = args;
                }
            }

            // 记录 tool_calls 到 outcome
            for tc in &turn_tool_calls {
                all_tool_calls.push(serde_json::json!({
                    "id": tc.id,
                    "name": tc.name,
                    "arguments": tc.arguments,
                }));
            }

            match turn_finish_reason {
                Some(FinishReason::Stop) => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        tx.send(MultiTurnEvent::Finish {
                            usage: total_usage,
                        }),
                    ).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(_)) => {
                            tracing::warn!(
                                "[multi_turn] Failed to send Finish event (channel closed)"
                            );
                        }
                        Err(_) => {
                            tracing::warn!(
                                "[multi_turn] tx.send(Finish) timeout after 5s, returning without Finish"
                            );
                        }
                    }
                    return Ok(MultiTurnOutcome {
                        text: accumulated_text,
                        usage: total_usage,
                        tool_calls: all_tool_calls,
                        tool_results: all_tool_results,
                    });
                }
                None => {
                    // 流提前结束未发出 Finish 事件，可能是 API 错误等异常情况，
                    // 不能当作正常 Stop 终止，否则会掩盖 LLM 流提前结束的问题
                    tracing::warn!(
                        "[multi_turn] stream ended without Finish event, treating as error"
                    );
                    return Err(AppError::stream_error(
                        "Stream ended without Finish event".to_string(),
                    ));
                }
                Some(FinishReason::ToolCalls) => {
                    // 进入工具执行阶段
                    // 1. 把 assistant 消息（含 tool_calls）追加到 messages
                    let assistant_msg = Message {
                        role: "assistant".to_string(),
                        content: turn_text.clone(),
                        tool_calls: if turn_tool_calls.is_empty() {
                            None
                        } else {
                            Some(turn_tool_calls.clone())
                        },
                        tool_call_id: None,
                    };
                    messages.push(assistant_msg);

                    // 2. 对每个 tool_call 执行工具
                    for tc in &turn_tool_calls {
                        let tool_output = Self::execute_tool(tools, tc, &tx).await;

                        all_tool_results.push(serde_json::json!({
                            "id": tc.id,
                            "name": tc.name,
                            "output": tool_output,
                        }));

                        // 3. 把 tool result 消息追加到 messages
                        let tool_msg = Message {
                            role: "tool".to_string(),
                            content: tool_output,
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        };
                        messages.push(tool_msg);
                    }

                    // 继续下一轮 LLM 调用
                    continue;
                }
                Some(FinishReason::Length) | Some(FinishReason::ContentFilter) => {
                    // 长度限制或内容过滤，发 Finish 终止
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        tx.send(MultiTurnEvent::Finish {
                            usage: total_usage,
                        }),
                    ).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(_)) => {
                            tracing::warn!(
                                "[multi_turn] Failed to send Finish event (channel closed)"
                            );
                        }
                        Err(_) => {
                            tracing::warn!(
                                "[multi_turn] tx.send(Finish) timeout after 5s, returning without Finish"
                            );
                        }
                    }
                    return Ok(MultiTurnOutcome {
                        text: accumulated_text,
                        usage: total_usage,
                        tool_calls: all_tool_calls,
                        tool_results: all_tool_results,
                    });
                }
            }
        }

        // 达到 max_turns 仍收到 ToolCalls
        Err(AppError::stream_error(format!(
            "max_turns({}) reached",
            max_turns
        )))
    }

    /// 执行单个工具调用，返回输出字符串（错误时返回 "Error: ..."）。
    ///
    /// 无论成功或失败都会发送 `MultiTurnEvent::ToolResult`，循环不因工具失败而终止。
    async fn execute_tool(
        tools: &[Arc<dyn Tool>],
        tc: &ToolCallRequest,
        tx: &mpsc::Sender<MultiTurnEvent>,
    ) -> String {
        // 查找工具
        let tool = tools.iter().find(|t| t.name() == tc.name);

        let Some(tool) = tool else {
            let msg = format!("Tool '{}' not found", tc.name);
            let _ = tx
                .send(MultiTurnEvent::ToolResult {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    output: format!("Error: {}", msg),
                })
                .await;
            return format!("Error: {}", msg);
        };

        // 解析参数 JSON
        let args: serde_json::Value = if tc.arguments.is_empty() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            match serde_json::from_str(&tc.arguments) {
                Ok(v) => v,
                Err(e) => {
                    let msg = format!("Invalid args for '{}': {}", tc.name, e);
                    let _ = tx
                        .send(MultiTurnEvent::ToolResult {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            output: format!("Error: {}", msg),
                        })
                        .await;
                    return format!("Error: {}", msg);
                }
            }
        };

        // 调用工具
        match tool.call(args).await {
            Ok(output) => {
                let output_str = match output {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                let _ = tx
                    .send(MultiTurnEvent::ToolResult {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        output: output_str.clone(),
                    })
                    .await;
                output_str
            }
            Err(e) => {
                let msg = format!("Tool '{}' failed: {}", tc.name, e);
                let _ = tx
                    .send(MultiTurnEvent::ToolResult {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        output: format!("Error: {}", msg),
                    })
                    .await;
                format!("Error: {}", msg)
            }
        }
    }
}

/// 手动实现的 Stream，包装 mpsc::Receiver<MultiTurnEvent>。
pub struct MultiTurnStream {
    rx: mpsc::Receiver<MultiTurnEvent>,
}

impl MultiTurnStream {
    pub(crate) fn new(rx: mpsc::Receiver<MultiTurnEvent>) -> Self {
        Self { rx }
    }
}

impl Stream for MultiTurnStream {
    type Item = MultiTurnEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};
    use crate::infrastructure::llm::types::{FinishReason, Message, ModelInfo, Provider, StreamEvent};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::watch;

    /// Mock Provider：按预设的 StreamEvent 列表依次返回，每次 stream() 调用递增计数器。
    struct MockProvider {
        responses: Vec<Vec<StreamEvent>>,
        call_count: AtomicUsize,
    }

    impl MockProvider {
        fn new(responses: Vec<Vec<StreamEvent>>) -> Self {
            Self {
                responses,
                call_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }
        fn api_key(&self) -> &str {
            ""
        }
        fn base_url(&self) -> &str {
            ""
        }
        fn models(&self) -> Vec<ModelInfo> {
            vec![]
        }

        async fn complete(
            &self,
            _: &str,
            _: &str,
            _: &[Message],
            _max_tokens: u64,
        ) -> Result<String, AppError> {
            Ok("mock".to_string())
        }

        async fn stream(
            &self,
            _model: &str,
            _system: &str,
            _messages: &[Message],
            _tools: &[ToolSpec],
            _max_tokens: u64,
        ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, AppError> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            let events = self.responses.get(idx).cloned().unwrap_or_default();
            Ok(Box::pin(futures_util::stream::iter(events)))
        }

        async fn test_connection(&self) -> Result<(), AppError> {
            Ok(())
        }
    }

    /// 始终失败的工具
    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing_tool"
        }

        async fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "failing_tool".to_string(),
                description: "always fails".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            }
        }

        async fn call(
            &self,
            _args: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            Err(ToolError::Execution("intentional failure".to_string()))
        }
    }

    /// 收集 stream 所有事件
    async fn collect_events(stream: MultiTurnStream) -> Vec<MultiTurnEvent> {
        let mut events = Vec::new();
        let mut stream = stream;
        while let Some(ev) = stream.next().await {
            events.push(ev);
        }
        events
    }

    // 测试 1：单轮无工具调用（Finish Stop 即终止）
    #[tokio::test]
    async fn test_single_turn_no_tools() {
        let provider = Arc::new(MockProvider::new(vec![vec![
            StreamEvent::TextDelta {
                content: "Hello".to_string(),
            },
            StreamEvent::Finish {
                reason: FinishReason::Stop,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            },
        ]]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 5, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        assert!(
            events
                .iter()
                .any(|e| matches!(e, MultiTurnEvent::TextDelta { content } if content == "Hello")),
            "expected TextDelta(Hello), got {:?}",
            events
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                MultiTurnEvent::Finish { usage }
                if usage.input_tokens == 10 && usage.output_tokens == 5
            )),
            "expected Finish with usage{{10,5}}, got {:?}",
            events
        );
        // 不应有 Error 事件
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, MultiTurnEvent::Error { .. })),
            "should not have Error event, got {:?}",
            events
        );
    }

    // 测试 2：max_turns 达到上限（每轮都返回 ToolCalls 但工具不存在，循环继续直到 max_turns）
    #[tokio::test]
    async fn test_max_turns_reached() {
        let turn_response = vec![
            StreamEvent::ToolCallStart {
                id: "1".to_string(),
                name: "nonexistent".to_string(),
            },
            StreamEvent::ToolCallEnd {
                id: "1".to_string(),
            },
            StreamEvent::Finish {
                reason: FinishReason::ToolCalls,
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            },
        ];
        // max_turns=2，需要 2 个响应（每轮一个）
        let provider = Arc::new(MockProvider::new(vec![
            turn_response.clone(),
            turn_response,
        ]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 2, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        // 末尾应有 Error("max_turns(2) reached")
        let last = events.last();
        assert!(
            matches!(last, Some(MultiTurnEvent::Error { message }) if message.contains("max_turns(2) reached")),
            "expected last event Error(max_turns), got {:?}",
            last
        );
        // 应有 2 个 ToolResult（每轮一个，均为 "Tool not found"）
        let tool_results: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, MultiTurnEvent::ToolResult { .. }))
            .collect();
        assert_eq!(tool_results.len(), 2, "expected 2 ToolResult events");
        assert!(
            tool_results.iter().all(|e| matches!(
                e,
                MultiTurnEvent::ToolResult { output, .. } if output.contains("Error:")
            )),
            "all ToolResult should contain Error:, got {:?}",
            tool_results
        );
    }

    // 测试 3：工具失败后继续循环直到正常结束
    #[tokio::test]
    async fn test_tool_failure_then_stop() {
        let provider = Arc::new(MockProvider::new(vec![
            // turn 0: 调用 failing_tool
            vec![
                StreamEvent::ToolCallStart {
                    id: "1".to_string(),
                    name: "failing_tool".to_string(),
                },
                StreamEvent::ToolCallEnd {
                    id: "1".to_string(),
                },
                StreamEvent::Finish {
                    reason: FinishReason::ToolCalls,
                    usage: TokenUsage {
                        input_tokens: 1,
                        output_tokens: 1,
                    },
                },
            ],
            // turn 1: 正常结束
            vec![
                StreamEvent::TextDelta {
                    content: "done".to_string(),
                },
                StreamEvent::Finish {
                    reason: FinishReason::Stop,
                    usage: TokenUsage {
                        input_tokens: 2,
                        output_tokens: 2,
                    },
                },
            ],
        ]));

        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(FailingTool)];
        let runner = MultiTurnRunner::new(provider, tools, "mock-model".to_string(), 5, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        // 应有 ToolResult 包含 "Error:" 和 "intentional failure"
        let tool_result = events.iter().find(|e| {
            matches!(e, MultiTurnEvent::ToolResult { name, .. } if name == "failing_tool")
        });
        assert!(
            matches!(tool_result, Some(MultiTurnEvent::ToolResult { output, .. }) if output.contains("Error:") && output.contains("intentional failure")),
            "expected ToolResult with Error + intentional failure, got {:?}",
            tool_result
        );

        // 应有 TextDelta("done")
        assert!(
            events
                .iter()
                .any(|e| matches!(e, MultiTurnEvent::TextDelta { content } if content == "done")),
            "expected TextDelta(done), got {:?}",
            events
        );

        // 应有 Finish，累计 usage = {1+2, 1+2} = {3,3}
        let finish = events.iter().find(|e| matches!(e, MultiTurnEvent::Finish { .. }));
        assert!(
            matches!(finish, Some(MultiTurnEvent::Finish { usage }) if usage.input_tokens == 3 && usage.output_tokens == 3),
            "expected Finish with usage{{3,3}}, got {:?}",
            finish
        );

        // 不应有 Error 事件
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, MultiTurnEvent::Error { .. })),
            "should not have Error event, got {:?}",
            events
        );
    }

    // 测试 4：取消信号（在 stream 开始前发送 cancel，立即终止）
    #[tokio::test]
    async fn test_cancellation_before_start() {
        let provider = Arc::new(MockProvider::new(vec![vec![
            StreamEvent::TextDelta {
                content: "won't reach".to_string(),
            },
            StreamEvent::Finish {
                reason: FinishReason::Stop,
                usage: TokenUsage::default(),
            },
        ]]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 5, 8_192);
        let (cancel_tx, cancel_rx) = watch::channel(false);
        // 在 run 之前发送取消信号
        cancel_tx.send(true).unwrap();

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        // 应只有一个 Error 事件，消息为 "Task cancelled"
        assert_eq!(events.len(), 1, "expected exactly 1 event, got {:?}", events);
        assert!(
            matches!(&events[0], MultiTurnEvent::Error { message } if message == "Task cancelled"),
            "expected Error(Task cancelled), got {:?}",
            events[0]
        );
    }

    // 测试 5：流提前结束未发出 Finish 事件（应返回 Error 而非正常终止）
    // 复现 P0 Bug：turn_finish_reason == None 不能被当作正常 Stop 终止
    #[tokio::test]
    async fn test_stream_ended_without_finish() {
        let provider = Arc::new(MockProvider::new(vec![vec![
            StreamEvent::TextDelta {
                content: "partial".to_string(),
            },
            // 故意不发 Finish 事件，流直接结束（模拟 API 错误等异常情况）
        ]]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 5, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        // 应有 TextDelta("partial")，流消费过程中的事件正常透传
        assert!(
            events
                .iter()
                .any(|e| matches!(e, MultiTurnEvent::TextDelta { content } if content == "partial")),
            "expected TextDelta(partial), got {:?}",
            events
        );

        // 不应有 Finish 事件，因为流未正常结束
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, MultiTurnEvent::Finish { .. })),
            "should not have Finish event, got {:?}",
            events
        );

        // 应有 Error 事件，消息包含 "Stream ended without Finish event"
        let error = events
            .iter()
            .find(|e| matches!(e, MultiTurnEvent::Error { .. }));
        assert!(
            matches!(error, Some(MultiTurnEvent::Error { message }) if message.contains("Stream ended without Finish event")),
            "expected Error with 'Stream ended without Finish event', got {:?}",
            error
        );
    }

    // 测试 6：memory-context 块在单个 TextDelta 中应被剥离
    #[tokio::test]
    async fn test_memory_context_block_stripped_single_chunk() {
        let provider = Arc::new(MockProvider::new(vec![vec![
            StreamEvent::TextDelta {
                content: "before<memory-context>secret memory data</memory-context>after".to_string(),
            },
            StreamEvent::Finish {
                reason: FinishReason::Stop,
                usage: TokenUsage::default(),
            },
        ]]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 5, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        // 收集所有 TextDelta 的 content 拼接
        let visible: String = events
            .iter()
            .filter_map(|e| {
                if let MultiTurnEvent::TextDelta { content } = e {
                    Some(content.clone())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(
            visible, "beforeafter",
            "memory-context 块应被剥离，got: {}",
            visible
        );
        assert!(
            !visible.contains("secret memory data"),
            "块内内容不应出现在可见输出"
        );
    }

    // 测试 7：memory-context 块跨多个 TextDelta chunk 应被剥离
    #[tokio::test]
    async fn test_memory_context_block_split_across_chunks() {
        let provider = Arc::new(MockProvider::new(vec![vec![
            StreamEvent::TextDelta {
                content: "Hello <memory-con".to_string(),
            },
            StreamEvent::TextDelta {
                content: "text>internal secret".to_string(),
            },
            StreamEvent::TextDelta {
                content: "</memory-context> world".to_string(),
            },
            StreamEvent::Finish {
                reason: FinishReason::Stop,
                usage: TokenUsage::default(),
            },
        ]]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 5, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        let visible: String = events
            .iter()
            .filter_map(|e| {
                if let MultiTurnEvent::TextDelta { content } = e {
                    Some(content.clone())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(
            visible, "Hello  world",
            "跨 chunk 的 memory-context 块应被剥离，got: '{}'",
            visible
        );
        assert!(
            !visible.contains("secret"),
            "块内内容不应出现在可见输出"
        );
    }

    // 测试 8：无 memory-context 标签时正常透传
    #[tokio::test]
    async fn test_no_memory_context_tag_passes_through() {
        let provider = Arc::new(MockProvider::new(vec![vec![
            StreamEvent::TextDelta {
                content: "正常文本输出".to_string(),
            },
            StreamEvent::Finish {
                reason: FinishReason::Stop,
                usage: TokenUsage::default(),
            },
        ]]));

        let runner = MultiTurnRunner::new(provider, vec![], "mock-model".to_string(), 5, 8_192);
        let (_cancel_tx, cancel_rx) = watch::channel(false);

        let stream = runner.run("system".to_string(), vec![], cancel_rx);
        let events = collect_events(stream).await;

        let visible: String = events
            .iter()
            .filter_map(|e| {
                if let MultiTurnEvent::TextDelta { content } = e {
                    Some(content.clone())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(visible, "正常文本输出");
    }
}
