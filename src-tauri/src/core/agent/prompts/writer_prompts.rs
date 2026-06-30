use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};

pub fn build_creative_system_prompt(
    language: &str,
    target_words: u32,
    identity_prefix: Option<&str>,
) -> String {
    let task_prompt = match language {
        "en" => {
            format!(
                r#"You are a skilled web-fiction novelist. Write chapter prose following the chapter memo and context package.

## Output format

=== PRE_WRITE_CHECK ===
<brief pre-flight check of what you will write>

=== CHAPTER_TITLE ===
<title>

=== CHAPTER_CONTENT ===
<full chapter prose>

## Requirements
- Target length: {} words
- Follow the chapter memo strictly
- Maintain character voice consistency
- Use natural language — avoid AI-flavored phrases
- End with a hook or cliffhanger
- Do NOT use: "It is worth noting", "However", "Interestingly", "Notably""#,
                target_words
            )
        }
        _ => {
            format!(
                r#"你是一位网络小说写手。按照章节 memo 和上下文包撰写正文。

## 输出格式

=== PRE_WRITE_CHECK ===
<写作自检表>

=== CHAPTER_TITLE ===
<标题>

=== CHAPTER_CONTENT ===
<完整正文>

## 要求
- 目标字数：{}字
- 严格按照章节 memo 写作
- 保持角色说话风格一致
- 使用自然中文——避免 AI 痕迹重的词
- 章尾留钩子
- 禁止使用：值得一提的是、不禁、缓缓、仿佛、宛如、竟然、猛地、忽然"#,
                target_words
            )
        }
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
}

pub fn build_creative_user_prompt(
    book_dir: &std::path::Path,
    chapter_number: u32,
    plan: &crate::core::agent::planner::PlanOutput,
    composed: &crate::core::agent::composer::ComposeOutput,
    language: &str,
) -> Result<String, crate::shared::errors::AppError> {
    let _story_dir = book_dir.join("story");
    let _read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    // Read previous chapter
    let previous_chapter = if chapter_number > 1 {
        let chapters_dir = book_dir.join("chapters");
        let prefix = format!("{:04}_", chapter_number - 1);
        let mut result = None;
        if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(&prefix) {
                    let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                    let chars: Vec<char> = content.chars().collect();
                    let len = chars.len();
                    if len > 500 {
                        let excerpt: String = chars[len-500..].iter().collect();
                        result = Some(format!("...{}", excerpt));
                    } else {
                        result = Some(content);
                    }
                    break;
                }
            }
        }
        result
    } else {
        None
    };

    let memo_section = if language == "en" {
        format!("## Chapter Memo\n\n{}", plan.memo.body)
    } else {
        format!("## 章节 Memo\n\n{}", plan.memo.body)
    };

    let context_section = render_context_sources(&composed.context_package.selected_context, language);

    let rule_section = if language == "en" {
        format!(
            "## Rule Stack\n- Hard: {}\n- Soft: {}",
            plan.intent.must_keep.join(", "),
            plan.intent.must_avoid.join(", ")
        )
    } else {
        format!(
            "## 规则栈\n- 硬护栏：{}\n- 软约束：{}",
            plan.intent.must_keep.join("、"),
            plan.intent.must_avoid.join("、")
        )
    };

    let prev_section = if let Some(prev) = previous_chapter {
        if language == "en" {
            format!("## Previous Chapter Ending\n\n{}", prev)
        } else {
            format!("## 前一章结尾\n\n{}", prev)
        }
    } else if language == "en" {
        "## Previous Chapter Ending\n\n(This is the first chapter)".to_string()
    } else {
        "## 前一章结尾\n\n（这是第一章）".to_string()
    };

    let task_line = if language == "en" {
        format!("Write chapter {}.", chapter_number)
    } else {
        format!("请续写第{}章。", chapter_number)
    };

    Ok(format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n- Output PRE_WRITE_CHECK first, then the chapter\n- Output only PRE_WRITE_CHECK, CHAPTER_TITLE, and CHAPTER_CONTENT blocks",
        task_line, memo_section, context_section, rule_section, prev_section
    ))
}

fn render_context_sources(sources: &[crate::core::agent::governance::ContextSource], language: &str) -> String {
    if sources.is_empty() {
        return if language == "en" {
            "## Selected Context\n\n(none)".to_string()
        } else {
            "## 已选上下文\n\n（无）".to_string()
        };
    }

    let header = if language == "en" { "## Selected Context" } else { "## 已选上下文" };
    let sections: Vec<String> = sources.iter().map(|s| {
        let excerpt = s.excerpt.as_deref().unwrap_or(&s.reason);
        format!("### {} ({})\n\n{}", s.source, s.reason, excerpt)
    }).collect();

    format!("{}\n\n{}", header, sections.join("\n\n"))
}
