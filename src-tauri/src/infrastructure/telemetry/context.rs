//! ═══════════════════════════════════════════════════════════════════════════
//! W3C Trace Context - traceparent 解析与生成
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::AppError;

/// W3C traceparent 中的 span_id / trace_id 长度（hex 字符数）。
pub const TRACE_ID_LEN: usize = 32;
pub const SPAN_ID_LEN: usize = 16;

/// W3C Trace Context 解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W3cTraceContext {
    pub trace_id: String,
    pub parent_span_id: String,
    pub sampled: bool,
}

impl W3cTraceContext {
    /// 解析 traceparent header。
    ///
    /// 格式: `version-trace_id-parent_span_id-trace_flags`
    /// 仅支持 version "00"，其余版本返回 Err。
    pub fn parse(traceparent: &str) -> Result<Self, AppError> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() != 4 {
            return Err(AppError::bad_request(format!(
                "Invalid traceparent: expected 4 parts, got {}",
                parts.len()
            )));
        }
        let version = parts[0];
        let trace_id = parts[1];
        let parent_span_id = parts[2];
        let trace_flags = parts[3];

        if version != "00" {
            return Err(AppError::bad_request(format!(
                "Unsupported traceparent version '{}', only '00' supported",
                version
            )));
        }
        if trace_id.len() != TRACE_ID_LEN || !is_hex(trace_id) {
            return Err(AppError::bad_request(format!(
                "Invalid trace_id '{}': expected {} hex chars",
                trace_id, TRACE_ID_LEN
            )));
        }
        if trace_id == "0".repeat(TRACE_ID_LEN) {
            return Err(AppError::bad_request(
                "Invalid trace_id: all-zero not allowed",
            ));
        }
        if parent_span_id.len() != SPAN_ID_LEN || !is_hex(parent_span_id) {
            return Err(AppError::bad_request(format!(
                "Invalid parent_span_id '{}': expected {} hex chars",
                parent_span_id, SPAN_ID_LEN
            )));
        }
        if parent_span_id == "0".repeat(SPAN_ID_LEN) {
            return Err(AppError::bad_request(
                "Invalid parent_span_id: all-zero not allowed",
            ));
        }
        if trace_flags.len() != 2 || !is_hex(trace_flags) {
            return Err(AppError::bad_request(format!(
                "Invalid trace_flags '{}': expected 2 hex chars",
                trace_flags
            )));
        }
        let flags_byte = u8::from_str_radix(trace_flags, 16)
            .map_err(|e| AppError::bad_request(format!("Invalid trace_flags: {}", e)))?;
        let sampled = (flags_byte & 0x01) != 0;

        Ok(Self {
            trace_id: trace_id.to_lowercase(),
            parent_span_id: parent_span_id.to_lowercase(),
            sampled,
        })
    }

    /// 生成 traceparent header 字符串。
    pub fn to_header(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id, self.parent_span_id, flags)
    }
}

/// Trace context 传播器：用于跨进程/跨服务传递 trace 上下文。
///
/// 当前实现为内存中的 context 携带者；未来可扩展为 HTTP header / Tauri event 注入。
pub struct TraceContextPropagator;

impl TraceContextPropagator {
    /// 从 trace_id + 当前 span_id 构造传播上下文。
    pub fn inject(trace_id: &str, span_id: &str, sampled: bool) -> W3cTraceContext {
        W3cTraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: span_id.to_string(),
            sampled,
        }
    }

    /// 提取 traceparent header 中的上下文。
    pub fn extract(traceparent: &str) -> Result<W3cTraceContext, AppError> {
        W3cTraceContext::parse(traceparent)
    }
}

fn is_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_traceparent() {
        let ctx = W3cTraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01").unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_span_id, "b7ad6b7169203331");
        assert!(ctx.sampled);
    }

    #[test]
    fn parse_unsampled_traceparent() {
        let ctx = W3cTraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00").unwrap();
        assert!(!ctx.sampled);
    }

    #[test]
    fn parse_rejects_invalid_version() {
        assert!(W3cTraceContext::parse("01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01").is_err());
    }

    #[test]
    fn parse_rejects_short_trace_id() {
        assert!(W3cTraceContext::parse("00-short-b7ad6b7169203331-01").is_err());
    }

    #[test]
    fn parse_rejects_all_zero_trace_id() {
        let zero_trace = "0".repeat(TRACE_ID_LEN);
        let header = format!("00-{}-b7ad6b7169203331-01", zero_trace);
        assert!(W3cTraceContext::parse(&header).is_err());
    }

    #[test]
    fn to_header_roundtrips() {
        let ctx = W3cTraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            parent_span_id: "b7ad6b7169203331".to_string(),
            sampled: true,
        };
        let header = ctx.to_header();
        let parsed = W3cTraceContext::parse(&header).unwrap();
        assert_eq!(ctx, parsed);
    }

    #[test]
    fn propagator_inject_and_extract() {
        let ctx = TraceContextPropagator::inject(
            "0af7651916cd43dd8448eb211c80319c",
            "b7ad6b7169203331",
            true,
        );
        let header = ctx.to_header();
        let extracted = TraceContextPropagator::extract(&header).unwrap();
        assert_eq!(ctx, extracted);
    }
}
