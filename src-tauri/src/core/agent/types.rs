use serde::{Deserialize, Serialize};

use super::collaboration_style::CollaborationStyle;
use super::effort::EffortLevel;

/// Events streamed from the agent engine to the frontend via Tauri Channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ChatEvent {
    TextDelta {
        content: String,
    },
    ReasoningDelta {
        content: String,
    },
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        args_delta: String,
    },
    ToolCallEnd {
        id: String,
    },
    ToolApprovalRequired {
        request_id: String,
        name: String,
        args: serde_json::Value,
    },
    Finish {
        input_tokens: u32,
        output_tokens: u32,
    },
    Error {
        message: String,
    },
}

/// Request payload for chat_send_message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub session_id: String,
    pub content: String,
    pub context_text: Option<String>,
    pub custom_instructions: Option<String>,
    /// Effort 级别覆盖(可选,缺省使用 settings 中的默认值)
    ///
    /// 设计:用户可在 settings 中设全局默认,也可在每次发送消息时临时调整。
    /// 例如日常对话用 Low,复杂任务用 High。
    #[serde(default)]
    pub effort: Option<EffortLevel>,
    /// 协作风格覆盖(可选,缺省使用 Efficient)
    ///
    /// 设计:与 Effort 正交 —— Effort 控制"做多少",Style 控制"怎么做"。
    /// 风格对应的 prompt 片段会合并到 custom_instructions 中注入 system prompt。
    #[serde(default)]
    pub collaboration_style: Option<CollaborationStyle>,
}
