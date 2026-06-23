use async_trait::async_trait;
use crate::errors::AppError;
use crate::infra::gc::utils;
use crate::infra::data_dir::DataDir;
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

    fn tool_defs(&self, ctx: &AgentContext) -> Vec<crate::infra::llm::types::ToolSpec> {
        ctx.tools.definitions().iter().map(|d| {
            crate::infra::llm::types::ToolSpec {
                name: d.name.clone(),
                description: d.description.clone(),
                parameters: d.parameters.clone(),
            }
        }).collect()
    }

    /// Write a chapter using two-phase approach: creative writing + state settlement
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

        // ── Phase 2: State settlement (Observer + Reflector) ──
        // Each sub-agent loads its OWN identity — not the writer's
        tracing::info!(chapter = chapter_number, "Phase 2a: observing facts");
        let observer_identity = AgentIdentity::load(data_dir, "observer");
        let observer_prefix = observer_identity.build_system_prefix();
        let observer_system = super::prompts::observer_prompts::build_system_prompt(&language, Some(&observer_prefix));
        let observer_user = super::prompts::observer_prompts::build_user_prompt(
            chapter_number,
            &creative.title,
            &creative.content,
            &language,
        );
        let observations = self.chat(ctx, &observer_system, &observer_user).await?;

        tracing::info!(chapter = chapter_number, "Phase 2b: reflecting into truth files");
        let reflector_identity = AgentIdentity::load(data_dir, "reflector");
        let reflector_prefix = reflector_identity.build_system_prefix();
        let settler_system = super::prompts::settler_prompts::build_system_prompt(&language, Some(&reflector_prefix));
        let settler_user = super::prompts::settler_prompts::build_user_message(
            chapter_number,
            &creative.title,
            &creative.content,
            book_dir,
            &observations.content,
            &language,
        )?;
        let settlement = self.chat(ctx, &settler_system, &settler_user).await?;

        // Parse settlement delta
        let delta = parse_settlement_delta(&settlement.content, chapter_number)?;

        // Save settlement outputs
        save_settlement_outputs(book_dir, chapter_number, &creative, &delta, &language)?;

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

pub struct SettlementDelta {
    pub updated_state: String,
    pub updated_hooks: String,
    pub chapter_summary: String,
    pub updated_subplots: String,
    pub updated_emotional_arcs: String,
    pub updated_character_matrix: String,
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

fn parse_settlement_delta(content: &str, _chapter_number: u32) -> Result<SettlementDelta, AppError> {
    // Try JSON parsing first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        return Ok(SettlementDelta {
            updated_state: json.get("updated_state")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            updated_hooks: json.get("updated_hooks")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            chapter_summary: json.get("chapter_summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            updated_subplots: json.get("updated_subplots")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            updated_emotional_arcs: json.get("updated_emotional_arcs")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            updated_character_matrix: json.get("updated_character_matrix")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    // Fallback: treat entire content as updated state
    Ok(SettlementDelta {
        updated_state: content.to_string(),
        updated_hooks: String::new(),
        chapter_summary: String::new(),
        updated_subplots: String::new(),
        updated_emotional_arcs: String::new(),
        updated_character_matrix: String::new(),
    })
}

fn save_settlement_outputs(
    book_dir: &std::path::Path,
    chapter_number: u32,
    creative: &CreativeOutput,
    delta: &SettlementDelta,
    language: &str,
) -> Result<(), AppError> {
    let story_dir = book_dir.join("story");
    let chapters_dir = book_dir.join("chapters");

    // Save chapter file
    std::fs::create_dir_all(&chapters_dir)?;
    let filename = format!("{:04}_{}.md", chapter_number, utils::sanitize_filename(&creative.title));
    let heading = if language == "en" {
        format!("# Chapter {}: {}", chapter_number, creative.title)
    } else {
        format!("# 第{}章 {}", chapter_number, creative.title)
    };
    std::fs::write(
        chapters_dir.join(filename),
        format!("{}\n\n{}", heading, creative.content),
    )?;

    // Save truth files
    if !delta.updated_state.is_empty() {
        std::fs::write(story_dir.join("current_state.md"), &delta.updated_state)?;
    }
    if !delta.updated_hooks.is_empty() {
        std::fs::write(story_dir.join("pending_hooks.md"), &delta.updated_hooks)?;
    }
    if !delta.chapter_summary.is_empty() {
        append_chapter_summary(&story_dir, &delta.chapter_summary, language)?;
    }
    if !delta.updated_subplots.is_empty() {
        std::fs::write(story_dir.join("subplot_board.md"), &delta.updated_subplots)?;
    }
    if !delta.updated_emotional_arcs.is_empty() {
        std::fs::write(story_dir.join("emotional_arcs.md"), &delta.updated_emotional_arcs)?;
    }
    if !delta.updated_character_matrix.is_empty() {
        std::fs::write(story_dir.join("character_matrix.md"), &delta.updated_character_matrix)?;
    }

    Ok(())
}

fn append_chapter_summary(
    story_dir: &std::path::Path,
    summary: &str,
    language: &str,
) -> Result<(), AppError> {
    let path = story_dir.join("chapter_summaries.md");
    let header = if language == "en" {
        "# Chapter Summaries\n\n| Chapter | Title | Characters | Key Events | State Changes | Hook Activity | Mood | Chapter Type |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n"
    } else {
        "# 章节摘要\n\n| 章节 | 标题 | 出场人物 | 关键事件 | 状态变化 | 伏笔动态 | 情绪基调 | 章节类型 |\n|------|------|----------|----------|----------|----------|----------|----------|\n"
    };

    let mut content = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        header.to_string()
    };

    // Extract only data rows from summary
    for line in summary.lines() {
        if line.starts_with('|') && !line.starts_with("| 章节") && !line.starts_with("| Chapter") && !line.starts_with("|--") {
            content.push_str(line);
            content.push('\n');
        }
    }

    std::fs::write(path, content)?;
    Ok(())
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infra::gc::utils::read_book_language_from_dir(book_dir)
}
