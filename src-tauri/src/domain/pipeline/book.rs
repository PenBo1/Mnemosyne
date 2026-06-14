use crate::errors::AppError;
use crate::domain::harness::ContextBuilder;
use crate::domain::story::BookConfig;

use super::PipelineRunner;

impl PipelineRunner {
    pub async fn create_book(
        &self,
        title: &str,
        genre: &str,
        brief: Option<&str>,
    ) -> Result<BookConfig, AppError> {
        tracing::info!(title, genre, "Creating new book");
        let start = std::time::Instant::now();

        let sm = self.story_manager();
        let book_id = uuid::Uuid::new_v4().to_string();

        let agent_config = self.get_agent_config("architect");
        let ctx = self.build_context(&book_id, "architect");
        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let user = if let Some(brief_content) = brief {
            format!("请根据以下创作简报创建小说基础设定：\n\n{}", brief_content)
        } else {
            format!("请为一本{}题材的小说《{}》创建基础设定", genre, title)
        };

        tracing::debug!(book_id = %book_id, "Calling architect LLM");
        let response = self.call_llm(&system, &user).await?;

        let architect_output: serde_json::Value = serde_json::from_str(&response)
            .unwrap_or(serde_json::json!({}));

        let chapter_words = architect_output
            .get("chapter_words")
            .and_then(|v| v.as_u64())
            .unwrap_or(3000) as u32;

        let target_chapters = architect_output
            .get("target_chapters")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as u32;

        let config = BookConfig {
            id: book_id.clone(),
            title: title.to_string(),
            genre: genre.to_string(),
            platform: "local".to_string(),
            status: Default::default(),
            language: "zh".to_string(),
            chapter_words,
            target_chapters,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        sm.create_book(&config)?;
        sm.save_control_doc(&book_id, "architect_output.json", &response)?;

        let elapsed = start.elapsed().as_secs();
        tracing::info!(book_id = %book_id, title, genre, elapsed_secs = elapsed, "Book created successfully");

        Ok(config)
    }
}
