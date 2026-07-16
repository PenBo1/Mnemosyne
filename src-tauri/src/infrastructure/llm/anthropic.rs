
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
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

    fn build_request(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec], stream: bool) -> serde_json::Value {
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

        let max_tokens = 8_192;

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

        body
    }
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

    async fn complete(&self, model: &str, system: &str, messages: &[Message]) -> Result<String, AppError> {
        let body = self.build_request(model, system, messages, &[], false);
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
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec]) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>, AppError> {
        let body = self.build_request(model, system, messages, tools, true);
        let resp = self.client.post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body).send().await
            .map_err(|e| AppError::stream_error(e.to_string()))?;

        let byte_stream = resp.bytes_stream();
        let buffer = super::sse_buffer::SseLineBuffer::new();
        let event_stream = byte_stream.scan(buffer, |buf, chunk| {
            let events: Vec<StreamEvent> = match chunk {
                Ok(bytes) => {
                    let lines = buf.push(bytes.as_ref());
                    let mut events = Vec::new();
                    let mut usage = TokenUsage::default();
                    for line in lines {
                        let line = line.trim();
                        if line.is_empty() || !line.starts_with("data: ") { continue; }
                        let data = &line[6..];
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            match json["type"].as_str() {
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
                                }
                                Some("message_delta") => {
                                    if let Some(u) = json.get("usage") {
                                        if let Some(output) = u.get("output_tokens").and_then(|v| v.as_u64()) {
                                            usage.output_tokens = output as u32;
                                        }
                                    }
                                }
                                Some("message_start") => {
                                    if let Some(msg) = json.get("message") {
                                        if let Some(u) = msg.get("usage") {
                                            if let Some(input) = u.get("input_tokens").and_then(|v| v.as_u64()) {
                                                usage.input_tokens = input as u32;
                                            }
                                        }
                                    }
                                }
                                Some("message_stop") => {
                                    events.push(StreamEvent::Finish {
                                        reason: FinishReason::Stop,
                                        usage,
                                    });
                                }
                                _ => {}
                            }
                        }
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