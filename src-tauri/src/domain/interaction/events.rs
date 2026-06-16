use serde::{Deserialize, Serialize};

/// Execution status for pipeline operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Idle,
    Planning,
    Composing,
    Writing,
    Assessing,
    Repairing,
    Persisting,
    WaitingHuman,
    Blocked,
    Completed,
    Failed,
}

/// Current execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub status: ExecutionStatus,
    pub book_id: Option<String>,
    pub chapter_number: Option<u32>,
    pub stage_label: Option<String>,
}

/// Interaction event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionEvent {
    pub kind: String,
    pub timestamp: u64,
    pub status: ExecutionStatus,
    pub book_id: Option<String>,
    pub chapter_number: Option<u32>,
    pub detail: Option<String>,
}

/// Check if an execution status is terminal
pub fn is_terminal_execution_status(status: &ExecutionStatus) -> bool {
    matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Blocked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_status() {
        assert!(is_terminal_execution_status(&ExecutionStatus::Completed));
        assert!(is_terminal_execution_status(&ExecutionStatus::Failed));
        assert!(is_terminal_execution_status(&ExecutionStatus::Blocked));
        assert!(!is_terminal_execution_status(&ExecutionStatus::Writing));
        assert!(!is_terminal_execution_status(&ExecutionStatus::Planning));
    }

    #[test]
    fn test_execution_state() {
        let state = ExecutionState {
            status: ExecutionStatus::Writing,
            book_id: Some("b1".into()),
            chapter_number: Some(3),
            stage_label: Some("writing chapter 3".into()),
        };
        assert_eq!(state.status, ExecutionStatus::Writing);
        assert_eq!(state.chapter_number, Some(3));
    }
}
