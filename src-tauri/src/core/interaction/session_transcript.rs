use serde::{Deserialize, Serialize};

/// Session transcript event for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEvent {
    pub event_type: String,
    pub version: u32,
    pub session_id: String,
    pub request_id: Option<String>,
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub seq: u64,
    pub role: Option<String>,
    pub timestamp: u64,
    pub message: Option<serde_json::Value>,
    pub input: Option<String>,
    pub book_id: Option<String>,
    pub session_kind: Option<String>,
    pub title: Option<String>,
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
}

/// Read transcript events from disk
pub fn read_transcript_events(project_root: &str, session_id: &str) -> Vec<TranscriptEvent> {
    let path = format!("{}/sessions/{}/transcript.jsonl", project_root, session_id);
    if let Ok(content) = std::fs::read_to_string(&path) {
        content.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    } else {
        Vec::new()
    }
}

/// Append transcript events to disk
pub fn append_transcript_events(
    project_root: &str,
    session_id: &str,
    events: &[TranscriptEvent],
) -> Result<(), crate::shared::errors::AppError> {
    let dir = format!("{}/sessions/{}", project_root, session_id);
    std::fs::create_dir_all(&dir).map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to create session dir: {}", e)))?;

    let path = format!("{}/transcript.jsonl", dir);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to open transcript: {}", e)))?;

    use std::io::Write;
    for event in events {
        let line = serde_json::to_string(event)
            .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to serialize event: {}", e)))?;
        writeln!(file, "{}", line)
            .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to write event: {}", e)))?;
    }

    Ok(())
}

/// Get the next sequence number for a session
pub fn next_transcript_seq(project_root: &str, session_id: &str) -> u64 {
    let events = read_transcript_events(project_root, session_id);
    events.iter().map(|e| e.seq).max().unwrap_or(0) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_transcript_empty() {
        let events = read_transcript_events("/nonexistent", "test");
        assert!(events.is_empty());
    }

    #[test]
    fn test_next_transcript_seq_empty() {
        let seq = next_transcript_seq("/nonexistent", "test");
        assert_eq!(seq, 1);
    }
}
