use crate::shared::errors::AppError;
use crate::core::agent::pipeline::PipelineRunner;

/// Project-level control operations
pub struct ProjectControl;

impl ProjectControl {
    /// Process a project-level interaction request
    pub async fn process_request(
        pipeline: &PipelineRunner,
        request: &crate::core::interaction::intents::InteractionRequest,
    ) -> Result<ControlResult, AppError> {
        match &request.intent {
            crate::core::interaction::intents::InteractionIntentType::CreateBook => {
                let title = request.title.as_deref().unwrap_or("Untitled");
                let genre = request.genre.as_deref().unwrap_or("general");
                let brief = request.instruction.as_deref();
                let book = pipeline.create_book(title, genre, brief).await?;
                Ok(ControlResult::BookCreated { book_id: book.id })
            }
            crate::core::interaction::intents::InteractionIntentType::WriteNext => {
                let book_id = request.book_id.as_deref()
                    .ok_or_else(|| AppError::bad_request("book_id required"))?;
                let result = pipeline.write_next_chapter(book_id, None).await?;
                Ok(ControlResult::ChapterWritten {
                    book_id: book_id.to_string(),
                    chapter: result.chapter_number,
                    word_count: result.word_count,
                })
            }
            crate::core::interaction::intents::InteractionIntentType::ListBooks => {
                Ok(ControlResult::BookList { books: vec![] })
            }
            _ => Ok(ControlResult::Passthrough),
        }
    }
}

pub enum ControlResult {
    BookCreated { book_id: String },
    ChapterWritten { book_id: String, chapter: u32, word_count: u32 },
    BookList { books: Vec<String> },
    Passthrough,
}
