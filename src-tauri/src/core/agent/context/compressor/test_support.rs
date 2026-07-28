//! ═══════════════════════════════════════════════════════════════════════════
//! Compressor Test Support - 测试辅助工具
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 共享测试辅助: mock LLM、消息构造 helper、压缩器构造 helper。

use std::sync::Arc;

use async_trait::async_trait;

use crate::infrastructure::llm::types::{Message, ToolCallRequest};
use crate::shared::error::AppError;

use super::summary::SummaryLlm;
use super::ContextCompressor;

// ── Mock LLM ────────────────────────────────────────────────────────────────

/// Mock LLM: 可配置返回内容或失败
pub struct MockSummaryLlm {
    pub response: Result<String, AppError>,
}

impl MockSummaryLlm {
    /// 创建返回成功的 Mock
    pub fn ok(response: &str) -> Arc<Self> {
        Arc::new(Self {
            response: Ok(response.to_string()),
        })
    }

    /// 创建返回错误的 Mock
    pub fn err() -> Arc<Self> {
        Arc::new(Self {
            response: Err(AppError::internal("mock LLM unavailable")),
        })
    }

    /// 创建返回临时错误的 Mock
    pub fn err_transient() -> Arc<Self> {
        Arc::new(Self {
            response: Err(AppError::network_timeout()),
        })
    }
}

#[async_trait]
impl SummaryLlm for MockSummaryLlm {
    async fn summarize(&self, _system: &str, _user: &str) -> Result<String, AppError> {
        self.response.clone()
    }
}

/// 捕获 LLM: 记录每次调用参数
pub struct CapturingSummaryLlm {
    pub response: Result<String, AppError>,
    pub captured: Arc<std::sync::Mutex<Vec<(String, String)>>>,
}

impl CapturingSummaryLlm {
    /// 创建返回成功的捕获 LLM
    pub fn ok(response: &str) -> Arc<Self> {
        Arc::new(Self {
            response: Ok(response.to_string()),
            captured: Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl SummaryLlm for CapturingSummaryLlm {
    async fn summarize(&self, system: &str, user: &str) -> Result<String, AppError> {
        self.captured
            .lock()
            .unwrap()
            .push((system.to_string(), user.to_string()));
        self.response.clone()
    }
}

// ── 消息构造辅助 ────────────────────────────────────────────────────────────

/// 创建用户消息
pub fn user_msg(content: &str) -> Message {
    Message {
        role: "user".to_string(),
        content: content.to_string(),
        tool_calls: None,
        tool_call_id: None,
    }
}

/// 创建助手消息
pub fn assistant_msg(content: &str) -> Message {
    Message {
        role: "assistant".to_string(),
        content: content.to_string(),
        tool_calls: None,
        tool_call_id: None,
    }
}

/// 创建工具消息
pub fn tool_msg(tool_call_id: &str, content: &str) -> Message {
    Message {
        role: "tool".to_string(),
        content: content.to_string(),
        tool_calls: None,
        tool_call_id: Some(tool_call_id.to_string()),
    }
}

/// 创建带工具调用的助手消息
pub fn assistant_with_tool_call(id: &str, name: &str) -> Message {
    Message {
        role: "assistant".to_string(),
        content: String::new(),
        tool_calls: Some(vec![ToolCallRequest {
            id: id.to_string(),
            name: name.to_string(),
            arguments: "{}".to_string(),
        }]),
        tool_call_id: None,
    }
}

/// 构建压缩器实例 (context_length = 10000, 阈值 5000 tokens)
pub fn build_compressor(llm: Arc<dyn SummaryLlm>) -> ContextCompressor {
    ContextCompressor::new(10_000, llm)
}