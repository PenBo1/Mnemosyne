//! ═══════════════════════════════════════════════════════════════════════════
//! AIGC 检测器 - 调用外部检测 API 并归一化分数
//! ═══════════════════════════════════════════════════════════════════════════

use std::time::Instant;

use crate::shared::error::AppError;

use super::types::DetectionResult;

// ── 常量定义 ────────────────────────────────────────────────────────────────

const REQUEST_TIMEOUT_SECS: u64 = 30;

// ── 公共接口 ────────────────────────────────────────────────────────────────

/// 各 provider 的默认 endpoint
pub fn default_api_url(provider: &str) -> Option<&'static str> {
    match provider {
        "gptzero" => Some("https://api.gptzero.me/v2/predict/text"),
        "originality" => Some("https://api.originality.ai/api/v1/scan/ai"),
        _ => None,
    }
}

/// 调用外部检测 API
pub async fn detect_ai_content(
    provider: &str,
    api_url: &str,
    api_key: &str,
    content: &str,
) -> Result<DetectionResult, AppError> {
    let start = Instant::now();
    tracing::info!(function = "detect_ai_content", provider, "入口");

    if content.trim().is_empty() {
        return Err(AppError::invalid_input("content cannot be empty"));
    }
    if api_key.is_empty() {
        return Err(AppError::invalid_input("detection API key is empty"));
    }
    if api_url.trim().is_empty() {
        return Err(AppError::invalid_input("apiUrl is empty"));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::internal(format!("HTTP client build failed: {}", e)))?;

    let detected_at = chrono::Utc::now().to_rfc3339();

    let result = match provider {
        "gptzero" => detect_gptzero(&client, api_url, api_key, content, &detected_at).await,
        "originality" => detect_originality(&client, api_url, api_key, content, &detected_at).await,
        "custom" => detect_custom(&client, api_url, api_key, content, &detected_at).await,
        other => Err(AppError::invalid_input(format!(
            "Unsupported provider: {} (expected gptzero/originality/custom)",
            other
        ))),
    };

    match &result {
        Ok(r) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "detect_ai_content", provider, duration_ms, score = r.score, "出口");
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "detect_ai_content", provider, duration_ms, error = %e, "错误");
        }
    }
    result
}

// ── Provider 实现 ────────────────────────────────────────────────────────────

async fn detect_gptzero(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    content: &str,
    detected_at: &str,
) -> Result<DetectionResult, AppError> {
    let body = serde_json::json!({ "document": content });
    let resp = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .header("X-Api-Key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("GPTZero request failed: {}", e)))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "GPTZero API failed: {} {}",
            status.as_u16(),
            text
        )));
    }
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal(format!("GPTZero response parse failed: {}", e)))?;
    let score = data
        .pointer("/documents/0/completely_generated_prob")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| AppError::invalid_format("GPTZero response missing completely_generated_prob"))?;
    Ok(DetectionResult {
        score,
        provider: "gptzero".into(),
        detected_at: detected_at.to_string(),
        details: Some(data),
    })
}

async fn detect_originality(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    content: &str,
    detected_at: &str,
) -> Result<DetectionResult, AppError> {
    let body = serde_json::json!({ "content": content });
    let resp = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Originality request failed: {}", e)))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "Originality API failed: {} {}",
            status.as_u16(),
            text
        )));
    }
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal(format!("Originality response parse failed: {}", e)))?;
    let score = data
        .pointer("/score/ai")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| AppError::invalid_format("Originality response missing score.ai"))?;
    Ok(DetectionResult {
        score,
        provider: "originality".into(),
        detected_at: detected_at.to_string(),
        details: Some(data),
    })
}

async fn detect_custom(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    content: &str,
    detected_at: &str,
) -> Result<DetectionResult, AppError> {
    let body = serde_json::json!({ "content": content });
    let resp = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Custom detection request failed: {}", e)))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "Custom detection API failed: {} {}",
            status.as_u16(),
            text
        )));
    }
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal(format!("Custom response parse failed: {}", e)))?;
    let score = data
        .get("score")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| AppError::invalid_format("Custom detection response missing score"))?;
    Ok(DetectionResult {
        score,
        provider: "custom".into(),
        detected_at: detected_at.to_string(),
        details: Some(data),
    })
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_api_url_for_known_providers() {
        assert!(default_api_url("gptzero").is_some());
        assert!(default_api_url("originality").is_some());
        assert!(default_api_url("custom").is_none());
    }
}