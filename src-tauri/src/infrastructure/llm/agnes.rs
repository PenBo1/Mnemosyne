//! ═══════════════════════════════════════════════════════════════════════════
//! Agnes Provider - Agnes API 实现
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 实现 Agnes API 调用（OpenAI 兼容协议）：
//! - 默认端点 https://apihub.agnes-ai.com/v1
//! - 支持 Agnes 2.0 Flash 等模型

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::collections::HashSet;
use super::types::*;
use super::openai_protocol;
use crate::shared::error::AppError;

/// Agnes 流式解析状态
///
/// 跨 chunk 维护 SSE 行缓冲与已开始的 tool_call id 集合。
/// `tool_call_ids` 用于在收到 `finish_reason == "tool_calls"` 时
/// 对每个已开始的 tool_call 发出 `StreamEvent::ToolCallEnd`。
#[derive(Default)]
struct AgnesStreamState {
    sse_buffer: super::sse_buffer::SseLineBuffer,
    tool_call_ids: HashSet<String>,
}

/// 解析单行 SSE 数据，返回产生的 StreamEvent 列表。
///
/// 该函数为 `stream()` 的可测试提取：传入一行 SSE 数据与可变状态，
/// 解析 Agnes（OpenAI 兼容）chat completion 流式协议并产出事件。
/// 收到 `finish_reason == "tool_calls"` 时，会对 `state.tool_call_ids`
/// 中所有已开始的 tool_call 发出 `StreamEvent::ToolCallEnd`，然后清空集合。
///
/// usage 读取路径：优先从 `choice.get("usage")` 读取（OpenAI 协议位置），
/// fallback 到 `json.get("usage")`（部分 Agnes 响应把 usage 放在顶层）。
fn parse_agnes_sse_line(line: &str, state: &mut AgnesStreamState) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    let line = line.trim();
    if line.is_empty() || !line.starts_with("data: ") {
        return events;
    }
    let data = &line[6..];
    if data == "[DONE]" {
        return events;
    }
    let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
        return events;
    };
    if let Some(choices) = json["choices"].as_array() {
        for choice in choices {
            if let Some(delta) = choice.get("delta") {
                if let Some(content) = delta["content"].as_str() {
                    if !content.is_empty() {
                        events.push(StreamEvent::TextDelta { content: content.to_string() });
                    }
                }
                if let Some(reasoning) = delta["reasoning_content"].as_str() {
                    if !reasoning.is_empty() {
                        events.push(StreamEvent::ReasoningDelta { content: reasoning.to_string() });
                    }
                }
                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                    for tc in tool_calls {
                        let id = tc["id"].as_str().unwrap_or("");
                        let name = tc["function"]["name"].as_str().unwrap_or("");
                        let args = tc["function"]["arguments"].as_str().unwrap_or("");
                        if !id.is_empty() && !name.is_empty() {
                            state.tool_call_ids.insert(id.to_string());
                            events.push(StreamEvent::ToolCallStart { id: id.to_string(), name: name.to_string() });
                        }
                        if !args.is_empty() {
                            events.push(StreamEvent::ToolCallDelta { id: id.to_string(), args_delta: args.to_string() });
                        }
                    }
                }
            }
            if let Some(finish) = choice["finish_reason"].as_str() {
                let reason = match finish {
                    "tool_calls" => {
                        // 对所有已开始的 tool_call 发出 End 事件
                        for id in state.tool_call_ids.drain() {
                            events.push(StreamEvent::ToolCallEnd { id });
                        }
                        FinishReason::ToolCalls
                    }
                    "length" => FinishReason::Length,
                    _ => FinishReason::Stop,
                };
                // usage 优先从 choice 内读取，fallback 到顶层 json
                let usage_source = choice.get("usage").or_else(|| json.get("usage"));
                let mut usage = TokenUsage::default();
                if let Some(u) = usage_source {
                    if let Some(pt) = u.get("prompt_tokens").and_then(|v| v.as_u64()) {
                        usage.input_tokens = pt as u32;
                    }
                    if let Some(ct) = u.get("completion_tokens").and_then(|v| v.as_u64()) {
                        usage.output_tokens = ct as u32;
                    }
                }
                events.push(StreamEvent::Finish { reason, usage });
            }
        }
    }
    events
}

pub struct AgnesProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AgnesProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "https://apihub.agnes-ai.com/v1".to_string());
        tracing::debug!(base_url = %url, "AgnesProvider created");
        Self {
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(30))
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("failed to build reqwest client for AgnesProvider"),
            api_key,
            base_url: url,
        }
    }
}

#[async_trait]
impl Provider for AgnesProvider {
    fn name(&self) -> &str { "agnes" }
    fn api_key(&self) -> &str { &self.api_key }
    fn base_url(&self) -> &str { &self.base_url }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo { id: "agnes-2.0-flash".into(), provider: "agnes".into(), name: "Agnes 2.0 Flash".into(), context_window: 256000, supports_tools: true, supports_streaming: true },
            ModelInfo { id: "agnes-1.5-flash".into(), provider: "agnes".into(), name: "Agnes 1.5 Flash".into(), context_window: 256000, supports_tools: true, supports_streaming: true },
        ]
    }

    async fn complete(&self, model: &str, system: &str, messages: &[Message], max_tokens: u64) -> Result<String, AppError> {
        tracing::info!(model = %model, messages = messages.len(), max_tokens, "Agnes complete request");
        let body = openai_protocol::build_request(model, system, messages, &[], false, max_tokens);
        let resp = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "Agnes request failed");
                AppError::stream_error(e.to_string())
            })?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await
            .map_err(|e| {
                tracing::error!(error = %e, "Agnes response parse failed");
                AppError::invalid_format(e.to_string())
            })?;
        if !status.is_success() {
            let msg = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(AppError::internal(format!("API {}: {}", status, msg)));
        }
        json["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
            .ok_or_else(|| {
                tracing::error!("No content in Agnes response");
                AppError::internal("No content in Agnes response")
            })
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec], max_tokens: u64) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>, AppError> {
        let start = std::time::Instant::now();
        tracing::info!(model = %model, messages = messages.len(), tools = tools.len(), max_tokens, "agnes_stream: enter");

        let result = async {
            let body = openai_protocol::build_request(model, system, messages, tools, true, max_tokens);
            self.client.post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body).send().await
                .map_err(|e| {
                    tracing::error!(error = %e, "Agnes stream request failed");
                    AppError::stream_error(e.to_string())
                })
        }.await;

        match &result {
            Ok(resp) => {
                let content_type = resp.headers().get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                tracing::info!(
                    model = %model,
                    status = %resp.status(),
                    content_type = %content_type,
                    duration_ms = start.elapsed().as_millis(),
                    "agnes_stream: exit (stream started)"
                );
            }
            Err(e) => tracing::error!(
                model = %model,
                error = %e,
                duration_ms = start.elapsed().as_millis(),
                "agnes_stream: error"
            ),
        }

        let resp = result?;

        // 非 2xx 响应必须在此处拦截：错误 body 不能交给 bytes_stream，
        // 否则会被当作空流，前端收到空回复（P0 bug 根因）。
        let http_status = resp.status();
        if !http_status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %http_status, body = %body, "Agnes stream non-2xx response");
            return Err(super::map_stream_http_error(http_status, &body));
        }

        // Content-Type 校验：必须是 text/event-stream 或 application/x-ndjson。
        // 非 SSE 响应（如 HTML 错误页、JSON 错误、代理拦截页）必须在此拦截，
        // 否则 bytes_stream 会沉默消费，前端无任何流事件（P0 bug 根因）。
        // 注意：必须把 content_type 拷贝成 String，否则借用 resp 后无法再调 resp.text()。
        let content_type: String = resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let ct_lower = content_type.to_lowercase();
        let is_sse = ct_lower.starts_with("text/event-stream") || ct_lower.starts_with("application/x-ndjson");
        if !is_sse {
            let body = resp.text().await.unwrap_or_default();
            let preview: String = body.chars().take(200).collect();
            tracing::error!(
                status = %http_status,
                content_type = %content_type,
                body = %body,
                "Agnes stream response is not text/event-stream"
            );
            return Err(AppError::stream_error(format!(
                "Agnes stream response is not text/event-stream: got {}, body preview: {}",
                content_type, preview
            )));
        }

        let byte_stream = resp.bytes_stream();
        let event_stream = byte_stream
            .scan(AgnesStreamState::default(), |state, chunk| {
                let events: Vec<StreamEvent> = match chunk {
                    Ok(bytes) => {
                        let chunk_len = bytes.as_ref().len();
                        let lines = state.sse_buffer.push(bytes.as_ref());
                        let lines_count = lines.len();
                        let mut events = Vec::new();
                        for line in lines {
                            events.extend(parse_agnes_sse_line(&line, state));
                        }
                        let events_count = events.len();
                        tracing::trace!(
                            chunk_len,
                            lines_count,
                            events_count,
                            "agnes_stream: chunk processed"
                        );
                        events
                    }
                    Err(e) => vec![StreamEvent::Error(e.to_string())],
                };
                futures_util::future::ready(Some(futures_util::stream::iter(events)))
            })
            .flatten();
        Ok(Box::pin(event_stream))
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        let start = std::time::Instant::now();
        tracing::info!(base_url = %self.base_url, "agnes_test_connection: enter");

        let result = async {
            let body = serde_json::json!({
                "model": "agnes-2.0-flash",
                "messages": [{ "role": "user", "content": "hi" }],
                "max_tokens": 1,
            });
            let resp = self.client.post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body)
                .send().await
                .map_err(|e| {
                    tracing::error!(error = %e, "Agnes connection failed");
                    AppError::connection_refused(self.base_url.clone())
                })?;

            if resp.status().is_success() {
                Ok(())
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::error!(status = %status, body = %body, "Agnes connection test failed");
                Err(AppError::provider_unavailable("agnes"))
            }
        }.await;

        match &result {
            Ok(()) => tracing::info!(
                duration_ms = start.elapsed().as_millis(),
                "agnes_test_connection: exit (success)"
            ),
            Err(e) => tracing::error!(
                error = %e,
                duration_ms = start.elapsed().as_millis(),
                "agnes_test_connection: error"
            ),
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_delta() {
        let mut state = AgnesStreamState::default();
        let line = r#"data: {"choices":[{"delta":{"content":"hello"}}]}"#;
        let events = parse_agnes_sse_line(line, &mut state);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::TextDelta { content } => assert_eq!(content, "hello"),
            other => panic!("expected TextDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_reasoning_delta() {
        let mut state = AgnesStreamState::default();
        let line = r#"data: {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}"#;
        let events = parse_agnes_sse_line(line, &mut state);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::ReasoningDelta { content } => assert_eq!(content, "thinking..."),
            other => panic!("expected ReasoningDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_tool_call_complete() {
        let mut state = AgnesStreamState::default();
        // 模拟 Agnes（OpenAI 兼容）工具调用流式序列：
        // 1. 首个 delta 携带 id + name（arguments 为空字符串）
        // 2. 后续 delta 仅携带 arguments 分片（无 id/name）
        // 3. 末尾 delta 携带 finish_reason="tool_calls"
        let lines = vec![
            r#"data: {"choices":[{"delta":{"tool_calls":[{"id":"call_abc","function":{"name":"get_weather","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"function":{"arguments":"{\"loc"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"function":{"arguments":"ation\":\"SF\"}"}}]}}]}"#,
            r#"data: {"choices":[{"finish_reason":"tool_calls","index":0}]}"#,
        ];
        let mut all_events = Vec::new();
        for line in lines {
            all_events.extend(parse_agnes_sse_line(line, &mut state));
        }

        // 验证事件序列：ToolCallStart → ToolCallDelta ×2 → ToolCallEnd → Finish(ToolCalls)
        assert!(all_events.iter().any(|e| matches!(
            e,
            StreamEvent::ToolCallStart { id, name } if id == "call_abc" && name == "get_weather"
        )), "expected ToolCallStart with id=call_abc and name=get_weather, got {:?}", all_events);
        assert!(all_events.iter().any(|e| matches!(
            e,
            StreamEvent::ToolCallEnd { id } if id == "call_abc"
        )), "expected ToolCallEnd with id=call_abc, got {:?}", all_events);
        assert!(all_events.iter().any(|e| matches!(
            e,
            StreamEvent::Finish { reason: FinishReason::ToolCalls, .. }
        )), "expected Finish with reason=ToolCalls, got {:?}", all_events);

        // 验证 ToolCallEnd 在 Finish 之前出现
        let end_idx = all_events.iter().position(|e| matches!(
            e,
            StreamEvent::ToolCallEnd { id } if id == "call_abc"
        ));
        let finish_idx = all_events.iter().position(|e| matches!(
            e,
            StreamEvent::Finish { reason: FinishReason::ToolCalls, .. }
        ));
        match (end_idx, finish_idx) {
            (Some(ei), Some(fi)) => assert!(ei < fi, "ToolCallEnd must come before Finish"),
            _ => panic!("missing ToolCallEnd or Finish event"),
        }

        // 验证集合在 Finish 后已清空（再发一个 finish 不会重复发 End）
        let extra_events = parse_agnes_sse_line(
            r#"data: {"choices":[{"finish_reason":"stop","index":0}]}"#,
            &mut state,
        );
        assert!(!extra_events.iter().any(|e| matches!(e, StreamEvent::ToolCallEnd { .. })),
            "tool_call_ids should be drained after first finish_reason=tool_calls");
    }

    #[test]
    fn parse_parallel_tool_calls_both_ended() {
        // 并行工具调用：多个 tool_call 在同一 delta 中开始，
        // 单个 finish_reason=tool_calls 应该对所有已开始的 id 发出 End
        let mut state = AgnesStreamState::default();
        let lines = vec![
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","function":{"name":"f1","arguments":""}},{"index":1,"id":"call_b","function":{"name":"f2","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"finish_reason":"tool_calls","index":0}]}"#,
        ];
        let mut all_events = Vec::new();
        for line in lines {
            all_events.extend(parse_agnes_sse_line(line, &mut state));
        }
        let end_ids: Vec<String> = all_events.iter().filter_map(|e| {
            if let StreamEvent::ToolCallEnd { id } = e { Some(id.clone()) } else { None }
        }).collect();
        assert_eq!(end_ids.len(), 2, "both parallel tool_calls should receive End, got {:?}", end_ids);
        assert!(end_ids.contains(&"call_a".to_string()));
        assert!(end_ids.contains(&"call_b".to_string()));
    }

    #[test]
    fn parse_finish_stop_with_usage() {
        let mut state = AgnesStreamState::default();
        let line = r#"data: {"choices":[{"finish_reason":"stop","index":0,"usage":{"prompt_tokens":42,"completion_tokens":7}}]}"#;
        let events = parse_agnes_sse_line(line, &mut state);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::Finish { reason, usage } => {
                assert!(matches!(reason, FinishReason::Stop));
                assert_eq!(usage.input_tokens, 42);
                assert_eq!(usage.output_tokens, 7);
            }
            other => panic!("expected Finish, got {:?}", other),
        }
    }

    #[test]
    fn parse_done_marker_ignored() {
        let mut state = AgnesStreamState::default();
        let events = parse_agnes_sse_line("data: [DONE]", &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_empty() {
        let mut state = AgnesStreamState::default();
        let events = parse_agnes_sse_line("data: not json", &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_non_data_line_ignored() {
        let mut state = AgnesStreamState::default();
        // SSE 事件行（event:）和注释行（:）应被忽略
        let events = parse_agnes_sse_line("event: message", &mut state);
        assert!(events.is_empty());
        let events = parse_agnes_sse_line(": comment", &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_empty_content_ignored() {
        let mut state = AgnesStreamState::default();
        // 经常发送 content="" 的首 delta，不应产生事件
        let line = r#"data: {"choices":[{"delta":{"content":""}}]}"#;
        let events = parse_agnes_sse_line(line, &mut state);
        assert!(events.is_empty());
    }
}
