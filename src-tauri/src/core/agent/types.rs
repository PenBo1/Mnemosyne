use serde::{Deserialize, Serialize};

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
}
