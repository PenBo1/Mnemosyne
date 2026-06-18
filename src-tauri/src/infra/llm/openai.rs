use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use super::types::*;
use crate::errors::AppError;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }

    fn build_tools_payload(tools: &[ToolSpec]) -> Vec<serde_json::Value> {
        tools.iter().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": { "name": t.name, "description": t.description, "parameters": t.parameters }
            })
        }).collect()
    }

    fn build_request(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec], stream: bool) -> serde_json::Value {
        let mut msgs = vec![serde_json::json!({ "role": "system", "content": system })];
        for m in messages {
            let mut entry = serde_json::json!({ "role": m.role, "content": m.content });
            if let Some(tc) = &m.tool_calls { entry["tool_calls"] = serde_json::to_value(tc).unwrap(); }
            if let Some(tcid) = &m.tool_call_id { entry["tool_call_id"] = serde_json::Value::String(tcid.clone()); }
            msgs.push(entry);
        }
        let mut body = serde_json::json!({ "model": model, "messages": msgs, "stream": stream });
        if !tools.is_empty() { body["tools"] = serde_json::json!(Self::build_tools_payload(tools)); }
        body
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

    async fn complete(&self, model: &str, system: &str, messages: &[Message]) -> Result<String, AppError> {
        let body = self.build_request(model, system, messages, &[], false);
        let resp = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body).send().await
            .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;
        let json: serde_json::Value = resp.json().await
            .map_err(|e| AppError::internal(format!("Response parse failed: {}", e)))?;
        json["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
            .ok_or_else(|| AppError::internal("No content in response"))
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec]) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = StreamEvent> + Send>>, AppError> {
        tracing::info!(model = %model, messages = messages.len(), tools = tools.len(), "OpenAI stream request");
        let body = self.build_request(model, system, messages, tools, true);
        let resp = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "OpenAI stream request failed");
                AppError::stream_error(e.to_string())
            })?;
        tracing::info!(status = %resp.status(), "OpenAI stream response received");

        let byte_stream = resp.bytes_stream();
        // Collect all events first, then deduplicate finish events
        let event_stream = byte_stream
            .filter_map(|chunk| async {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let mut events = Vec::new();
                        for line in text.lines() {
                            let line = line.trim();
                            if line.is_empty() || !line.starts_with("data: ") { continue; }
                            let data = &line[6..];
                            if data == "[DONE]" { continue; }
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(choices) = json["choices"].as_array() {
                                    for choice in choices {
                                        if let Some(delta) = choice.get("delta") {
                                            if let Some(content) = delta["content"].as_str() {
                                                if !content.is_empty() { events.push(StreamEvent::TextDelta { content: content.to_string() }); }
                                            }
                                            if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                                for tc in tool_calls {
                                                    let id = tc["id"].as_str().unwrap_or("");
                                                    let name = tc["function"]["name"].as_str().unwrap_or("");
                                                    let args = tc["function"]["arguments"].as_str().unwrap_or("");
                                                    if !id.is_empty() && !name.is_empty() {
                                                        events.push(StreamEvent::ToolCallStart { id: id.to_string(), name: name.to_string() });
                                                    }
                                                    if !args.is_empty() {
                                                        events.push(StreamEvent::ToolCallDelta { id: id.to_string(), args_delta: args.to_string() });
                                                    }
                                                }
                                            }
                                        }
                                        if let Some(finish) = choice["finish_reason"].as_str() {
                                            let reason = match finish { "tool_calls" => FinishReason::ToolCalls, "length" => FinishReason::Length, _ => FinishReason::Stop };
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
                            }
                        }
                        if events.is_empty() { None } else { Some(futures::stream::iter(events)) }
                    }
                    Err(e) => Some(futures::stream::iter(vec![StreamEvent::Error(e.to_string())])),
                }
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
