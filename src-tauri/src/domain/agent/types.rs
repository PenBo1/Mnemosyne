use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    #[serde(rename = "TurnStarted")]
    TurnStarted {
        session_id: String,
    },
    #[serde(rename = "StreamDelta")]
    StreamDelta {
        session_id: String,
        content: String,
    },
    #[serde(rename = "ToolCallBegin")]
    ToolCallBegin {
        session_id: String,
        tool_call_id: String,
        tool: String,
        args: String,
    },
    #[serde(rename = "ToolCallEnd")]
    ToolCallEnd {
        session_id: String,
        tool_call_id: String,
        output: String,
        is_error: bool,
    },
    #[serde(rename = "TurnCompleted")]
    TurnCompleted {
        session_id: String,
        input_tokens: u32,
        output_tokens: u32,
    },
    #[serde(rename = "Error")]
    Error {
        session_id: String,
        error: String,
    },
    #[serde(rename = "CompactionTriggered")]
    CompactionTriggered {
        session_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct Submission {
    pub id: String,
    pub op: Op,
}

#[derive(Debug, Clone)]
pub enum Op {
    UserInput {
        session_id: String,
        content: String,
    },
    ToolApproval {
        tool_call_id: String,
        approved: bool,
    },
    Compact {
        session_id: String,
    },
    Cancel {
        session_id: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
