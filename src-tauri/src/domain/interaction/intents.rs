use serde::{Deserialize, Serialize};

/// Interaction intent type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionIntentType {
    DevelopBook,
    ShowBookDraft,
    CreateBook,
    DiscardBookDraft,
    ListBooks,
    SelectBook,
    ContinueBook,
    WriteNext,
    PauseBook,
    ResumeBook,
    ReviseChapter,
    RewriteChapter,
    PatchChapterText,
    ReplaceChapterText,
    EditTruth,
    RenameEntity,
    UpdateFocus,
    UpdateAuthorIntent,
    Chat,
    ExplainStatus,
    ExplainFailure,
    ExportBook,
}

/// Interaction request from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRequest {
    pub intent: InteractionIntentType,
    pub book_id: Option<String>,
    pub chapter_number: Option<u32>,
    pub title: Option<String>,
    pub genre: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub chapter_word_count: Option<u32>,
    pub target_chapters: Option<u32>,
    pub instruction: Option<String>,
    pub format: Option<String>,
    pub approved_only: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_serialization() {
        let intent = InteractionIntentType::WriteNext;
        let json = serde_json::to_string(&intent).unwrap();
        assert_eq!(json, "\"write_next\"");
    }

    #[test]
    fn test_request_creation() {
        let request = InteractionRequest {
            intent: InteractionIntentType::CreateBook,
            book_id: None,
            chapter_number: None,
            title: Some("Test Book".into()),
            genre: Some("fantasy".into()),
            platform: None,
            language: None,
            chapter_word_count: None,
            target_chapters: None,
            instruction: None,
            format: None,
            approved_only: None,
        };
        assert_eq!(request.intent, InteractionIntentType::CreateBook);
        assert_eq!(request.title.as_deref(), Some("Test Book"));
    }
}
