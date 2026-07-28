//! ═══════════════════════════════════════════════════════════════════════════
//! OpenAI Provider - OpenAI API 实现
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 实现 OpenAI Chat Completions API 调用：
//! - 非流式完成（complete）
//! - 流式完成（stream）
//! - 工具调用（complete_with_tools）

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::collections::HashSet;
use super::types::*;
use super::openai_protocol;
use crate::shared::error::AppError;

/// OpenAI 流式解析状态
///
/// 跨 chunk 维护 SSE 行缓冲与已开始的 tool_call id 集合。
/// `tool_call_ids` 用于在收到 `finish_reason == "tool_calls"` 时
/// 对每个已开始的 tool_call 发出 `StreamEvent::ToolCallEnd`。
#[derive(Default)]
struct OpenAiStreamState {
    sse_buffer: super::sse_buffer::SseLineBuffer,
    /// 已开始的 tool_call id 集合，待 finish_reason=tool_calls 时发 End
    tool_call_ids: HashSet<String>,
}


/// 解析单行 SSE 数据，返回产生的 StreamEvent 列表。
///
/// 该函数为 `stream()` 的可测试提取：传入一行 SSE 数据与可变状态，
/// 解析 OpenAI chat completion 流式协议并产出事件。
/// 收到 `finish_reason == "tool_calls"` 时，会对 `state.tool_call_ids`
/// 中所有已开始的 tool_call 发出 `StreamEvent::ToolCallEnd`，然后清空集合。
fn parse_openai_sse_line(line: &str, state: &mut OpenAiStreamState) -> Vec<StreamEvent> {
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
                let mut usage = TokenUsage::default();
                if let Some(u) = choice.get("usage") {
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

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(30))
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("failed to build reqwest client for OpenAiProvider"),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }
    fn api_key(&self) -> &str { &self.api_key }
    fn base_url(&self) -> &str { &self.base_url }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo { id: "gpt-4o".into(), provider: "openai".into(), name: "GPT-4o".into(), context_window: 128000, supports_tools: true, supports_streaming: true },
            ModelInfo { id: "gpt-4o-mini".into(), provider: "openai".into(), name: "GPT-4o Mini".into(), context_window: 128000, supports_tools: true, supports_streaming: true },
            ModelInfo { id: "gpt-4.1".into(), provider: "openai".into(), name: "GPT-4.1".into(), context_window: 1047576, supports_tools: true, supports_streaming: true },
        ]
    }

    async fn complete(&self, model: &str, system: &str, messages: &[Message], max_tokens: u64) -> Result<String, AppError> {
        let start = std::time::Instant::now();
        tracing::info!(model = %model, messages = messages.len(), max_tokens, "openai_complete: enter");

        let result = async {
            let body = openai_protocol::build_request(model, system, messages, &[], false, max_tokens);
            let resp = self.client.post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body).send().await
                .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;
            let status = resp.status();
            let json: serde_json::Value = resp.json().await
                .map_err(|e| AppError::internal(format!("Response parse failed: {}", e)))?;
            if !status.is_success() {
                let msg = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(AppError::internal(format!("API {}: {}", status, msg)));
            }
            json["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
                .ok_or_else(|| AppError::internal("No content in response"))
        }.await;
        
        match &result {
            Ok(_) => tracing::info!(
                model = %model,
                duration_ms = start.elapsed().as_millis(),
                "openai_complete: exit"
            ),
            Err(e) => tracing::error!(
                model = %model,
                error = %e,
                duration_ms = start.elapsed().as_millis(),
                "openai_complete: error"
            ),
        }
        result
    }

    /// 非流式带工具调用完成
    ///
    /// 返回值是 `choices[0].message` 的 JSON 字符串（含 `content` 和 `tool_calls` 字段），
    /// 由调用方决定如何解析。这样既支持纯文本响应，也支持工具调用响应。
    async fn complete_with_tools(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
        max_tokens: u64,
    ) -> Result<String, AppError> {
        let start = std::time::Instant::now();
        tracing::info!(model = %model, messages = messages.len(), tools = tools.len(), max_tokens, "openai_complete_with_tools: enter");

        let result = async {
            let body = openai_protocol::build_request(model, system, messages, tools, false, max_tokens);
            let resp = self.client.post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body).send().await
                .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;
            let status = resp.status();
            let json: serde_json::Value = resp.json().await
                .map_err(|e| AppError::internal(format!("Response parse failed: {}", e)))?;
            if !status.is_success() {
                let msg = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(AppError::internal(format!("API {}: {}", status, msg)));
            }
            // 返回完整 message JSON（含 content + tool_calls），由调用方解析
            serde_json::to_string(&json["choices"][0]["message"])
                .map_err(|e| AppError::internal(format!("Serialize response failed: {}", e)))
        }.await;

        match &result {
            Ok(_) => tracing::info!(
                model = %model,
                duration_ms = start.elapsed().as_millis(),
                "openai_complete_with_tools: exit"
            ),
            Err(e) => tracing::error!(
                model = %model,
                error = %e,
                duration_ms = start.elapsed().as_millis(),
                "openai_complete_with_tools: error"
            ),
        }
        result
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec], max_tokens: u64) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>, AppError> {
        tracing::info!(model = %model, messages = messages.len(), tools = tools.len(), max_tokens, "OpenAI stream request");
        let body = openai_protocol::build_request(model, system, messages, tools, true, max_tokens);
        let resp = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "OpenAI stream request failed");
                AppError::stream_error(e.to_string())
            })?;
        tracing::info!(status = %resp.status(), "OpenAI stream response received");

        // 非 2xx 响应必须在此处拦截：错误 body 不能交给 bytes_stream，
        // 否则会被当作空流，前端收到空回复（P0 bug 根因）。
        let http_status = resp.status();
        if !http_status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %http_status, body = %body, "OpenAI stream non-2xx response");
            return Err(super::map_stream_http_error(http_status, &body));
        }

        let byte_stream = resp.bytes_stream();
        let event_stream = byte_stream
            .scan(OpenAiStreamState::default(), |state, chunk| {
                let events: Vec<StreamEvent> = match chunk {
                    Ok(bytes) => {
                        let lines = state.sse_buffer.push(bytes.as_ref());
                        let mut events = Vec::new();
                        for line in lines {
                            events.extend(parse_openai_sse_line(&line, state));
                        }
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
        let resp = self.client.get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send().await
            .map_err(|e| AppError::internal(format!("Connection failed: {}", e)))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(AppError::internal(format!("API returned {}: {}", status, body)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_delta() {
        let mut state = OpenAiStreamState::default();
        let line = r#"data: {"choices":[{"delta":{"content":"hello"}}]}"#;
        let events = parse_openai_sse_line(line, &mut state);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::TextDelta { content } => assert_eq!(content, "hello"),
            other => panic!("expected TextDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_tool_call_complete() {
        let mut state = OpenAiStreamState::default();
        // 模拟 OpenAI 工具调用流式序列：
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
            all_events.extend(parse_openai_sse_line(line, &mut state));
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
        let extra_events = parse_openai_sse_line(
            r#"data: {"choices":[{"finish_reason":"stop","index":0}]}"#,
            &mut state,
        );
        assert!(!extra_events.iter().any(|e| matches!(e, StreamEvent::ToolCallEnd { .. })),
            "tool_call_ids should be drained after first finish_reason=tool_calls");
    }

    #[test]
    fn parse_done_marker_ignored() {
        let mut state = OpenAiStreamState::default();
        let events = parse_openai_sse_line("data: [DONE]", &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_empty() {
        let mut state = OpenAiStreamState::default();
        let events = parse_openai_sse_line("data: not json", &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_non_data_line_ignored() {
        let mut state = OpenAiStreamState::default();
        // SSE 事件行（event:）和注释行（:）应被忽略
        let events = parse_openai_sse_line("event: message", &mut state);
        assert!(events.is_empty());
        let events = parse_openai_sse_line(": comment", &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_reasoning_delta() {
        let mut state = OpenAiStreamState::default();
        let line = r#"data: {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}"#;
        let events = parse_openai_sse_line(line, &mut state);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::ReasoningDelta { content } => assert_eq!(content, "thinking..."),
            other => panic!("expected ReasoningDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_finish_stop_with_usage() {
        let mut state = OpenAiStreamState::default();
        let line = r#"data: {"choices":[{"finish_reason":"stop","index":0,"usage":{"prompt_tokens":42,"completion_tokens":7}}]}"#;
        let events = parse_openai_sse_line(line, &mut state);
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
    fn parse_empty_content_ignored() {
        let mut state = OpenAiStreamState::default();
        // OpenAI 经常发送 content="" 的首 delta，不应产生事件
        let line = r#"data: {"choices":[{"delta":{"content":""}}]}"#;
        let events = parse_openai_sse_line(line, &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_parallel_tool_calls_both_ended() {
        // OpenAI 并行工具调用：多个 tool_call 在同一 delta 中开始，
        // 单个 finish_reason=tool_calls 应该对所有已开始的 id 发出 End
        let mut state = OpenAiStreamState::default();
        let lines = vec![
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","function":{"name":"f1","arguments":""}},{"index":1,"id":"call_b","function":{"name":"f2","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"finish_reason":"tool_calls","index":0}]}"#,
        ];
        let mut all_events = Vec::new();
        for line in lines {
            all_events.extend(parse_openai_sse_line(line, &mut state));
        }
        let end_ids: Vec<String> = all_events.iter().filter_map(|e| {
            if let StreamEvent::ToolCallEnd { id } = e { Some(id.clone()) } else { None }
        }).collect();
        assert_eq!(end_ids.len(), 2, "both parallel tool_calls should receive End, got {:?}", end_ids);
        assert!(end_ids.contains(&"call_a".to_string()));
        assert!(end_ids.contains(&"call_b".to_string()));
    }
}