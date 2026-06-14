use crate::errors::AppError;
use crate::domain::harness::ContextBuilder;
use crate::domain::story::{ChapterContent, AuditResult};

use super::PipelineRunner;

impl PipelineRunner {
    pub async fn revise_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
        audit: &AuditResult,
    ) -> Result<String, AppError> {
        tracing::info!(book_id, chapter = chapter_number, "Revising chapter");
        let start = std::time::Instant::now();

        let sm = self.story_manager();

        let agent_config = self.get_agent_config("reviser");
        let ctx = self.build_context(book_id, "reviser");

        let chapter = sm.load_chapter(book_id, chapter_number)?
            .ok_or_else(|| AppError::not_found(format!("Chapter {} not found", chapter_number)))?;

        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let audit_json = serde_json::to_string_pretty(audit).unwrap_or_default();
        let issues_summary: Vec<String> = audit.issues.iter().map(|i| {
            format!("[{:?}] {}: {}", i.severity, i.category, i.description)
        }).collect();

        let user = format!(
            "请修订以下章节：\n\n章节内容：\n{}\n\n审计结果：\n{}\n\n需要修复的问题：\n{}\n\n请修复所有 critical 和 warning 级别的问题，保持原文风格",
            chapter.content,
            audit_json,
            issues_summary.join("\n")
        );

        let response = self.call_llm(&system, &user).await?;

        let revised = ChapterContent {
            number: chapter_number,
            title: chapter.title,
            content: response.clone(),
        };
        sm.save_chapter(book_id, &revised)?;

        let elapsed = start.elapsed().as_millis();
        tracing::info!(book_id, chapter = chapter_number, elapsed_ms = elapsed, "Chapter revised");

        Ok(response)
    }
}
