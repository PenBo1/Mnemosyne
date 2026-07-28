//! ═══════════════════════════════════════════════════════════════════════════
//! Anthropic Provider - Claude API 实现
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 实现 Anthropic Claude API 调用：
//! - 支持 Claude 3.5/4 系列模型
//! - 支持工具调用
//! - 支持 Prompt Caching

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::collections::HashMap;
use super::types::*;
use crate::shared::error::AppError;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(30))
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("failed to build reqwest client for AnthropicProvider"),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
        }
    }

    fn build_request(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec], stream: bool, max_tokens: u64) -> serde_json::Value {
        let mut msgs = Vec::new();
        for m in messages {
            if m.role == "system" {
                continue;
            }
            msgs.push(serde_json::json!({
                "role": m.role,
                "content": m.content
            }));
        }

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": msgs,
            "stream": stream,
        });

        if !system.is_empty() && !messages.iter().any(|m| m.role == "system") {
            body["system"] = serde_json::Value::String(system.to_string());
        }

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(
                tools.iter().map(|t| serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })).collect::<Vec<_>>()
            );
        }

        // 自动应用 Anthropic prompt cache（5 分钟 TTL，无需 beta header）
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        body
    }
}

// ── Anthropic prompt caching（Task D4）─────────────────────────
//
// 对照 Anthropic Prompt Caching 文档：在请求 body 上注入最多 4 个
// `cache_control` 断点（1 system 末尾 + 3 最后非 system 消息末尾），
// 让前缀稳定的部分被缓存，降低重复 token 计费与延迟。
//
// 与 codex 的差异：codex 走 OpenAI Responses API 的 `prompt_cache_key`
// （在 client.rs 中由 server 端隐式缓存），Anthropic 走显式
// `cache_control` 断点，因此需要本模块。

/// Anthropic prompt cache 断点数量上限。
///
/// 对照 Anthropic 文档：最多 4 个 `cache_control` 断点
/// （1 system + 3 messages）。
pub const MAX_CACHE_BREAKPOINTS: usize = 4;

/// 在请求 body 上注入 Anthropic prompt cache 断点。
///
/// 对照 Anthropic Prompt Caching 文档：
/// - 最多 4 个 `cache_control` 断点
/// - 第 1 个：`system` 末尾（system 转为 block array 形式）
/// - 后 3 个：最后 3 个非 system 消息的 content 末尾
///   （content 从 string 转为 block array 形式）
///
/// 幂等：重复调用不会叠加 cache_control，也不会在已缓存的消息上
/// 重复注入（避免超出 4 个断点上限）。已存在的断点会计入上限，
/// 防止向更早的消息追加新断点。
///
/// 跳过：空 content / 空 system 不注入断点。
///
/// # 参数
/// - `body`：Anthropic `/v1/messages` 请求 body（含 `system` 与 `messages` 字段）
/// - `ttl`：缓存 TTL
///
/// # 注意
/// - `CacheTtl::OneHour` 需调用方额外设置
///   `anthropic-beta: extended-cache-ttl-2025-04-11` header
///   （`CacheTtl::beta_header()` 返回 header 值）
/// - 本函数只修改 body，不修改 HTTP headers
pub fn apply_cache_control(body: &mut serde_json::Value, ttl: CacheTtl) {
    let cache_control = serde_json::json!({
        "type": "ephemeral",
        "ttl": ttl.as_str(),
    });

    let mut breakpoints = 0usize;

    // 1. system 末尾断点（system 必须转为 block array 形式）
    if breakpoints < MAX_CACHE_BREAKPOINTS {
        if let Some(system) = body.get_mut("system") {
            if !system.is_null() {
                if ensure_block_array_with_cache(system, &cache_control) {
                    breakpoints += 1;
                } else if has_cache_control_on_last_block(system) {
                    // 已有断点 → 计数（避免向更早的消息追加新断点）
                    breakpoints += 1;
                }
            }
        }
    }

    // 2. 最后 N 个非 system 消息断点（N = MAX_CACHE_BREAKPOINTS - breakpoints）
    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for msg in messages.iter_mut().rev() {
            if breakpoints >= MAX_CACHE_BREAKPOINTS {
                break;
            }
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if role == "system" {
                continue;
            }
            if let Some(content) = msg.get_mut("content") {
                if ensure_block_array_with_cache(content, &cache_control) {
                    breakpoints += 1;
                } else if has_cache_control_on_last_block(content) {
                    // 已有断点 → 计数（避免向更早的消息追加新断点）
                    breakpoints += 1;
                }
            }
        }
    }
}

/// 将 content/system 规范化为 block array，并在最后一个 block 注入 cache_control。
///
/// 输入可能是：
/// - string → 转为 `[{type: text, text: ..., cache_control: ...}]`
/// - array → 在最后一个 block 上注入 cache_control（若尚无）
///
/// 返回 `true` 表示新注入了 cache_control；`false` 表示未注入
/// （空内容 / 已存在 cache_control / 既非 string 也非 array）。
fn ensure_block_array_with_cache(
    value: &mut serde_json::Value,
    cache_control: &serde_json::Value,
) -> bool {
    if value.is_string() {
        let text = value.as_str().unwrap_or("").to_string();
        if text.is_empty() {
            return false;
        }
        *value = serde_json::json!([{
            "type": "text",
            "text": text,
            "cache_control": cache_control.clone(),
        }]);
        return true;
    }

    if let Some(arr) = value.as_array_mut() {
        if arr.is_empty() {
            return false;
        }
        if let Some(last) = arr.last_mut() {
            return inject_cache_control_if_absent(last, cache_control);
        }
    }

    false
}

/// 检查 value（string 或 block array）的最后一个 block 是否已有 cache_control。
///
/// - string → 无 cache_control（string 形式不带 cache_control）
/// - array → 检查最后一个 block
/// - 其他 → false
fn has_cache_control_on_last_block(value: &serde_json::Value) -> bool {
    if let Some(arr) = value.as_array() {
        arr.last()
            .map(|last| last.get("cache_control").is_some())
            .unwrap_or(false)
    } else {
        false
    }
}

/// 若 block 上尚无 cache_control，则注入。
///
/// 幂等：已存在 cache_control 的 block 不重复注入，返回 false。
fn inject_cache_control_if_absent(
    block: &mut serde_json::Value,
    cache_control: &serde_json::Value,
) -> bool {
    if block.get("cache_control").is_some() {
        return false;
    }
    block["cache_control"] = cache_control.clone();
    true
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str { "anthropic" }
    fn api_key(&self) -> &str { &self.api_key }
    fn base_url(&self) -> &str { &self.base_url }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo { id: "claude-sonnet-4-20250514".into(), provider: "anthropic".into(), name: "Claude Sonnet 4".into(), context_window: 200000, supports_tools: true, supports_streaming: true },
            ModelInfo { id: "claude-3-5-haiku-20241022".into(), provider: "anthropic".into(), name: "Claude 3.5 Haiku".into(), context_window: 200000, supports_tools: true, supports_streaming: true },
            ModelInfo { id: "claude-3-opus-20240229".into(), provider: "anthropic".into(), name: "Claude 3 Opus".into(), context_window: 200000, supports_tools: true, supports_streaming: true },
        ]
    }

    async fn complete(&self, model: &str, system: &str, messages: &[Message], max_tokens: u64) -> Result<String, AppError> {
        let start = std::time::Instant::now();
        tracing::info!(model = %model, messages = messages.len(), max_tokens, "anthropic_complete: enter");

        let result = async {
            let body = self.build_request(model, system, messages, &[], false, max_tokens);
            let resp = self.client.post(format!("{}/v1/messages", self.base_url))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body).send().await
                .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;
            let json: serde_json::Value = resp.json().await
                .map_err(|e| AppError::internal(format!("Response parse failed: {}", e)))?;
            json["content"][0]["text"].as_str().map(|s| s.to_string())
                .ok_or_else(|| {
                    tracing::warn!(response = %json, "Anthropic no content");
                    AppError::internal("No content in Anthropic response")
                })
        }.await;
        
        match &result {
            Ok(_) => tracing::info!(
                model = %model,
                duration_ms = start.elapsed().as_millis(),
                "anthropic_complete: exit"
            ),
            Err(e) => tracing::error!(
                model = %model,
                error = %e,
                duration_ms = start.elapsed().as_millis(),
                "anthropic_complete: error"
            ),
        }
        result
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec], max_tokens: u64) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>, AppError> {
        let body = self.build_request(model, system, messages, tools, true, max_tokens);
        let resp = self.client.post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body).send().await
            .map_err(|e| AppError::stream_error(e.to_string()))?;

        // 非 2xx 响应必须在此处拦截：错误 body 不能交给 bytes_stream，
        // 否则会被当作空流，前端收到空回复（P0 bug 根因）。
        let http_status = resp.status();
        if !http_status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %http_status, body = %body, "Anthropic stream non-2xx response");
            return Err(super::map_stream_http_error(http_status, &body));
        }

        let byte_stream = resp.bytes_stream();
        let state = AnthropicStreamState::new();
        let event_stream = byte_stream.scan(state, |state, chunk| {
            let events: Vec<StreamEvent> = match chunk {
                Ok(bytes) => {
                    let lines = state.sse_buffer.push(bytes.as_ref());
                    let mut events = Vec::new();
                    for line in lines {
                        events.extend(parse_anthropic_sse_events(&line, state));
                    }
                    events
                }
                Err(e) => vec![StreamEvent::Error(e.to_string())],
            };
            futures_util::future::ready(Some(futures_util::stream::iter(events)))
        }).flatten();
        Ok(Box::pin(event_stream))
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        let resp = self.client.get(format!("{}/v1/models", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

/// Anthropic SSE 流式解析状态
///
/// 跨 chunk 累积：
/// - `sse_buffer`：跨 chunk 行缓冲（多字节字符切断 / SSE 行切断恢复）
/// - `usage`：token 计数（`message_start` 提供 input_tokens，`message_delta` 提供 output_tokens）
/// - `tool_call_ids`：`index → tool_call_id` 映射，关联 `content_block_start` 与后续 delta/stop
/// - `stop_reason`：`message_delta` 缓存的停止原因，待 `message_stop` 决定 `FinishReason`
struct AnthropicStreamState {
    sse_buffer: super::sse_buffer::SseLineBuffer,
    usage: TokenUsage,
    tool_call_ids: HashMap<usize, String>,
    stop_reason: Option<String>,
}

impl AnthropicStreamState {
    fn new() -> Self {
        Self {
            sse_buffer: super::sse_buffer::SseLineBuffer::new(),
            usage: TokenUsage::default(),
            tool_call_ids: HashMap::new(),
            stop_reason: None,
        }
    }
}

/// 解析单行 Anthropic SSE，返回 0..N 个 `StreamEvent`。
///
/// 纯函数（仅依赖 `state` 与 `line`，无 I/O），便于用 fixture 字符串测试。
///
/// 处理的事件类型：
/// - `content_block_start`：`tool_use` block → `ToolCallStart { id, name }`，并记录 index→id
/// - `content_block_delta`：`thinking` → `ReasoningDelta`，`text` → `TextDelta`，
///   `input_json_delta` → `ToolCallDelta { id, args_delta }`（id 从 index→id 映射查找）
/// - `content_block_stop`：若 index 在映射中 → `ToolCallEnd { id }`，并移除映射
/// - `message_start`：累积 `usage.input_tokens`
/// - `message_delta`：累积 `usage.output_tokens`，缓存 `delta.stop_reason`
/// - `message_stop`：依据缓存的 `stop_reason` 决定 `FinishReason`（`tool_use` → `ToolCalls`，否则 `Stop`）
fn parse_anthropic_sse_events(line: &str, state: &mut AnthropicStreamState) -> Vec<StreamEvent> {
    let line = line.trim();
    if line.is_empty() || !line.starts_with("data: ") {
        return Vec::new();
    }
    let data = &line[6..];
    let json: serde_json::Value = match serde_json::from_str(data) {
        Ok(j) => j,
        Err(_) => return Vec::new(),
    };
    let mut events = Vec::new();
    match json["type"].as_str() {
        Some("content_block_start") => {
            if let Some(cb) = json.get("content_block") {
                if cb["type"].as_str() == Some("tool_use") {
                    let id = cb["id"].as_str().unwrap_or("").to_string();
                    let name = cb["name"].as_str().unwrap_or("").to_string();
                    let index = json["index"].as_u64().unwrap_or(0) as usize;
                    if !id.is_empty() {
                        state.tool_call_ids.insert(index, id.clone());
                        events.push(StreamEvent::ToolCallStart { id, name });
                    }
                }
            }
        }
        Some("content_block_delta") => {
            if let Some(thinking) = json["delta"]["thinking"].as_str() {
                if !thinking.is_empty() {
                    events.push(StreamEvent::ReasoningDelta { content: thinking.to_string() });
                }
            }
            if let Some(text) = json["delta"]["text"].as_str() {
                if !text.is_empty() {
                    events.push(StreamEvent::TextDelta { content: text.to_string() });
                }
            }
            if json["delta"]["type"].as_str() == Some("input_json_delta") {
                let index = json["index"].as_u64().unwrap_or(0) as usize;
                if let Some(id) = state.tool_call_ids.get(&index).cloned() {
                    if let Some(partial) = json["delta"]["partial_json"].as_str() {
                        if !partial.is_empty() {
                            events.push(StreamEvent::ToolCallDelta {
                                id,
                                args_delta: partial.to_string(),
                            });
                        }
                    }
                }
            }
        }
        Some("content_block_stop") => {
            let index = json["index"].as_u64().unwrap_or(0) as usize;
            if let Some(id) = state.tool_call_ids.remove(&index) {
                events.push(StreamEvent::ToolCallEnd { id });
            }
        }
        Some("message_start") => {
            if let Some(msg) = json.get("message") {
                if let Some(u) = msg.get("usage") {
                    if let Some(input) = u.get("input_tokens").and_then(|v| v.as_u64()) {
                        state.usage.input_tokens = input as u32;
                    }
                }
            }
        }
        Some("message_delta") => {
            if let Some(u) = json.get("usage") {
                if let Some(output) = u.get("output_tokens").and_then(|v| v.as_u64()) {
                    state.usage.output_tokens = output as u32;
                }
            }
            if let Some(reason) = json["delta"]["stop_reason"].as_str() {
                state.stop_reason = Some(reason.to_string());
            }
        }
        Some("message_stop") => {
            let reason = match state.stop_reason.as_deref() {
                Some("tool_use") => FinishReason::ToolCalls,
                _ => FinishReason::Stop,
            };
            events.push(StreamEvent::Finish {
                reason,
                usage: state.usage,
            });
            // 重置 stop_reason，防止下个 chunk 误用
            state.stop_reason = None;
        }
        _ => {}
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个混合 text + tool_use 的 Anthropic SSE fixture（仅 `data:` 行）。
    fn fixture_tool_use_lines() -> Vec<&'static str> {
        vec![
            r#"data: {"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"claude","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":1}}}"#,
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
            r#"data: {"type":"content_block_stop","index":0}"#,
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_01abc","name":"get_weather","input":{}}}"#,
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"location\":"}}"#,
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"\"San Francisco\"}"}}"#,
            r#"data: {"type":"content_block_stop","index":1}"#,
            r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":56}}"#,
            r#"data: {"type":"message_stop"}"#,
        ]
    }

    #[test]
    fn parses_tool_use_sse_sequence() {
        let mut state = AnthropicStreamState::new();
        let mut all_events = Vec::new();
        for line in fixture_tool_use_lines() {
            all_events.extend(parse_anthropic_sse_events(line, &mut state));
        }

        // 期望事件序列：
        //   TextDelta("Hello")
        //   ToolCallStart { id: "toolu_01abc", name: "get_weather" }
        //   ToolCallDelta { id: "toolu_01abc", args_delta: "{\"location\":" }
        //   ToolCallDelta { id: "toolu_01abc", args_delta: "\"San Francisco\"}" }
        //   ToolCallEnd { id: "toolu_01abc" }
        //   Finish { reason: ToolCalls, usage: { input: 10, output: 56 } }
        let mut idx = 0;

        match &all_events[idx] {
            StreamEvent::TextDelta { content } => assert_eq!(content, "Hello"),
            e => panic!("expected TextDelta, got {:?}", e),
        }
        idx += 1;

        match &all_events[idx] {
            StreamEvent::ToolCallStart { id, name } => {
                assert_eq!(id, "toolu_01abc");
                assert_eq!(name, "get_weather");
            }
            e => panic!("expected ToolCallStart, got {:?}", e),
        }
        idx += 1;

        match &all_events[idx] {
            StreamEvent::ToolCallDelta { id, args_delta } => {
                assert_eq!(id, "toolu_01abc");
                assert_eq!(args_delta, "{\"location\":");
            }
            e => panic!("expected ToolCallDelta #1, got {:?}", e),
        }
        idx += 1;

        match &all_events[idx] {
            StreamEvent::ToolCallDelta { id, args_delta } => {
                assert_eq!(id, "toolu_01abc");
                assert_eq!(args_delta, "\"San Francisco\"}");
            }
            e => panic!("expected ToolCallDelta #2, got {:?}", e),
        }
        idx += 1;

        match &all_events[idx] {
            StreamEvent::ToolCallEnd { id } => assert_eq!(id, "toolu_01abc"),
            e => panic!("expected ToolCallEnd, got {:?}", e),
        }
        idx += 1;

        match &all_events[idx] {
            StreamEvent::Finish { reason, usage } => {
                assert!(matches!(reason, FinishReason::ToolCalls), "expected ToolCalls, got {:?}", reason);
                assert_eq!(usage.input_tokens, 10);
                assert_eq!(usage.output_tokens, 56);
            }
            e => panic!("expected Finish, got {:?}", e),
        }
        idx += 1;

        assert_eq!(
            idx, all_events.len(),
            "unexpected extra events: {:?}", &all_events[idx..]
        );

        // 校验：message_stop 后 stop_reason 已被重置，tool_call_ids 已被清空
        assert!(state.stop_reason.is_none());
        assert!(state.tool_call_ids.is_empty());
    }

    #[test]
    fn parses_stop_reason_stop_for_normal_end_turn() {
        let mut state = AnthropicStreamState::new();
        let mut all_events = Vec::new();
        for line in [
            r#"data: {"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"claude","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":1}}}"#,
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
            r#"data: {"type":"content_block_stop","index":0}"#,
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":5}}"#,
            r#"data: {"type":"message_stop"}"#,
        ] {
            all_events.extend(parse_anthropic_sse_events(line, &mut state));
        }

        // 最后一个事件应为 Finish with Stop reason，且 usage 跨事件累积正确
        match all_events.last() {
            Some(StreamEvent::Finish { reason, usage }) => {
                assert!(matches!(reason, FinishReason::Stop), "expected Stop, got {:?}", reason);
                assert_eq!(usage.input_tokens, 10);
                assert_eq!(usage.output_tokens, 5);
            }
            e => panic!("expected Finish last, got {:?}", e),
        }
    }

    #[test]
    fn parses_extended_thinking_delta() {
        let mut state = AnthropicStreamState::new();
        let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}"#;
        let events = parse_anthropic_sse_events(line, &mut state);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::ReasoningDelta { content } => assert_eq!(content, "hmm"),
            e => panic!("expected ReasoningDelta, got {:?}", e),
        }
    }

    #[test]
    fn ignores_non_data_and_invalid_json_lines() {
        let mut state = AnthropicStreamState::new();
        // 空 / event: 行 / 非 JSON 数据 → 无事件
        assert!(parse_anthropic_sse_events("", &mut state).is_empty());
        assert!(parse_anthropic_sse_events("event: message_start", &mut state).is_empty());
        assert!(parse_anthropic_sse_events("data: not json", &mut state).is_empty());
        assert!(parse_anthropic_sse_events(": comment", &mut state).is_empty());
    }

    #[test]
    fn empty_tool_use_id_does_not_register() {
        // content_block.id 为空时不注册映射，也不发 ToolCallStart
        let mut state = AnthropicStreamState::new();
        let line = r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"","name":"x","input":{}}}"#;
        let events = parse_anthropic_sse_events(line, &mut state);
        assert!(events.is_empty(), "empty id should not emit ToolCallStart");
        assert!(state.tool_call_ids.is_empty());

        // 后续 input_json_delta 也不应产生事件（无 id 映射）
        let delta_line = r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{}"}}"#;
        let events = parse_anthropic_sse_events(delta_line, &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn provider_constructor_smoke() {
        let p = AnthropicProvider::new("test-key".to_string(), None);
        assert_eq!(p.name(), "anthropic");
        assert_eq!(p.api_key(), "test-key");
        assert_eq!(p.base_url(), "https://api.anthropic.com");
        assert!(!p.models().is_empty());
    }

    // ── Task D4: apply_cache_control 单元测试 ──

    #[test]
    fn cache_ttl_as_str_returns_correct_values() {
        assert_eq!(CacheTtl::FiveMinutes.as_str(), "5m");
        assert_eq!(CacheTtl::OneHour.as_str(), "1h");
    }

    #[test]
    fn cache_ttl_default_is_five_minutes() {
        assert_eq!(CacheTtl::default(), CacheTtl::FiveMinutes);
    }

    #[test]
    fn cache_ttl_beta_header_only_for_one_hour() {
        assert_eq!(CacheTtl::FiveMinutes.beta_header(), None);
        assert_eq!(
            CacheTtl::OneHour.beta_header(),
            Some("extended-cache-ttl-2025-04-11")
        );
    }

    #[test]
    fn apply_cache_control_converts_string_system_to_array_with_breakpoint() {
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "system": "You are helpful.",
            "messages": [],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        let system = body["system"].as_array().expect("system 应转为 array");
        assert_eq!(system.len(), 1);
        assert_eq!(system[0]["type"], "text");
        assert_eq!(system[0]["text"], "You are helpful.");
        assert_eq!(system[0]["cache_control"]["type"], "ephemeral");
        assert_eq!(system[0]["cache_control"]["ttl"], "5m");
    }

    #[test]
    fn apply_cache_control_injects_breakpoint_on_last_3_non_system_messages() {
        // 含 system → 1 system + 3 messages = 4 断点
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "msg1"},
                {"role": "assistant", "content": "resp1"},
                {"role": "user", "content": "msg2"},
                {"role": "assistant", "content": "resp2"},
                {"role": "user", "content": "msg3"},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        let messages = body["messages"].as_array().unwrap();
        // 前 2 个消息保持 string（未注入 cache_control）
        assert!(messages[0]["content"].is_string(), "msg1 应保持 string");
        assert!(messages[1]["content"].is_string(), "resp1 应保持 string");
        // 后 3 个消息应转为 array 并注入 cache_control
        for i in 2..=4 {
            let content = messages[i]["content"]
                .as_array()
                .unwrap_or_else(|| panic!("messages[{}] content 应为 array", i));
            assert_eq!(content.len(), 1);
            assert_eq!(
                content[0]["cache_control"]["type"],
                "ephemeral",
                "messages[{}] 应有 cache_control",
                i
            );
            assert_eq!(content[0]["cache_control"]["ttl"], "5m");
        }
    }

    #[test]
    fn apply_cache_control_injects_4_breakpoints_total() {
        // system + 5 个消息 → 应注入 1 system + 3 messages = 4 个断点
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "msg1"},
                {"role": "assistant", "content": "resp1"},
                {"role": "user", "content": "msg2"},
                {"role": "assistant", "content": "resp2"},
                {"role": "user", "content": "msg3"},
                {"role": "assistant", "content": "resp3"},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        // system 1 个断点
        let system = body["system"].as_array().unwrap();
        assert!(system[0].get("cache_control").is_some(), "system 应有断点");

        let messages = body["messages"].as_array().unwrap();
        // 前 3 个消息保持 string
        assert!(messages[0]["content"].is_string());
        assert!(messages[1]["content"].is_string());
        assert!(messages[2]["content"].is_string());
        // 后 3 个消息有 cache_control
        for i in 3..=5 {
            let content = messages[i]["content"].as_array().unwrap();
            assert!(
                content[0].get("cache_control").is_some(),
                "messages[{}] 应有 cache_control",
                i
            );
        }
    }

    #[test]
    fn apply_cache_control_is_idempotent() {
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "msg1"},
                {"role": "assistant", "content": "resp1"},
                {"role": "user", "content": "msg2"},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        // system 不应重复
        let system = body["system"].as_array().unwrap();
        assert_eq!(system.len(), 1);
        assert!(system[0].get("cache_control").is_some());

        // 最后 3 个消息各有 1 个 cache_control，不叠加
        let messages = body["messages"].as_array().unwrap();
        for i in 0..3 {
            let content = messages[i]["content"].as_array().unwrap();
            assert_eq!(content.len(), 1, "messages[{}] 不应追加额外 block", i);
            assert!(content[0].get("cache_control").is_some());
        }
    }

    #[test]
    fn apply_cache_control_skips_empty_content() {
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": ""},
                {"role": "assistant", "content": "resp"},
                {"role": "user", "content": "msg"},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        let messages = body["messages"].as_array().unwrap();
        // 空 content 应保持 string（不转 array）
        assert_eq!(messages[0]["content"], "");
        // resp / msg 应有 cache_control
        assert_eq!(
            messages[1]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        assert_eq!(
            messages[2]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
    }

    #[test]
    fn apply_cache_control_one_hour_ttl_uses_1h() {
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "system": "You are helpful.",
            "messages": [],
        });
        apply_cache_control(&mut body, CacheTtl::OneHour);

        let system = body["system"].as_array().unwrap();
        assert_eq!(system[0]["cache_control"]["ttl"], "1h");
    }

    #[test]
    fn apply_cache_control_skips_system_role_in_messages() {
        // messages 数组中混入 system role 应跳过
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "messages": [
                {"role": "system", "content": "sys-in-msgs"},
                {"role": "user", "content": "msg1"},
                {"role": "assistant", "content": "resp1"},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        let messages = body["messages"].as_array().unwrap();
        // system role 应保持 string（不转 array）
        assert!(messages[0]["content"].is_string());
        // user / assistant 应有 cache_control
        assert!(messages[1]["content"][0].get("cache_control").is_some());
        assert!(messages[2]["content"][0].get("cache_control").is_some());
    }

    #[test]
    fn apply_cache_control_handles_already_array_content() {
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "block1"},
                    {"type": "text", "text": "block2"}
                ]},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        let content = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2, "不应追加额外 block");
        // cache_control 应在最后一个 block（block2）
        assert!(content[0].get("cache_control").is_none(), "第一个 block 不应有 cache_control");
        assert!(content[1].get("cache_control").is_some(), "最后一个 block 应有 cache_control");
    }

    #[test]
    fn apply_cache_control_no_system_only_messages_breakpoints() {
        // 无 system → 仅注入消息断点（最多 4 个）
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": "msg1"},
                {"role": "assistant", "content": "resp1"},
                {"role": "user", "content": "msg2"},
                {"role": "assistant", "content": "resp2"},
            ],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        let messages = body["messages"].as_array().unwrap();
        // 4 个消息全部应有 cache_control（无 system 占用 1 个名额）
        for (i, msg) in messages.iter().enumerate() {
            let content = msg["content"]
                .as_array()
                .unwrap_or_else(|| panic!("messages[{}] content 应为 array", i));
            assert!(
                content[0].get("cache_control").is_some(),
                "messages[{}] 应有 cache_control",
                i
            );
        }
    }

    #[test]
    fn apply_cache_control_empty_messages_no_panic() {
        let mut body = serde_json::json!({
            "model": "claude",
            "max_tokens": 100,
            "system": "You are helpful.",
            "messages": [],
        });
        apply_cache_control(&mut body, CacheTtl::FiveMinutes);

        // system 应有断点，messages 为空数组
        let system = body["system"].as_array().unwrap();
        assert!(system[0].get("cache_control").is_some());
        assert!(body["messages"].as_array().unwrap().is_empty());
    }

    #[test]
    fn build_request_auto_applies_cache_control() {
        let p = AnthropicProvider::new("test-key".to_string(), None);
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: "hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let body = p.build_request("claude", "You are helpful.", &messages, &[], false, 100);

        // system 应被转为 array 并注入 cache_control
        let system = body["system"]
            .as_array()
            .expect("build_request 应自动应用 cache_control，使 system 为 array");
        assert!(system[0].get("cache_control").is_some());

        // 最后一个消息应有 cache_control
        let msgs = body["messages"].as_array().unwrap();
        assert!(msgs[0]["content"].is_array(), "消息 content 应转为 array");
        assert!(msgs[0]["content"][0].get("cache_control").is_some());
    }
}