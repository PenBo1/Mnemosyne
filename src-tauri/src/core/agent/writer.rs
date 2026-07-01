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

        // S5.7: PRE_WRITE_CHECK 软对齐检查（只 warn 不 fail）
        // memo 已在 planner 阶段严格解析过，这里只检查 LLM 自检是否覆盖了关键段落
        self.verify_pre_write_check_aligns_with_memo(
            &creative.pre_write_check,
            chapter_number,
            &language,
        );

        Ok(WriteOutput {
            chapter_number,
            title: creative.title,
            content: creative.content,
            word_count: creative.word_count,
            pre_write_check: creative.pre_write_check,
        })
    }

    /// S5.7: PRE_WRITE_CHECK 与 chapter memo 的软对齐检查。
    ///
    /// 移植自 inkos `verifyPreWriteCheckAlignsWithMemo`。memo 已在 planner
    /// 阶段严格解析过，这里只 warn —— LLM 自检可能跳过或简写了某行。
    fn verify_pre_write_check_aligns_with_memo(
        &self,
        pre_write_check: &str,
        chapter_number: u32,
        language: &str,
    ) {
        match check_pre_write_check_alignment(pre_write_check, language) {
            PreWriteCheckAlignment::Empty => {
                tracing::warn!(
                    chapter = chapter_number,
                    "PRE_WRITE_CHECK is empty; cannot verify memo alignment"
                );
            }
            PreWriteCheckAlignment::Missing(missing) => {
                tracing::warn!(
                    chapter = chapter_number,
                    missing = ?missing,
                    "PRE_WRITE_CHECK missing memo sections"
                );
            }
            PreWriteCheckAlignment::Aligned => {}
        }
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

/// S5.7: PRE_WRITE_CHECK 软对齐检查结果
#[derive(Debug, Clone, PartialEq)]
pub enum PreWriteCheckAlignment {
    /// PRE_WRITE_CHECK 为空，无法对齐
    Empty,
    /// 缺少的 memo 段落标签
    Missing(Vec<String>),
    /// 全部对齐
    Aligned,
}

/// S5.7: 检查 PRE_WRITE_CHECK 是否包含 chapter memo 的关键段落标识。
///
/// 移植自 inkos `verifyPreWriteCheckAlignsWithMemo`。这是软对齐检查 ——
/// 只 warn 不 fail，因为 LLM 自检可能跳过或简写了某行。
/// memo 已在 planner 阶段严格解析过，这里只检查 LLM 自检输出是否覆盖了关键概念。
pub fn check_pre_write_check_alignment(pre_write_check: &str, language: &str) -> PreWriteCheckAlignment {
    if pre_write_check.trim().is_empty() {
        return PreWriteCheckAlignment::Empty;
    }
    let required: &[(&str, &str)] = if language == "en" {
        &[
            ("Current task", "Current task"),
            ("Do not", "Do not"),
            ("end-of-chapter", "Required end-of-chapter change"),
        ]
    } else {
        &[
            ("当前任务", "当前任务"),
            ("不要做", "不要做"),
            ("章尾", "章尾必须发生的改变"),
        ]
    };
    let missing: Vec<String> = required.iter()
        .filter(|(needle, _)| !pre_write_check.contains(needle))
        .map(|(_, label)| label.to_string())
        .collect();
    if missing.is_empty() {
        PreWriteCheckAlignment::Aligned
    } else {
        PreWriteCheckAlignment::Missing(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_empty_pre_write_check() {
        assert_eq!(
            check_pre_write_check_alignment("", "zh"),
            PreWriteCheckAlignment::Empty
        );
        assert_eq!(
            check_pre_write_check_alignment("   \n  ", "en"),
            PreWriteCheckAlignment::Empty
        );
    }

    #[test]
    fn test_alignment_all_sections_present_zh() {
        let check = "当前任务：介绍主角\n不要做：穿越\n章尾：抛出钩子";
        assert_eq!(
            check_pre_write_check_alignment(check, "zh"),
            PreWriteCheckAlignment::Aligned
        );
    }

    #[test]
    fn test_alignment_all_sections_present_en() {
        let check = "Current task: intro protagonist\nDo not: time travel\nend-of-chapter: hook";
        assert_eq!(
            check_pre_write_check_alignment(check, "en"),
            PreWriteCheckAlignment::Aligned
        );
    }

    #[test]
    fn test_alignment_missing_sections_zh() {
        let check = "当前任务：介绍主角";
        let result = check_pre_write_check_alignment(check, "zh");
        match result {
            PreWriteCheckAlignment::Missing(missing) => {
                assert_eq!(missing.len(), 2);
                assert!(missing.contains(&"不要做".to_string()));
                assert!(missing.contains(&"章尾必须发生的改变".to_string()));
            }
            _ => panic!("expected Missing, got {:?}", result),
        }
    }

    #[test]
    fn test_alignment_missing_sections_en() {
        let check = "Current task: intro\nend-of-chapter: hook";
        let result = check_pre_write_check_alignment(check, "en");
        match result {
            PreWriteCheckAlignment::Missing(missing) => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0], "Do not");
            }
            _ => panic!("expected Missing, got {:?}", result),
        }
    }

    #[test]
    fn test_alignment_all_missing_zh() {
        let check = "这是一段没有关键标识的自检";
        let result = check_pre_write_check_alignment(check, "zh");
        match result {
            PreWriteCheckAlignment::Missing(missing) => {
                assert_eq!(missing.len(), 3);
            }
            _ => panic!("expected Missing with 3 items, got {:?}", result),
        }
    }
}
