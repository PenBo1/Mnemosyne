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

    // S5.6: direction 置顶 —— 把 author_intent.md 和 current_focus.md 从普通 context 中拆出来
    // 作为「用户方向」段置顶，优先于模型默认，必须遵循。
    let direction_sources: std::collections::HashSet<&str> = [
        "story/author_intent.md",
        "story/current_focus.md",
    ].iter().copied().collect();

    let direction_entries: Vec<&crate::core::agent::governance::ContextSource> = composed
        .context_package
        .selected_context
        .iter()
        .filter(|e| direction_sources.contains(e.source.as_str()))
        .collect();
    let other_entries: Vec<&crate::core::agent::governance::ContextSource> = composed
        .context_package
        .selected_context
        .iter()
        .filter(|e| !direction_sources.contains(e.source.as_str()))
        .collect();

    let user_direction_block = if !direction_entries.is_empty() {
        let rendered = render_context_sources(&direction_entries, language);
        if language == "en" {
            format!("## User direction (overrides model defaults — must follow)\n{}\n", rendered)
        } else {
            format!("## 用户方向（优先于模型默认，必须遵循）\n{}\n", rendered)
        }
    } else {
        String::new()
    };

    let context_section = render_context_sources(&other_entries, language);

    // S5.6: rule stack 完整渲染（hard / soft / diagnostic）
    let rule_section = if language == "en" {
        let hard = if composed.rule_stack.sections.hard.is_empty() {
            "(none)".to_string()
        } else {
            composed.rule_stack.sections.hard.join(", ")
        };
        let soft = if composed.rule_stack.sections.soft.is_empty() {
            "(none)".to_string()
        } else {
            composed.rule_stack.sections.soft.join(", ")
        };
        let diagnostic = if composed.rule_stack.sections.diagnostic.is_empty() {
            "none".to_string()
        } else {
            composed.rule_stack.sections.diagnostic.join(", ")
        };
        format!(
            "## Rule Stack\n- Hard: {}\n- Soft: {}\n- Diagnostic: {}",
            hard, soft, diagnostic
        )
    } else {
        let hard = if composed.rule_stack.sections.hard.is_empty() {
            "(无)".to_string()
        } else {
            composed.rule_stack.sections.hard.join("、")
        };
        let soft = if composed.rule_stack.sections.soft.is_empty() {
            "(无)".to_string()
        } else {
            composed.rule_stack.sections.soft.join("、")
        };
        let diagnostic = if composed.rule_stack.sections.diagnostic.is_empty() {
            "无".to_string()
        } else {
            composed.rule_stack.sections.diagnostic.join("、")
        };
        format!(
            "## 规则栈\n- 硬护栏：{}\n- 软约束：{}\n- 诊断规则：{}",
            hard, soft, diagnostic
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

    // S5.6: 顺序 task → user_direction → memo → context → rule_stack → prev → output instructions
    // direction 段非空时置顶（优先于 memo），空则跳过避免多余空段
    let mut parts: Vec<String> = vec![task_line];
    if !user_direction_block.is_empty() {
        parts.push(user_direction_block);
    }
    parts.push(memo_section);
    parts.push(context_section);
    parts.push(rule_section);
    parts.push(prev_section);
    parts.push(
        "- Output PRE_WRITE_CHECK first, then the chapter\n- Output only PRE_WRITE_CHECK, CHAPTER_TITLE, and CHAPTER_CONTENT blocks".to_string()
    );
    Ok(parts.join("\n\n"))
}

fn render_context_sources(
    sources: &[&crate::core::agent::governance::ContextSource],
    language: &str,
) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::composer::ComposeOutput;
    use crate::core::agent::governance::{
        ChapterIntent, ChapterMemo, ChapterTrace, ContextPackage, ContextSource, RuleStack,
        RuleStackSections,
    };
    use crate::core::agent::planner::PlanOutput;

    fn make_plan() -> PlanOutput {
        PlanOutput {
            intent: ChapterIntent {
                chapter: 1,
                goal: "介绍主角".to_string(),
                outline_node: None,
                arc_context: None,
                must_keep: vec![],
                must_avoid: vec![],
                style_emphasis: vec![],
            },
            memo: ChapterMemo {
                chapter: 1,
                goal: "介绍主角".to_string(),
                is_golden_opening: true,
                body: "## 本章目标\n建立主角形象".to_string(),
                thread_refs: vec![],
            },
        }
    }

    fn make_compose_output(
        sources: Vec<ContextSource>,
        sections: RuleStackSections,
    ) -> ComposeOutput {
        ComposeOutput {
            context_package: ContextPackage {
                chapter: 1,
                selected_context: sources,
            },
            rule_stack: RuleStack {
                layers: vec![],
                sections,
                override_edges: vec![],
                active_overrides: vec![],
            },
            trace: ChapterTrace::default(),
        }
    }

    /// direction 段存在时，应出现在 memo 段之前（置顶优先）
    #[test]
    fn test_direction_block_placed_before_memo_when_present_zh() {
        let sources = vec![
            ContextSource {
                source: "story/author_intent.md".to_string(),
                reason: "用户方向".to_string(),
                excerpt: Some("写一本玄幻小说".to_string()),
            },
            ContextSource {
                source: "story/current_state.md".to_string(),
                reason: "当前状态".to_string(),
                excerpt: Some("第一章刚开始".to_string()),
            },
        ];
        let composed = make_compose_output(sources, RuleStackSections::default());
        let plan = make_plan();
        let result = build_creative_user_prompt(
            std::path::Path::new("/nonexistent"),
            1,
            &plan,
            &composed,
            "zh",
        )
        .unwrap();

        let dir_pos = result.find("用户方向").expect("direction block should be present");
        let memo_pos = result.find("章节 Memo").expect("memo section should be present");
        assert!(
            dir_pos < memo_pos,
            "direction block should come before memo"
        );
    }

    /// 没有 direction entries 时，不应出现 direction 段
    #[test]
    fn test_direction_block_omitted_when_no_direction_entries() {
        let sources = vec![ContextSource {
            source: "story/current_state.md".to_string(),
            reason: "当前状态".to_string(),
            excerpt: Some("第一章刚开始".to_string()),
        }];
        let composed = make_compose_output(sources, RuleStackSections::default());
        let plan = make_plan();
        let result = build_creative_user_prompt(
            std::path::Path::new("/nonexistent"),
            1,
            &plan,
            &composed,
            "zh",
        )
        .unwrap();

        assert!(
            !result.contains("用户方向"),
            "direction block should not appear when no direction entries"
        );
    }

    /// rule stack 三层（hard / soft / diagnostic）都应渲染到输出
    #[test]
    fn test_rule_stack_renders_all_sections_zh() {
        let sections = RuleStackSections {
            hard: vec!["禁止穿越".to_string()],
            soft: vec!["保持悬疑".to_string()],
            diagnostic: vec!["检查节奏".to_string()],
        };
        let composed = make_compose_output(vec![], sections);
        let plan = make_plan();
        let result = build_creative_user_prompt(
            std::path::Path::new("/nonexistent"),
            1,
            &plan,
            &composed,
            "zh",
        )
        .unwrap();

        assert!(result.contains("硬护栏：禁止穿越"));
        assert!(result.contains("软约束：保持悬疑"));
        assert!(result.contains("诊断规则：检查节奏"));
    }

    /// rule stack 空段应显示占位符（中文）
    #[test]
    fn test_rule_stack_empty_sections_show_placeholder_zh() {
        let composed = make_compose_output(vec![], RuleStackSections::default());
        let plan = make_plan();
        let result = build_creative_user_prompt(
            std::path::Path::new("/nonexistent"),
            1,
            &plan,
            &composed,
            "zh",
        )
        .unwrap();

        assert!(result.contains("硬护栏：(无)"));
        assert!(result.contains("软约束：(无)"));
        assert!(result.contains("诊断规则：无"));
    }

    /// rule stack 三层渲染（英文）
    #[test]
    fn test_rule_stack_renders_all_sections_en() {
        let sections = RuleStackSections {
            hard: vec!["no time travel".to_string()],
            soft: vec!["keep suspense".to_string()],
            diagnostic: vec!["check pacing".to_string()],
        };
        let composed = make_compose_output(vec![], sections);
        let plan = make_plan();
        let result = build_creative_user_prompt(
            std::path::Path::new("/nonexistent"),
            1,
            &plan,
            &composed,
            "en",
        )
        .unwrap();

        assert!(result.contains("Hard: no time travel"));
        assert!(result.contains("Soft: keep suspense"));
        assert!(result.contains("Diagnostic: check pacing"));
    }

    /// direction 段英文版应使用 "User direction" 标题
    #[test]
    fn test_direction_block_english_label() {
        let sources = vec![ContextSource {
            source: "story/author_intent.md".to_string(),
            reason: "user direction".to_string(),
            excerpt: Some("Write a fantasy novel".to_string()),
        }];
        let composed = make_compose_output(sources, RuleStackSections::default());
        let plan = make_plan();
        let result = build_creative_user_prompt(
            std::path::Path::new("/nonexistent"),
            1,
            &plan,
            &composed,
            "en",
        )
        .unwrap();

        assert!(result.contains("User direction (overrides model defaults"));
    }
}
