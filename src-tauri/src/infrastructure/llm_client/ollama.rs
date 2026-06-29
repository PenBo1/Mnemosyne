use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use super::types::*;
use crate::shared::errors::AppError;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        tracing::debug!(base_url = %url, "OllamaProvider created");
        Self { client: Client::new(), base_url: url }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str { "ollama" }
    fn api_key(&self) -> &str { "" }
    fn base_url(&self) -> &str { &self.base_url }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo { id: "llama3.1".into(), provider: "ollama".into(), name: "Llama 3.1".into(), context_window: 128000, supports_tools: true, supports_streaming: true },
            ModelInfo { id: "qwen2.5".into(), provider: "ollama".into(), name: "Qwen 2.5".into(), context_window: 128000, supports_tools: true, supports_streaming: true },
        ]
    }

    async fn complete(&self, model: &str, system: &str, messages: &[Message]) -> Result<String, AppError> {
        tracing::info!(model = %model, messages = messages.len(), "Ollama complete request");
        let mut msgs = vec![serde_json::json!({ "role": "system", "content": system })];
        for m in messages { msgs.push(serde_json::json!({ "role": m.role, "content": m.content })); }
        let body = serde_json::json!({ "model": model, "messages": msgs, "stream": false });
        let resp = self.client.post(format!("{}/api/chat", self.base_url))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "Ollama request failed");
                AppError::stream_error(e.to_string())
            })?;
        let json: serde_json::Value = resp.json().await
            .map_err(|e| {
                tracing::error!(error = %e, "Ollama response parse failed");
                AppError::invalid_format(e.to_string())
            })?;
        json["message"]["content"].as_str().map(|s| s.to_string())
            .ok_or_else(|| {
                tracing::error!("No content in Ollama response");
                AppError::internal("No content in Ollama response")
            })
    }

    async fn stream(&self, model: &str, system: &str, messages: &[Message], _tools: &[ToolSpec]) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = StreamEvent> + Send>>, AppError> {
        tracing::info!(model = %model, messages = messages.len(), "Ollama stream request");
        let mut msgs = vec![serde_json::json!({ "role": "system", "content": system })];
        for m in messages { msgs.push(serde_json::json!({ "role": m.role, "content": m.content })); }
        let body = serde_json::json!({ "model": model, "messages": msgs, "stream": true });
        let resp = self.client.post(format!("{}/api/chat", self.base_url))
            .json(&body).send().await
            .map_err(|e| {
                tracing::error!(error = %e, "Ollama stream failed");
                AppError::stream_error(e.to_string())
            })?;
        tracing::info!(status = %resp.status(), "Ollama stream response received");

        let byte_stream = resp.bytes_stream();
        let event_stream = byte_stream.map(|chunk| {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut events = Vec::new();
                    for line in text.lines() {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(content) = json["message"]["content"].as_str() {
                                if !content.is_empty() { events.push(StreamEvent::TextDelta { content: content.to_string() }); }
                            }
                            if json["done"].as_bool().unwrap_or(false) {
                                events.push(StreamEvent::Finish { reason: FinishReason::Stop, usage: TokenUsage {
                                    input_tokens: json["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                                    output_tokens: json["eval_count"].as_u64().unwrap_or(0) as u32,
                                }});
                            }
                        }
                    }
                    futures::stream::iter(events)
                }
                Err(e) => {
                    tracing::error!(error = %e, "Ollama stream chunk error");
                    futures::stream::iter(vec![StreamEvent::Error(e.to_string())])
                }
            }
        }).flatten();
        Ok(Box::pin(event_stream))
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        tracing::info!(base_url = %self.base_url, "Ollama test_connection");
        let resp = self.client.get(format!("{}/api/tags", self.base_url))
            .send().await
            .map_err(|e| {
                tracing::error!(error = %e, "Ollama connection failed");
                AppError::connection_refused(self.base_url.clone())
            })?;

        if resp.status().is_success() {
            tracing::info!("Ollama connection test passed");
            Ok(())
        } else {
            let status = resp.status();
            tracing::error!(status = %status, "Ollama connection test failed");
            Err(AppError::provider_unavailable("ollama"))
        }
    }
}
