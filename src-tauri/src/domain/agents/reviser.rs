use async_trait::async_trait;
use crate::errors::AppError;
use crate::domain::story::AuditResult;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::reviser_prompts;

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
    ) -> Result<ReviseOutput, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let system = reviser_prompts::build_system_prompt(&mode, &language);
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
            word_count: count_words(&revised_content, &language),
            fixed_issues: audit.issues.iter()
                .filter(|i| i.severity == crate::domain::story::AuditSeverity::Critical)
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
    let config_path = book_dir.join("book.json");
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            return config.get("language").and_then(|v| v.as_str()).map(|s| s.to_string());
        }
    }
    Some("zh".to_string())
}

fn count_words(text: &str, language: &str) -> u32 {
    if language == "en" {
        text.split_whitespace().count() as u32
    } else {
        let mut count = 0u32;
        for ch in text.chars() {
            if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() {
            } else if !ch.is_whitespace() {
                count += 1;
            }
        }
        let ascii_words: u32 = text.split_whitespace()
            .filter(|w| w.bytes().all(|b| b.is_ascii()))
            .count() as u32;
        count + ascii_words
    }
}
