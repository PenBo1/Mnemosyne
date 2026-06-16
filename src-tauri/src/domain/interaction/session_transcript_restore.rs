/// Restore agent messages from transcript events
pub fn restore_agent_messages_from_transcript(
    project_root: &str,
    session_id: &str,
) -> Vec<RestoredMessage> {
    let events = super::session_transcript::read_transcript_events(project_root, session_id);
    let mut messages = Vec::new();

    for event in &events {
        if event.event_type == "message" {
            if let Some(role) = &event.role {
                if let Some(msg) = &event.message {
                    let content = msg.get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    messages.push(RestoredMessage {
                        role: role.clone(),
                        content,
                        timestamp: event.timestamp,
                    });
                }
            }
        }
    }

    messages
}

#[derive(Debug, Clone)]
pub struct RestoredMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

/// Adapt restored messages for model consumption
pub fn adapt_restored_messages_for_model(messages: Vec<RestoredMessage>) -> Vec<SimpleMessage> {
    messages.into_iter().map(|m| SimpleMessage {
        role: m.role,
        content: m.content,
    }).collect()
}

#[derive(Debug, Clone)]
pub struct SimpleMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_empty_transcript() {
        let messages = restore_agent_messages_from_transcript("/nonexistent", "test");
        assert!(messages.is_empty());
    }
}
