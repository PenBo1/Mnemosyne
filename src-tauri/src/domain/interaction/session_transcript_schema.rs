use crate::errors::AppError;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Session transcript schema for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMessage {
    pub role: TranscriptRole,
    pub content: String,
    pub thinking: Option<String>,
    pub tool_call_id: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptRole {
    User,
    Assistant,
    ToolResult,
    System,
}

/// Session transcript for a book session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSessionTranscript {
    pub session_id: String,
    pub book_id: String,
    pub messages: Vec<TranscriptMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl BookSessionTranscript {
    pub fn new(session_id: String, book_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            session_id,
            book_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn append(&mut self, role: TranscriptRole, content: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.messages.push(TranscriptMessage {
            role,
            content,
            thinking: None,
            tool_call_id: None,
            timestamp: now,
        });
        self.updated_at = now;
    }
}

/// Save book session transcript to disk
pub fn save_book_session_transcript(
    project_root: &str,
    session_id: &str,
    transcript: &BookSessionTranscript,
) -> Result<(), AppError> {
    let dir = format!("{}/sessions/{}", project_root, session_id);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::internal(format!("Failed to create dir: {}", e)))?;

    let path = format!("{}/transcript.json", dir);
    let json = serde_json::to_string_pretty(transcript)
        .map_err(|e| AppError::internal(format!("Failed to serialize: {}", e)))?;
    std::fs::write(&path, json)
        .map_err(|e| AppError::internal(format!("Failed to write: {}", e)))?;
    Ok(())
}

/// Load book session transcript from disk
pub fn load_book_session_transcript(
    project_root: &str,
    session_id: &str,
) -> Result<Option<BookSessionTranscript>, AppError> {
    let path = format!("{}/sessions/{}/transcript.json", project_root, session_id);
    if !Path::new(&path).exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::internal(format!("Failed to read: {}", e)))?;
    let transcript = serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse: {}", e)))?;
    Ok(Some(transcript))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_creation() {
        let t = BookSessionTranscript::new("s1".into(), "b1".into());
        assert_eq!(t.session_id, "s1");
        assert_eq!(t.book_id, "b1");
        assert!(t.messages.is_empty());
    }

    #[test]
    fn test_transcript_append() {
        let mut t = BookSessionTranscript::new("s1".into(), "b1".into());
        t.append(TranscriptRole::User, "Hello".into());
        assert_eq!(t.messages.len(), 1);
        assert_eq!(t.messages[0].role, TranscriptRole::User);
    }
}
