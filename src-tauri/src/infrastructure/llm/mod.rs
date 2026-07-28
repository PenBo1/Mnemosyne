//! ═══════════════════════════════════════════════════════════════════════════
//! LLM 模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供多 LLM 提供商的统一接口：
//! - OpenAI API
//! - Anthropic Claude
//! - Ollama 本地模型
//! - Agnes（自定义）
//!
//! 支持流式响应、工具调用、嵌入向量生成。

pub mod types;
pub mod tool;
pub mod openai_protocol;
pub mod openai;
pub mod ollama;
pub mod agnes;
pub mod anthropic;
pub mod presets;
pub mod registry;
pub mod commands;
pub mod state;
pub mod embedding;
pub mod sse_buffer;
pub mod client;

use crate::shared::error::{status, AppError};

/// 将 LLM 流式请求的非 2xx HTTP 响应映射为 AppError
///
/// 状态码映射：
/// - 401 → API_KEY_INVALID
/// - 429 → RATE_LIMITED
/// - 其他 4xx/5xx → INTERNAL_ERROR
pub(crate) fn map_stream_http_error(http_status: reqwest::StatusCode, body: &str) -> AppError {
    match http_status.as_u16() {
        401 => AppError::new(
            status::API_KEY_INVALID,
            "API_KEY_INVALID",
            format!("LLM API returned 401: {}", body),
        ),
        429 => AppError::rate_limited(None, format!("LLM API rate limited (429): {}", body)),
        _ => AppError::internal(format!("LLM API returned {}: {}", http_status, body)),
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_401_to_api_key_invalid() {
        let err = map_stream_http_error(reqwest::StatusCode::UNAUTHORIZED, "invalid api key");
        assert_eq!(err.status, status::API_KEY_INVALID);
        assert_eq!(err.code, "API_KEY_INVALID");
        assert!(err.message.contains("401"), "message should contain 401: {}", err.message);
        assert!(err.message.contains("invalid api key"), "message should contain body: {}", err.message);
    }

    #[test]
    fn map_429_to_rate_limited_with_retry_keywords() {
        let err = map_stream_http_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "slow down");
        assert_eq!(err.status, status::RATE_LIMITED);
        assert!(err.message.contains("rate limit"), "message should contain 'rate limit': {}", err.message);
        assert!(err.message.contains("429"), "message should contain '429': {}", err.message);
        assert!(err.message.contains("slow down"), "message should contain body: {}", err.message);
    }

    #[test]
    fn map_500_to_internal_error() {
        let err = map_stream_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "boom");
        assert_eq!(err.status, status::INTERNAL_ERROR);
        assert!(err.message.contains("500"), "message should contain 500: {}", err.message);
        assert!(err.message.contains("boom"), "message should contain body: {}", err.message);
    }

    #[test]
    fn map_other_4xx_to_internal_error() {
        let err = map_stream_http_error(reqwest::StatusCode::FORBIDDEN, "no access");
        assert_eq!(err.status, status::INTERNAL_ERROR);
        assert!(err.message.contains("403"), "message should contain 403: {}", err.message);
        assert!(err.message.contains("no access"), "message should contain body: {}", err.message);
    }
}