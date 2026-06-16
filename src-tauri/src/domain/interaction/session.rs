use serde::{Deserialize, Serialize};

/// Session kind — determines the conversation surface and available tools
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionKind {
    Chat,
    BookCreate,
    Book,
    Short,
    Play,
    Edit,
}

/// Play mode — determines interaction style
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayMode {
    Open,
    Guided,
}

/// Pending decision that needs user confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDecision {
    pub kind: String,
    pub book_id: String,
    pub chapter_number: Option<u32>,
    pub summary: String,
}

/// Pipeline stage status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub label: String,
    pub status: PipelineStageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStageStatus {
    Pending,
    Active,
    Completed,
}

/// Tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: String,
    pub tool: String,
    pub agent: Option<String>,
    pub label: String,
    pub status: ToolExecutionStatus,
    pub args: Option<serde_json::Value>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub stages: Option<Vec<PipelineStage>>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolExecutionStatus {
    Running,
    Processing,
    Completed,
    Error,
}

/// Interaction message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionMessage {
    pub role: MessageRole,
    pub content: String,
    pub thinking: Option<String>,
    pub tool_executions: Option<Vec<ToolExecution>>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Book creation draft
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookCreationDraft {
    pub concept: String,
    pub title: Option<String>,
    pub genre: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub target_chapters: Option<u32>,
    pub chapter_word_count: Option<u32>,
    pub blurb: Option<String>,
    pub world_premise: Option<String>,
    pub setting_notes: Option<String>,
    pub protagonist: Option<String>,
    pub supporting_cast: Option<String>,
    pub conflict_core: Option<String>,
    pub volume_outline: Option<String>,
    pub constraints: Option<String>,
    pub author_intent: Option<String>,
    pub current_focus: Option<String>,
    pub next_question: Option<String>,
    pub missing_fields: Vec<String>,
    pub ready_to_create: bool,
}

/// Draft round for collaborative book creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftRound {
    pub round: u32,
    pub message: String,
    pub timestamp: u64,
}

/// Interaction session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionSession {
    pub session_id: String,
    pub kind: SessionKind,
    pub book_id: Option<String>,
    pub messages: Vec<InteractionMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl InteractionSession {
    pub fn new(session_id: String, kind: SessionKind, book_id: Option<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            session_id,
            kind,
            book_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn append_message(&mut self, role: MessageRole, content: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.messages.push(InteractionMessage {
            role,
            content,
            thinking: None,
            tool_executions: None,
            timestamp: now,
        });
        self.updated_at = now;
    }
}

/// Book session with pipeline state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSession {
    pub session_id: String,
    pub book_id: String,
    pub kind: SessionKind,
    pub messages: Vec<InteractionMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl BookSession {
    pub fn new(session_id: String, book_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            session_id,
            book_id,
            kind: SessionKind::Book,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Global session for cross-book interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSession {
    pub session_id: String,
    pub messages: Vec<InteractionMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = InteractionSession::new("test-1".into(), SessionKind::Chat, None);
        assert_eq!(session.session_id, "test-1");
        assert_eq!(session.kind, SessionKind::Chat);
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_append_message() {
        let mut session = InteractionSession::new("test-1".into(), SessionKind::Book, Some("book-1".into()));
        session.append_message(MessageRole::User, "Hello".into());
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, MessageRole::User);
    }

    #[test]
    fn test_book_session() {
        let session = BookSession::new("s1".into(), "b1".into());
        assert_eq!(session.kind, SessionKind::Book);
        assert_eq!(session.book_id, "b1");
    }
}
