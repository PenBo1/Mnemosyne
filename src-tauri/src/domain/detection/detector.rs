// AIGC 检测器 —— 调用外部检测 API,归一化为 0-1 分数。
//
// 三种 provider:
// - gptzero:     POST {api_url}  X-Api-Key 头, body {document}, 取 documents[0].completely_generated_prob
// - originality: POST {api_url}  Authorization Bearer, body {content}, 取 score.ai
// - custom:      POST {api_url}  Authorization Bearer, body {content}, 取 score
//
// 网络请求统一 30s 超时。非 2xx 响应显式报错(无静默回退)。

use crate::shared::error::AppError;

use super::types::DetectionResult;

const REQUEST_TIMEOUT_SECS: u64 = 30;

/// 各 provider 的默认 endpoint(前端未传 api_url 时使用)。
pub fn default_api_url(provider: &str) -> Option<&'static str> {
    match provider {
        "gptzero" => Some("https://api.gptzero.me/v2/predict/text"),
        "originality" => Some("https://api.originality.ai/api/v1/scan/ai"),
        _ => None, // custom 必须由前端显式传 api_url
    }
}

/// 调用外部检测 API。`api_key` 由命令层从 secrets 解析后传入。
pub async fn detect_ai_content(
    provider: &str,
    api_url: &str,
    api_key: &str,
    content: &str,
) -> Result<DetectionResult, AppError> {
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

    match provider {
        "gptzero" => detect_gptzero(&client, api_url, api_key, content, &detected_at).await,
        "originality" => detect_originality(&client, api_url, api_key, content, &detected_at).await,
        "custom" => detect_custom(&client, api_url, api_key, content, &detected_at).await,
        other => Err(AppError::invalid_input(format!(
            "Unsupported provider: {} (expected gptzero/originality/custom)",
            other
        ))),
    }
}

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
        .unwrap_or(0.0);
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
        .unwrap_or(0.0);
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
    // custom endpoint 至少返回 { score: number }
    let score = data
        .get("score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Ok(DetectionResult {
        score,
        provider: "custom".into(),
        detected_at: detected_at.to_string(),
        details: Some(data),
    })
}

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
