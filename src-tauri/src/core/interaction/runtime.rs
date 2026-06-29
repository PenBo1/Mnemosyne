use crate::shared::errors::AppError;

/// Project-level runtime for managing agent sessions
pub struct Runtime;

impl Default for Runtime {
    fn default() -> Self { Self }
}

impl Runtime {
    /// Create a new runtime
    pub fn new() -> Self {
        Self
    }

    /// Run an interaction request
    pub async fn run_request(
        &self,
        _project_root: &str,
        request: &crate::core::interaction::intents::InteractionRequest,
        pipeline: &crate::core::agent::pipeline::PipelineRunner,
    ) -> Result<RuntimeResult, AppError> {
        match &request.intent {
            crate::core::interaction::intents::InteractionIntentType::CreateBook => {
                let title = request.title.as_deref().unwrap_or("Untitled");
                let genre = request.genre.as_deref().unwrap_or("general");
                let brief = request.instruction.as_deref();
                let book = pipeline.create_book(title, genre, brief).await?;
                Ok(RuntimeResult::BookCreated { book_id: book.id })
            }
            crate::core::interaction::intents::InteractionIntentType::WriteNext => {
                let book_id = request.book_id.as_deref()
                    .ok_or_else(|| AppError::bad_request("book_id required"))?;
                let result = pipeline.write_next_chapter(book_id, None).await?;
                Ok(RuntimeResult::ChapterWritten {
                    book_id: book_id.to_string(),
                    chapter: result.chapter_number,
                    word_count: result.word_count,
                })
            }
            crate::core::interaction::intents::InteractionIntentType::Chat => {
                Ok(RuntimeResult::Chat)
            }
            _ => Ok(RuntimeResult::Passthrough),
        }
    }
}

pub enum RuntimeResult {
    BookCreated { book_id: String },
    ChapterWritten { book_id: String, chapter: u32, word_count: u32 },
    Chat,
    Passthrough,
}
