use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::state_store::gc::utils;
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::planner::PlanOutput;
use super::composer::ComposeOutput;
use super::prompts::writer_prompts;
use super::agent_identity::AgentIdentity;

pub struct WriterAgent;

impl Default for WriterAgent {
    fn default() -> Self { Self }
}
impl WriterAgent {
    pub fn new() -> Self { Self }

    /// Write a chapter (Phase 1: creative writing only).
    ///
    /// Observer + Reflector 阶段由 pipeline 独立编排（见 ObserverAgent /
    /// ReflectorAgent），不再内嵌在 Writer 中，符合"职责单一"原则。
    /// 之前内嵌的 Phase 2a/2b 写入的 markdown truth files 没有下游消费者
    /// （runner 从 story/state.json 读取 StoryState），属于死代码路径，已移除。
    pub async fn write_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        plan: &PlanOutput,
        composed: &ComposeOutput,
        target_words: u32,
        data_dir: &DataDir,
    ) -> Result<WriteOutput, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());

        // Load agent identity from data directory
        let identity = AgentIdentity::load(data_dir, "writer");
        let task_query = format!("write chapter {} of a novel", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;

        // ── Phase 1: Creative writing ──
        tracing::info!(chapter = chapter_number, "Phase 1: creative writing");
        let creative_system = writer_prompts::build_creative_system_prompt(
            &language,
            target_words,
            Some(&identity_prefix),
        );
        let creative_user = writer_prompts::build_creative_user_prompt(
            book_dir,
            chapter_number,
            plan,
            composed,
            &language,
        )?;

        let creative_response = self.chat(ctx, &creative_system, &creative_user).await?;
        let creative = parse_creative_output(&creative_response.content, chapter_number, &language)?;

        Ok(WriteOutput {
            chapter_number,
            title: creative.title,
            content: creative.content,
            word_count: creative.word_count,
            pre_write_check: creative.pre_write_check,
        })
    }
}

#[async_trait]
impl BaseAgent for WriterAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Writer
    }

    fn name(&self) -> &str {
        "writer"
    }
}

pub struct CreativeOutput {
    pub title: String,
    pub content: String,
    pub word_count: u32,
    pub pre_write_check: String,
}

pub struct WriteOutput {
    pub chapter_number: u32,
    pub title: String,
    pub content: String,
    pub word_count: u32,
    pub pre_write_check: String,
}

fn parse_creative_output(content: &str, chapter_number: u32, language: &str) -> Result<CreativeOutput, AppError> {
    let extract = |tag: &str| -> String {
        let pattern = format!("=== {} ===", tag);
        if let Some(start) = content.find(&pattern) {
            let after = &content[start + pattern.len()..];
            // Find next === TAG === or end of string
            let end = after.find("===").unwrap_or(after.len());
            after[..end].trim().to_string()
        } else {
            String::new()
        }
    };

    let pre_write_check = extract("PRE_WRITE_CHECK");
    let title = extract("CHAPTER_TITLE");
    let mut content_section = extract("CHAPTER_CONTENT");

    // Fallback: if no markers found, use entire content
    if content_section.is_empty() {
        content_section = content.to_string();
    }

    // Fallback title
    let title = if title.is_empty() {
        if language == "en" {
            format!("Chapter {}", chapter_number)
        } else {
            format!("第{}章", chapter_number)
        }
    } else {
        title
    };

    let word_count = utils::count_words(&content_section, language);

    Ok(CreativeOutput {
        title,
        content: content_section,
        word_count,
        pre_write_check,
    })
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
}
