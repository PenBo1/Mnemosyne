//! ═══════════════════════════════════════════════════════════════════════════
//! LLM 类型 - 通用数据类型
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型 ID
    pub id: String,
    /// Provider 名称
    pub provider: String,
    /// 模型名称
    pub name: String,
    /// 上下文窗口大小
    pub context_window: usize,
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 是否支持流式输出
    pub supports_streaming: bool,
}

/// 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 角色（user/assistant/system/tool）
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 工具调用请求
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRequest>>,
    /// 工具调用 ID（tool 角色时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// 调用 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 字符串）
    pub arguments: String,
}

/// 工具规格定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数 JSON Schema
    pub parameters: serde_json::Value,
}

/// 流式事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// 文本增量
    TextDelta { content: String },
    /// 推理增量（如 Claude 的 extended thinking）
    ReasoningDelta { content: String },
    /// 工具调用开始
    ToolCallStart { id: String, name: String },
    /// 工具调用参数增量
    ToolCallDelta { id: String, args_delta: String },
    /// 工具调用结束
    ToolCallEnd { id: String },
    /// 完成事件
    Finish { reason: FinishReason, usage: TokenUsage },
    /// 错误事件
    Error(String),
}

/// 完成原因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    /// 正常结束
    Stop,
    /// 工具调用
    ToolCalls,
    /// 达到长度限制
    Length,
    /// 内容过滤
    ContentFilter,
}

/// Token 使用统计
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入 Token 数
    pub input_tokens: u32,
    /// 输出 Token 数
    pub output_tokens: u32,
}

/// Anthropic prompt cache TTL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheTtl {
    /// 5 分钟（默认）
    #[default]
    FiveMinutes,
    /// 1 小时（需 beta header）
    OneHour,
}

impl CacheTtl {
    /// 序列化为 API 字段值
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheTtl::FiveMinutes => "5m",
            CacheTtl::OneHour => "1h",
        }
    }

    /// 对应的 beta header 值
    pub fn beta_header(&self) -> Option<&'static str> {
        match self {
            CacheTtl::FiveMinutes => None,
            CacheTtl::OneHour => Some("extended-cache-ttl-2025-04-11"),
        }
    }
}

/// Provider Trait
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Provider 名称
    fn name(&self) -> &str;
    /// 可用模型列表
    fn models(&self) -> Vec<ModelInfo>;
    /// API 密钥
    fn api_key(&self) -> &str;
    /// API 基础 URL
    fn base_url(&self) -> &str;

    /// 非流式完成
    async fn complete(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        max_tokens: u64,
    ) -> Result<String, crate::shared::error::AppError>;

    /// 非流式完成（带工具调用支持）
    async fn complete_with_tools(
        &self,
        _model: &str,
        _system: &str,
        _messages: &[Message],
        _tools: &[ToolSpec],
        _max_tokens: u64,
    ) -> Result<String, crate::shared::error::AppError> {
        Err(crate::shared::error::AppError::internal(
            "complete_with_tools not supported by this provider"
        ))
    }

    /// 流式完成
    async fn stream(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
        max_tokens: u64,
    ) -> Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>,
        crate::shared::error::AppError,
    >;

    /// 测试连接
    async fn test_connection(&self) -> Result<(), crate::shared::error::AppError>;
}