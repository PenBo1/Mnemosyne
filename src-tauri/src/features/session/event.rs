use serde::{Deserialize, Serialize};

/// Events emitted by the agent session back to clients.
///
/// Modeled after Codex CLI's SQ/EQ pattern:
/// - The agent emits `Event` values on the event queue
/// - Clients receive them and update UI/state accordingly
/// - Each event carries the submission ID that triggered it (for correlation)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Event {
    // ── Session lifecycle ──────────────────────────────────────

    /// Session has been configured and is ready
    SessionConfigured(SessionConfiguredPayload),

    /// Session is shutting down
    SessionShutdown,

    // ── Turn lifecycle ─────────────────────────────────────────

    /// A new turn (user input processing) has started
    TurnStarted(TurnStartedPayload),

    /// The turn has completed successfully
    TurnCompleted(TurnCompletedPayload),

    /// The turn failed with an error
    TurnFailed(TurnFailedPayload),

    // ── Streaming ──────────────────────────────────────────────

    /// A text delta from the streaming LLM response
    StreamDelta(StreamDeltaPayload),

    // ── Tool execution ─────────────────────────────────────────

    /// A tool call has started
    ToolCallBegin(ToolCallBeginPayload),

    /// A tool call has completed
    ToolCallEnd(ToolCallEndPayload),

    /// Tool execution needs user approval
    ToolApprovalRequest(ToolApprovalRequestPayload),

    /// Tool approval was granted
    ToolApprovalGranted(ToolApprovalGrantedPayload),

    /// Tool approval was rejected
    ToolApprovalRejected(ToolApprovalRejectedPayload),

    // ── Pipeline stages ────────────────────────────────────────

    /// Pipeline stage started (plan, compose, write, audit, revise)
    PipelineStageStarted(PipelineStageStartedPayload),

    /// Pipeline stage completed
    PipelineStageCompleted(PipelineStageCompletedPayload),

    /// Pipeline stage failed
    PipelineStageFailed(PipelineStageFailedPayload),

    /// Full pipeline completed
    PipelineCompleted(PipelineCompletedPayload),

    // ── Progress ───────────────────────────────────────────────

    /// General progress update
    Progress(ProgressPayload),

    /// Context compaction was triggered
    CompactionTriggered(CompactionTriggeredPayload),

    // ── Error ──────────────────────────────────────────────────

    /// A non-fatal error occurred
    Error(ErrorPayload),
}

// ── Payload types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfiguredPayload {
    pub session_id: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStartedPayload {
    pub session_id: String,
    pub submission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCompletedPayload {
    pub session_id: String,
    pub submission_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFailedPayload {
    pub session_id: String,
    pub submission_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDeltaPayload {
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallBeginPayload {
    pub session_id: String,
    pub tool_call_id: String,
    pub tool: String,
    pub args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEndPayload {
    pub session_id: String,
    pub tool_call_id: String,
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalRequestPayload {
    pub session_id: String,
    pub tool_call_id: String,
    pub tool: String,
    pub args: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalGrantedPayload {
    pub session_id: String,
    pub tool_call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalRejectedPayload {
    pub session_id: String,
    pub tool_call_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageStartedPayload {
    pub book_id: String,
    pub chapter_number: u32,
    pub stage: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageCompletedPayload {
    pub book_id: String,
    pub chapter_number: u32,
    pub stage: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageFailedPayload {
    pub book_id: String,
    pub chapter_number: u32,
    pub stage: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineCompletedPayload {
    pub book_id: String,
    pub chapter_number: u32,
    pub word_count: u32,
    pub audit_passed: bool,
    pub elapsed_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub message: String,
    pub percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionTriggeredPayload {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub session_id: Option<String>,
    pub error: String,
    pub fatal: bool,
}

impl Event {
    /// Check if this is a terminal event (session ended)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Event::SessionShutdown | Event::TurnFailed(_))
    }

    /// Get the session ID from the event, if applicable
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Event::SessionConfigured(p) => Some(&p.session_id),
            Event::TurnStarted(p) => Some(&p.session_id),
            Event::TurnCompleted(p) => Some(&p.session_id),
            Event::TurnFailed(p) => Some(&p.session_id),
            Event::StreamDelta(p) => Some(&p.session_id),
            Event::ToolCallBegin(p) => Some(&p.session_id),
            Event::ToolCallEnd(p) => Some(&p.session_id),
            Event::ToolApprovalRequest(p) => Some(&p.session_id),
            Event::ToolApprovalGranted(p) => Some(&p.session_id),
            Event::ToolApprovalRejected(p) => Some(&p.session_id),
            Event::CompactionTriggered(p) => Some(&p.session_id),
            Event::Error(p) => p.session_id.as_deref(),
            _ => None,
        }
    }
}
