use serde::{Deserialize, Serialize};

/// Unique identifier for a submission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubmissionId(pub String);

impl SubmissionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Operations that clients submit to the agent session.
///
/// Modeled after Codex CLI's SQ/EQ pattern:
/// - Clients send `Op` values via the submission queue
/// - The agent processes them sequentially in the submission loop
/// - Each `Op` produces zero or more `Event` values on the event queue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Op {
    /// Send a user message (interactive chat or pipeline command)
    UserInput(UserInputPayload),

    /// Cancel the current operation
    Interrupt(InterruptPayload),

    /// Gracefully shut down the session
    Shutdown,

    /// Approve a pending tool execution
    ApproveTool(ApproveToolPayload),

    /// Reject a pending tool execution
    RejectTool(RejectToolPayload),

    /// Trigger pipeline: write next chapter
    WriteNextChapter(WriteNextChapterPayload),

    /// Trigger pipeline: create a new book
    CreateBook(CreateBookPayload),

    /// Trigger pipeline: plan a chapter
    PlanChapter(PlanChapterPayload),

    /// Trigger pipeline: audit a chapter
    AuditChapter(AuditChapterPayload),

    /// Trigger pipeline: revise a chapter
    ReviseChapter(ReviseChapterPayload),
}

/// Payload for user input submissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputPayload {
    pub session_id: String,
    pub content: String,
}

/// Payload for interrupt submissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptPayload {
    pub reason: Option<String>,
}

/// Payload for tool approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveToolPayload {
    pub tool_call_id: String,
}

/// Payload for tool rejection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectToolPayload {
    pub tool_call_id: String,
    pub reason: Option<String>,
}

/// Payload for write next chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteNextChapterPayload {
    pub workspace_id: String,
    pub book_id: String,
    pub target_words: Option<u32>,
}

/// Payload for book creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookPayload {
    pub workspace_id: String,
    pub title: String,
    pub genre: String,
    pub brief: Option<String>,
}

/// Payload for chapter planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanChapterPayload {
    pub workspace_id: String,
    pub book_id: String,
    pub context: Option<String>,
}

/// Payload for chapter auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChapterPayload {
    pub workspace_id: String,
    pub book_id: String,
    pub chapter_number: u32,
}

/// Payload for chapter revision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseChapterPayload {
    pub workspace_id: String,
    pub book_id: String,
    pub chapter_number: u32,
}

/// A submission wraps an Op with a unique ID for tracking
#[derive(Debug, Clone)]
pub struct Submission {
    pub id: SubmissionId,
    pub op: Op,
}

impl Submission {
    pub fn new(op: Op) -> Self {
        Self {
            id: SubmissionId::new(),
            op,
        }
    }
}
