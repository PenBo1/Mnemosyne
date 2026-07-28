//! ═══════════════════════════════════════════════════════════════════════════
//! Types - Agent 类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

use super::collaboration_style::CollaborationStyle;
use super::effort::EffortLevel;

// ── 聊天事件 ────────────────────────────────────────────────────────────────

/// Agent 引擎向前端发送的流式事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ChatEvent {
    /// 文本增量
    TextDelta {
        content: String,
    },
    /// 推理增量
    ReasoningDelta {
        content: String,
    },
    /// 工具调用开始
    ToolCallStart {
        id: String,
        name: String,
    },
    /// 工具调用增量
    ToolCallDelta {
        id: String,
        args_delta: String,
    },
    /// 工具调用结束
    ToolCallEnd {
        id: String,
    },
    /// 工具审批请求
    ToolApprovalRequired {
        request_id: String,
        name: String,
        args: serde_json::Value,
    },
    /// 重试事件
    /// 
    /// LLM 流式调用因临时性错误 (503/429 等) 重试时派发,
    /// 让前端能在 UI 上分隔前次不完整输出与重试输出。
    Retry {
        attempt: u32,
        max_attempts: u32,
    },
    /// 完成事件
    Finish {
        input_tokens: u32,
        output_tokens: u32,
    },
    /// 错误事件
    Error {
        message: String,
    },
}

// ── 聊天请求 ────────────────────────────────────────────────────────────────

/// 聊天消息请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    /// 会话标识符
    pub session_id: String,
    /// 消息内容
    pub content: String,
    /// 上下文文本
    pub context_text: Option<String>,
    /// 自定义指令
    pub custom_instructions: Option<String>,
    /// Effort 级别覆盖 (可选, 缺省使用 settings 中的默认值)
    /// 
    /// 设计: 用户可在 settings 中设全局默认, 也可在每次发送消息时临时调整。
    /// 例如日常对话用 Low, 复杂任务用 High。
    #[serde(default)]
    pub effort: Option<EffortLevel>,
    /// 协作风格覆盖 (可选, 缺省使用 Efficient)
    /// 
    /// 设计: 与 Effort 正交 —— Effort 控制 "做多少", Style 控制 "怎么做"。
    /// 风格对应的 prompt 片段会合并到 custom_instructions 中注入 system prompt。
    #[serde(default)]
    pub collaboration_style: Option<CollaborationStyle>,
    /// 工具白名单 (可选, 缺省不二次过滤)
    /// 
    /// 设计: 与 EffortLevel 取交集。前端可传工具名列表收紧工具集,
    /// 仅保留列表中且 EffortLevel 允许的工具。None 时返回 EffortLevel 筛选后的完整集。
    #[serde(default)]
    pub tool_whitelist: Option<Vec<String>>,
}