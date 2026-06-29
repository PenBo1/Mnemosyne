//! API 错误分类器 — 智能降级和恢复。
//!
//! 移植自 Hermes Agent 的 `agent/error_classifier.py`。
//! 提供结构化的错误分类法和优先级有序的分类管线，
//! 确定正确的恢复策略（重试、轮换凭证、降级到其他模型、压缩上下文或中止）。

use serde::{Serialize, Deserialize};

// ── 错误分类法 ──────────────────────────────────────────────────

/// 错误降级原因 — 决定恢复策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverReason {
    /// 认证/授权失败 — 刷新/轮换凭证
    Auth,
    /// 认证永久失败 — 中止
    AuthPermanent,
    /// 额度耗尽 — 立即轮换
    Billing,
    /// 限流(429) — 退避后轮换
    RateLimit,
    /// 服务过载(503/529) — 退避
    Overloaded,
    /// 服务器错误(500/502) — 重试
    ServerError,
    /// 超时 — 重建客户端+重试
    Timeout,
    /// 上下文溢出 — 压缩（非降级）
    ContextOverflow,
    /// 负载过大(413) — 压缩负载
    PayloadTooLarge,
    /// 模型不存在 — 降级到其他模型
    ModelNotFound,
    /// 内容策略拦截 — 不重试
    ContentPolicyBlocked,
    /// 请求格式错误(400) — 中止或剥离后重试
    FormatError,
    /// 未知 — 带退避重试
    Unknown,
}

impl FailoverReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::AuthPermanent => "auth_permanent",
            Self::Billing => "billing",
            Self::RateLimit => "rate_limit",
            Self::Overloaded => "overloaded",
            Self::ServerError => "server_error",
            Self::Timeout => "timeout",
            Self::ContextOverflow => "context_overflow",
            Self::PayloadTooLarge => "payload_too_large",
            Self::ModelNotFound => "model_not_found",
            Self::ContentPolicyBlocked => "content_policy_blocked",
            Self::FormatError => "format_error",
            Self::Unknown => "unknown",
        }
    }
}

// ── 分类结果 ──────────────────────────────────────────────────

/// 分类后的 API 错误 — 包含恢复建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedError {
    /// 错误分类原因
    pub reason: FailoverReason,
    /// HTTP 状态码
    pub status_code: Option<u16>,
    /// 提供者名称
    pub provider: String,
    /// 模型名称
    pub model: String,
    /// 错误消息
    pub message: String,
    /// 是否可重试
    pub retryable: bool,
    /// 是否应压缩上下文
    pub should_compress: bool,
    /// 是否应轮换凭证
    pub should_rotate_credential: bool,
    /// 是否应降级到其他模型
    pub should_fallback: bool,
}

impl ClassifiedError {
    /// 是否为认证错误
    pub fn is_auth(&self) -> bool {
        matches!(self.reason, FailoverReason::Auth | FailoverReason::AuthPermanent)
    }
}

// ── 模式匹配数组 ──────────────────────────────────────────────────

/// 计费耗尽模式（非瞬时限流）
const BILLING_PATTERNS: &[&str] = &[
    "insufficient credits", "insufficient_quota", "insufficient_balance",
    "credit balance", "credits exhausted", "no usable credits",
    "top up your credits", "payment required", "billing hard limit",
    "exceeded your current quota", "account is deactivated",
    "plan does not include", "out of funds", "run out of funds",
    "balance_depleted",
];

/// 限流模式（瞬时，会恢复）
const RATE_LIMIT_PATTERNS: &[&str] = &[
    "rate limit", "rate_limit", "too many requests", "throttled",
    "requests per minute", "tokens per minute", "requests per day",
    "try again in", "please retry after", "resource_exhausted",
];

/// 上下文溢出模式
const CONTEXT_OVERFLOW_PATTERNS: &[&str] = &[
    "context length", "context size", "maximum context", "token limit",
    "too many tokens", "reduce the length", "exceeds the limit",
    "context window", "prompt is too long", "prompt exceeds max length",
    "max_tokens", "maximum number of tokens", "exceeds the max_model_len",
    "max_model_len", "input is too long", "context length exceeded",
    "超过最大长度", "上下文长度",
];

/// 模型不存在模式
const MODEL_NOT_FOUND_PATTERNS: &[&str] = &[
    "is not a valid model", "invalid model", "model not found",
    "model_not_found", "does not exist", "no such model",
    "unknown model", "unsupported model",
];

/// 认证模式
const AUTH_PATTERNS: &[&str] = &[
    "invalid api key", "invalid_api_key", "authentication",
    "unauthorized", "forbidden", "invalid token", "token expired",
    "token revoked", "access denied",
];

/// 内容策略拦截模式
const CONTENT_POLICY_PATTERNS: &[&str] = &[
    "flagged for possible cybersecurity risk",
    "violates our usage policies", "violates openai's usage policies",
    "prompt was flagged by our safety",
    "responses cannot be generated due to safety",
    "content_filter", "responsibleaipolicyviolation",
];

/// 负载过大模式
const PAYLOAD_TOO_LARGE_PATTERNS: &[&str] = &[
    "request entity too large", "payload too large", "error code: 413",
];

// ── 分类管线 ──────────────────────────────────────────────────

/// 分类 API 错误 — 优先级有序的分类管线
///
/// # 参数
/// - `error_message`: 错误消息文本
/// - `status_code`: HTTP 状态码（可选）
/// - `provider`: 当前提供者名称
/// - `model`: 当前模型名称
pub fn classify_api_error(
    error_message: &str,
    status_code: Option<u16>,
    provider: &str,
    model: &str,
) -> ClassifiedError {
    let error_msg = error_message.to_lowercase();

    // 1. 内容策略拦截（最高优先级）
    if contains_any(&error_msg, CONTENT_POLICY_PATTERNS) {
        return ClassifiedError {
            reason: FailoverReason::ContentPolicyBlocked,
            status_code,
            provider: provider.to_string(),
            model: model.to_string(),
            message: error_message.to_string(),
            retryable: false,
            should_compress: false,
            should_rotate_credential: false,
            should_fallback: true,
        };
    }

    // 2. HTTP 状态码分类
    if let Some(status) = status_code {
        if let Some(classified) = classify_by_status(status, &error_msg, provider, model, error_message) {
            return classified;
        }
    }

    // 3. 消息模式匹配（无状态码时）
    if let Some(classified) = classify_by_message(&error_msg, provider, model, error_message) {
        return classified;
    }

    // 4. 回退：未知
    ClassifiedError {
        reason: FailoverReason::Unknown,
        status_code,
        provider: provider.to_string(),
        model: model.to_string(),
        message: error_message.to_string(),
        retryable: true,
        should_compress: false,
        should_rotate_credential: false,
        should_fallback: false,
    }
}

// ── 按状态码分类 ──────────────────────────────────────────────

fn classify_by_status(
    status: u16,
    error_msg: &str,
    provider: &str,
    model: &str,
    raw_message: &str,
) -> Option<ClassifiedError> {
    let make = |reason, retryable, compress, rotate, fallback| {
        ClassifiedError {
            reason,
            status_code: Some(status),
            provider: provider.to_string(),
            model: model.to_string(),
            message: raw_message.to_string(),
            retryable,
            should_compress: compress,
            should_rotate_credential: rotate,
            should_fallback: fallback,
        }
    };

    match status {
        401 => Some(make(FailoverReason::Auth, false, false, true, true)),
        403 => {
            if contains_any(error_msg, BILLING_PATTERNS) {
                Some(make(FailoverReason::Billing, false, false, true, true))
            } else {
                Some(make(FailoverReason::Auth, false, false, false, true))
            }
        }
        402 => Some(make(FailoverReason::Billing, false, false, true, true)),
        404 => {
            if contains_any(error_msg, MODEL_NOT_FOUND_PATTERNS) {
                Some(make(FailoverReason::ModelNotFound, false, false, false, true))
            } else if contains_any(error_msg, BILLING_PATTERNS) {
                Some(make(FailoverReason::Billing, false, false, true, true))
            } else {
                Some(make(FailoverReason::Unknown, true, false, false, false))
            }
        }
        413 => Some(make(FailoverReason::PayloadTooLarge, true, true, false, false)),
        429 => Some(make(FailoverReason::RateLimit, true, false, true, true)),
        400 => classify_400(error_msg, provider, model, raw_message),
        500 | 502 => Some(make(FailoverReason::ServerError, true, false, false, false)),
        503 | 529 => Some(make(FailoverReason::Overloaded, true, false, false, false)),
        _ if status >= 400 && status < 500 => Some(make(FailoverReason::FormatError, false, false, false, true)),
        _ if status >= 500 && status < 600 => Some(make(FailoverReason::ServerError, true, false, false, false)),
        _ => None,
    }
}

// ── 400 错误细分 ──────────────────────────────────────────────

fn classify_400(
    error_msg: &str,
    provider: &str,
    model: &str,
    raw_message: &str,
) -> Option<ClassifiedError> {
    let make = |reason, retryable, compress, rotate, fallback| {
        ClassifiedError {
            reason,
            status_code: Some(400),
            provider: provider.to_string(),
            model: model.to_string(),
            message: raw_message.to_string(),
            retryable,
            should_compress: compress,
            should_rotate_credential: rotate,
            should_fallback: fallback,
        }
    };

    if contains_any(error_msg, CONTEXT_OVERFLOW_PATTERNS) {
        Some(make(FailoverReason::ContextOverflow, true, true, false, false))
    } else if contains_any(error_msg, MODEL_NOT_FOUND_PATTERNS) {
        Some(make(FailoverReason::ModelNotFound, false, false, false, true))
    } else if contains_any(error_msg, RATE_LIMIT_PATTERNS) {
        Some(make(FailoverReason::RateLimit, true, false, true, true))
    } else if contains_any(error_msg, BILLING_PATTERNS) {
        Some(make(FailoverReason::Billing, false, false, true, true))
    } else {
        Some(make(FailoverReason::FormatError, false, false, false, true))
    }
}

// ── 按消息模式分类 ──────────────────────────────────────────

fn classify_by_message(
    error_msg: &str,
    provider: &str,
    model: &str,
    raw_message: &str,
) -> Option<ClassifiedError> {
    let make = |reason, retryable, compress, rotate, fallback| {
        ClassifiedError {
            reason,
            status_code: None,
            provider: provider.to_string(),
            model: model.to_string(),
            message: raw_message.to_string(),
            retryable,
            should_compress: compress,
            should_rotate_credential: rotate,
            should_fallback: fallback,
        }
    };

    if contains_any(error_msg, PAYLOAD_TOO_LARGE_PATTERNS) {
        Some(make(FailoverReason::PayloadTooLarge, true, true, false, false))
    } else if contains_any(error_msg, BILLING_PATTERNS) {
        Some(make(FailoverReason::Billing, false, false, true, true))
    } else if contains_any(error_msg, RATE_LIMIT_PATTERNS) {
        Some(make(FailoverReason::RateLimit, true, false, true, true))
    } else if contains_any(error_msg, CONTEXT_OVERFLOW_PATTERNS) {
        Some(make(FailoverReason::ContextOverflow, true, true, false, false))
    } else if contains_any(error_msg, AUTH_PATTERNS) {
        Some(make(FailoverReason::Auth, false, false, true, true))
    } else if contains_any(error_msg, MODEL_NOT_FOUND_PATTERNS) {
        Some(make(FailoverReason::ModelNotFound, false, false, false, true))
    } else {
        None
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────

/// 检查文本是否包含任一模式
fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_rate_limit() {
        let result = classify_api_error("Rate limit exceeded, try again in 60s", Some(429), "openai", "gpt-4");
        assert_eq!(result.reason, FailoverReason::RateLimit);
        assert!(result.retryable);
    }

    #[test]
    fn test_classify_auth() {
        let result = classify_api_error("Invalid API key", Some(401), "anthropic", "claude-3");
        assert_eq!(result.reason, FailoverReason::Auth);
        assert!(!result.retryable);
        assert!(result.should_rotate_credential);
    }

    #[test]
    fn test_classify_context_overflow() {
        let result = classify_api_error("Context length exceeded: max 128000 tokens", Some(400), "openai", "gpt-4");
        assert_eq!(result.reason, FailoverReason::ContextOverflow);
        assert!(result.should_compress);
    }

    #[test]
    fn test_classify_model_not_found() {
        let result = classify_api_error("model not found: gpt-99", Some(404), "openai", "gpt-99");
        assert_eq!(result.reason, FailoverReason::ModelNotFound);
        assert!(result.should_fallback);
    }

    #[test]
    fn test_classify_billing() {
        let result = classify_api_error("Insufficient credits", Some(402), "openai", "gpt-4");
        assert_eq!(result.reason, FailoverReason::Billing);
        assert!(!result.retryable);
    }

    #[test]
    fn test_classify_server_error() {
        let result = classify_api_error("Internal server error", Some(500), "openai", "gpt-4");
        assert_eq!(result.reason, FailoverReason::ServerError);
        assert!(result.retryable);
    }

    #[test]
    fn test_classify_content_policy() {
        let result = classify_api_error("Violates our usage policies", None, "openai", "gpt-4");
        assert_eq!(result.reason, FailoverReason::ContentPolicyBlocked);
        assert!(!result.retryable);
    }

    #[test]
    fn test_classify_unknown() {
        let result = classify_api_error("Something went wrong", None, "unknown", "model");
        assert_eq!(result.reason, FailoverReason::Unknown);
        assert!(result.retryable);
    }
}
