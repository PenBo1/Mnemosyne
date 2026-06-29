use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use super::types::*;
use super::openai_protocol;
use crate::shared::errors::AppError;

pub struct AgnesProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AgnesProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "https://apihub.agnes-ai.com/v1".to_string());
        tracing::debug!(base_url = %url, "AgnesProvider created");
        Self { client: Client::new(), api_key, base_url: url }
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

    async fn complete(&self, model: &str, system: &str, messages: &[Message]) -> Result<String, AppError> {
        tracing::info!(model = %model, messages = messages.len(), "Agnes complete request");
        let body = openai_protocol::build_request(model, system, messages, &[], false);
        let resp = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "Agnes request failed");
                AppError::stream_error(e.to_string())
            })?;
        let json: serde_json::Value = resp.json().await
            .map_err(|e| {
                tracing::error!(error = %e, "Agnes response parse failed");
                AppError::invalid_format(e.to_string())
            })?;
        json["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
            .ok_or_else(|| {
                tracing::error!("No content in Agnes response");
                AppError::internal("No content in Agnes response")
            })
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], tools: &[ToolSpec]) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = StreamEvent> + Send>>, AppError> {
        tracing::info!(model = %model, messages = messages.len(), tools = tools.len(), "Agnes stream request");
        let body = openai_protocol::build_request(model, system, messages, tools, true);
        let resp = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "Agnes stream request failed");
                AppError::stream_error(e.to_string())
            })?;
        tracing::info!(status = %resp.status(), "Agnes stream response received");

        let byte_stream = resp.bytes_stream();
        let event_stream = byte_stream.filter_map(|chunk| async {
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
                            let mut usage = TokenUsage::default();
                            if let Some(u) = json.get("usage") {
                                if let Some(pt) = u.get("prompt_tokens").and_then(|v| v.as_u64()) {
                                    usage.input_tokens = pt as u32;
                                }
                                if let Some(ct) = u.get("completion_tokens").and_then(|v| v.as_u64()) {
                                    usage.output_tokens = ct as u32;
                                }
                            }
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
        }).flatten();
        Ok(Box::pin(event_stream))
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        tracing::info!(base_url = %self.base_url, "Agnes test_connection");
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
            tracing::info!("Agnes connection test passed");
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "Agnes connection test failed");
            Err(AppError::provider_unavailable("agnes"))
        }
    }
}
