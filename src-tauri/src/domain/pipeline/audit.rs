use crate::errors::AppError;
use crate::domain::harness::ContextBuilder;
use crate::domain::story::AuditResult;

use super::PipelineRunner;

impl PipelineRunner {
    pub async fn audit_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
    ) -> Result<AuditResult, AppError> {
        tracing::info!(book_id, chapter = chapter_number, "Auditing chapter");
        let start = std::time::Instant::now();

        let sm = self.story_manager();
        let state = sm.load_state(book_id)?;

        let agent_config = self.get_agent_config("auditor");
        let ctx = self.build_context(book_id, "auditor");

        let chapter = sm.load_chapter(book_id, chapter_number)?
            .ok_or_else(|| AppError::not_found(format!("Chapter {} not found", chapter_number)))?;

        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let user = format!(
            "请审计以下章节：\n\n章节标题：{}\n章节内容：\n{}\n\n当前状态：第{}章，共{}字",
            chapter.title,
            chapter.content,
            state.current_chapter,
            state.total_words
        );

        let response = self.call_llm(&system, &user).await?;
        let result: AuditResult = serde_json::from_str(&response)
            .unwrap_or(AuditResult {
                passed: false,
                score: 0.0,
                issues: Vec::new(),
                summary: response,
            });

        let elapsed = start.elapsed().as_millis();
        let critical_count = result.issues.iter()
            .filter(|i| i.severity == crate::domain::story::AuditSeverity::Critical)
            .count();
        let warning_count = result.issues.iter()
            .filter(|i| i.severity == crate::domain::story::AuditSeverity::Warning)
            .count();

        tracing::info!(
            book_id,
            chapter = chapter_number,
            passed = result.passed,
            score = result.score,
            critical = critical_count,
            warnings = warning_count,
            elapsed_ms = elapsed,
            "Chapter audit completed"
        );

        Ok(result)
    }
}
