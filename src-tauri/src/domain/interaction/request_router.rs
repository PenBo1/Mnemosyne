use crate::errors::AppError;
use crate::domain::pipeline::PipelineRunner;

/// Route interaction requests to the appropriate handler
pub fn route_interaction_request(
    request: &crate::domain::interaction::intents::InteractionRequest,
    _pipeline: &PipelineRunner,
) -> Result<RouteResult, AppError> {
    match &request.intent {
        crate::domain::interaction::intents::InteractionIntentType::CreateBook => {
            Ok(RouteResult::NeedsConfirmation {
                action: "create_book".to_string(),
                instruction: format!(
                    "Create a {} book titled '{}'",
                    request.genre.as_deref().unwrap_or("general"),
                    request.title.as_deref().unwrap_or("Untitled")
                ),
            })
        }
        crate::domain::interaction::intents::InteractionIntentType::WriteNext => {
            let book_id = request.book_id.as_deref()
                .ok_or_else(|| AppError::bad_request("book_id required for write_next"))?;
            Ok(RouteResult::ExecuteNow {
                action: "write_next".to_string(),
                book_id: book_id.to_string(),
            })
        }
        crate::domain::interaction::intents::InteractionIntentType::Chat => {
            Ok(RouteResult::Passthrough)
        }
        _ => Ok(RouteResult::Passthrough),
    }
}

pub enum RouteResult {
    NeedsConfirmation { action: String, instruction: String },
    ExecuteNow { action: String, book_id: String },
    Passthrough,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::interaction::intents::{InteractionIntentType, InteractionRequest};

    #[test]
    fn test_route_chat() {
        let req = InteractionRequest {
            intent: InteractionIntentType::Chat,
            book_id: None, chapter_number: None, title: None, genre: None,
            platform: None, language: None, chapter_word_count: None,
            target_chapters: None, instruction: None, format: None, approved_only: None,
        };
        let pipeline = crate::domain::pipeline::PipelineRunner::new(crate::domain::pipeline::PipelineConfig {
            provider: std::sync::Arc::new(crate::infra::llm::OllamaProvider::new(None)),
            model: "test".into(),
            project_root: std::path::PathBuf::from("/tmp"),
            model_overrides: std::collections::HashMap::new(),
            memory_store: None,
            data_dir: crate::infra::data_dir::DataDir::new(std::path::PathBuf::from("/tmp")),
        });
        let result = route_interaction_request(&req, &pipeline).unwrap();
        assert!(matches!(result, RouteResult::Passthrough));
    }
}
