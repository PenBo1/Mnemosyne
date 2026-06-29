use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::features::story::AuditResult;
use crate::infrastructure::state_store::gc::utils;
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::reviser_prompts;
use super::agent_identity::AgentIdentity;

#[derive(Debug, Clone, PartialEq)]
pub enum ReviseMode {
    Auto,
    Polish,
    Rewrite,
    Rework,
    SpotFix,
}

impl Default for ReviseMode {
    fn default() -> Self {
        Self::Auto
    }
}

pub struct ReviserAgent;

impl Default for ReviserAgent {
    fn default() -> Self { Self }
}
impl ReviserAgent {
    pub fn new() -> Self { Self }

    /// Revise a chapter based on audit issues
    pub async fn revise_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        chapter_content: &str,
        audit: &AuditResult,
        mode: ReviseMode,
        data_dir: &DataDir,
    ) -> Result<ReviseOutput, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let identity = AgentIdentity::load(data_dir, "reviser");
        let task_query = format!("revise chapter {} based on audit feedback", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = reviser_prompts::build_system_prompt(&mode, &language, Some(&identity_prefix));
        let user = reviser_prompts::build_user_message(
            chapter_number,
            chapter_content,
            audit,
            &language,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let revised_content = extract_revised_content(&response.content);

        Ok(ReviseOutput {
            chapter_number,
            content: revised_content.clone(),
            word_count: utils::count_words(&revised_content, &language),
            fixed_issues: audit.issues.iter()
                .filter(|i| i.severity == crate::features::story::AuditSeverity::Critical)
                .map(|i| i.description.clone())
                .collect(),
        })
    }
}

#[async_trait]
impl BaseAgent for ReviserAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Reviser
    }

    fn name(&self) -> &str {
        "reviser"
    }
}

pub struct ReviseOutput {
    pub chapter_number: u32,
    pub content: String,
    pub word_count: u32,
    pub fixed_issues: Vec<String>,
}

fn extract_revised_content(content: &str) -> String {
    // Try to find revised content markers
    if let Some(start) = content.find("=== REVISED_CONTENT ===") {
        let after = &content[start + "=== REVISED_CONTENT ===".len()..];
        if let Some(end) = after.find("===") {
            return after[..end].trim().to_string();
        }
    }

    // If no markers, treat entire content as revised
    content.to_string()
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
}
