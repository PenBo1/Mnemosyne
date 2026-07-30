//! ═══════════════════════════════════════════════════════════════════════════
//! Embedding 客户端 - OpenAI 兼容 API 调用
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 本地(Ollama OpenAI 兼容端点 / LM Studio)与云端(OpenAI / 其他兼容服务)
//! 走同一套协议，仅 base_url / api_key / model 不同。

use std::sync::OnceLock;

use super::types::EmbeddingConfig;
use crate::shared::error::AppError;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 复用全局 reqwest::Client(连接池共享,避免每次请求重建)
fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default()
    })
}

/// 检查 embedding 服务是否可用（快速失败）
pub async fn check_service_available(config: &EmbeddingConfig) -> Result<(), AppError> {
    if !config.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }
    if config.base_url.trim().is_empty() {
        return Err(AppError::invalid_input("Embedding base_url is empty"));
    }

    let base_url = config.base_url.trim_end_matches('/');
    let health_url = format!("{}/models", base_url);

    let client = http_client();
    let resp = client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(r) => Err(AppError::connection_refused(format!(
            "Embedding service returned status {}",
            r.status()
        ))),
        Err(e) => Err(AppError::connection_refused(format!(
            "Embedding service unavailable: {}. Make sure the service is running at {}",
            e, base_url
        ))),
    }
}

/// 调用 /v1/embeddings 对单段文本生成向量。
pub async fn embed(text: &str, config: &EmbeddingConfig) -> Result<Vec<f32>, AppError> {
    let vecs = embed_batch(&[text.to_string()], config).await?;
    vecs.into_iter().next()
        .ok_or_else(|| AppError::internal("Embedding response missing data"))
}

/// 调用 /v1/embeddings 对多段文本批量生成向量。
pub async fn embed_batch(
    texts: &[String],
    config: &EmbeddingConfig,
) -> Result<Vec<Vec<f32>>, AppError> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    if !config.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }
    if config.base_url.trim().is_empty() {
        return Err(AppError::invalid_input("Embedding base_url is empty"));
    }
    if config.model.trim().is_empty() {
        return Err(AppError::invalid_input("Embedding model is empty"));
    }

    let url = format!("{}/embeddings", config.base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": config.model,
        "input": texts,
    });

    let client = http_client();
    let mut req = client.post(&url).json(&body);
    if !config.api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    tracing::info!(url = %url, model = %config.model, count = texts.len(), "Embedding request");
    let resp = req.send().await
        .map_err(|e| AppError::connection_refused(format!("Embedding request failed: {}", e)))?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await
        .map_err(|e| AppError::invalid_format(format!("Embedding response parse failed: {}", e)))?;

    if !status.is_success() {
        let msg = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str())
            .unwrap_or("Unknown embedding error");
        return Err(AppError::internal(format!("Embedding API {}: {}", status, msg)));
    }

    let data = json.get("data").and_then(|d| d.as_array())
        .ok_or_else(|| AppError::invalid_format("Embedding response missing 'data' array"))?;

    let mut result = Vec::with_capacity(data.len());
    for item in data {
        let emb = item.get("embedding").and_then(|e| e.as_array())
            .ok_or_else(|| AppError::invalid_format("Embedding item missing 'embedding'"))?;
        let vec: Vec<f32> = emb.iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        if vec.len() != config.dim {
            return Err(AppError::invalid_format(format!(
                "Embedding dim mismatch: expected {}, got {}", config.dim, vec.len()
            )));
        }
        result.push(vec);
    }
    Ok(result)
}

/// 测试 embedding 连接:对一段测试文本生成向量,返回维度。
pub async fn test_connection(config: &EmbeddingConfig) -> Result<usize, AppError> {
    let vec = embed("hello", config).await?;
    Ok(vec.len())
}
