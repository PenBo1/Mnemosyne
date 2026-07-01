use crate::shared::errors::AppError;
use crate::core::agent::pipeline::PipelineRunner;

/// Route interaction requests to the appropriate handler
pub fn route_interaction_request(
    request: &crate::core::interaction::intents::InteractionRequest,
    _pipeline: &PipelineRunner,
) -> Result<RouteResult, AppError> {
    match &request.intent {
        crate::core::interaction::intents::InteractionIntentType::CreateBook => {
            Ok(RouteResult::NeedsConfirmation {
                action: "create_book".to_string(),
                instruction: format!(
                    "Create a {} book titled '{}'",
                    request.genre.as_deref().unwrap_or("general"),
                    request.title.as_deref().unwrap_or("Untitled")
                ),
            })
        }
        crate::core::interaction::intents::InteractionIntentType::WriteNext => {
            let book_id = request.book_id.as_deref()
                .ok_or_else(|| AppError::bad_request("book_id required for write_next"))?;
            Ok(RouteResult::ExecuteNow {
                action: "write_next".to_string(),
                book_id: book_id.to_string(),
            })
        }
        crate::core::interaction::intents::InteractionIntentType::Chat => {
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
    use crate::core::interaction::intents::{InteractionIntentType, InteractionRequest};

    #[test]
    fn test_route_chat() {
        let req = InteractionRequest {
            intent: InteractionIntentType::Chat,
            book_id: None, chapter_number: None, title: None, genre: None,
            platform: None, language: None, chapter_word_count: None,
            target_chapters: None, instruction: None, format: None, approved_only: None,
        };
        let pipeline = crate::core::agent::pipeline::PipelineRunner::new(crate::core::agent::pipeline::PipelineConfig {
            provider: std::sync::Arc::new(crate::infrastructure::llm_client::OllamaProvider::new(None)),
            model: "test".into(),
            project_root: std::path::PathBuf::from("/tmp"),
            model_overrides: std::collections::HashMap::new(),
            agent_providers: std::collections::HashMap::new(),
            memory_store: None,
            data_dir: crate::infrastructure::file_storage::data_dir::DataDir::new(std::path::PathBuf::from("/tmp")),
            user_profile: None,
            fallback_model: None,
            db: None,
            context_budget: None,
        });
        let result = route_interaction_request(&req, &pipeline).unwrap();
        assert!(matches!(result, RouteResult::Passthrough));
    }
}
