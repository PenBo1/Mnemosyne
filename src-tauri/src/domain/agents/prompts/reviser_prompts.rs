use crate::domain::agents::reviser::ReviseMode;

pub fn build_system_prompt(mode: &ReviseMode, language: &str, identity_prefix: Option<&str>) -> String {
    let mode_desc = match mode {
        ReviseMode::Auto => match language {
            "en" => "Auto mode: Fix all critical issues first, then warning issues. Preserve the original style and tone. Minimize changes.",
            _ => "自动模式：先修复所有 critical 问题，再修复 warning 问题。保持原文风格和语气。最小化改动。",
        },
        ReviseMode::Polish => match language {
            "en" => "Polish mode: Only improve expression, rhythm, and paragraph breathing. Do NOT change facts or plot conclusions.",
            _ => "润色模式：只改表达、节奏、段落呼吸，不改事实与剧情结论。",
        },
        ReviseMode::Rewrite => match language {
            "en" => "Rewrite mode: Allow restructuring problem paragraphs, adjusting imagery and narrative intensity. Preserve core facts and character motivations.",
            _ => "改写模式：允许重组问题段落、调整画面和叙述力度。保留核心事实与人物动机。",
        },
        ReviseMode::Rework => match language {
            "en" => "Rework mode: Can reconstruct scene progression and conflict organization, but do NOT change main settings or major event outcomes.",
            _ => "重写模式：可重构场景推进和冲突组织，但不改主设定和大事件结果。",
        },
        ReviseMode::SpotFix => match language {
            "en" => "Spot-fix mode: Make targeted fixes only. Do NOT rewrite any paragraphs — patch specific words, phrases, or sentences.",
            _ => "定点修复模式：只做针对性修复。不要重写任何段落——只修补特定的词、短语或句子。",
        },
    };

    let task_prompt = match language {
        "en" => {
            format!(
                r#"You are a revision specialist. Revise the chapter based on the audit feedback.

## Revision mode
{}

## Revision principles
1. Fix all Critical issues first
2. Then fix Warning issues
3. Preserve the original style and tone
4. Minimize changes — only modify what's necessary
5. Do NOT introduce new problems

## Output
Return the full revised chapter text wrapped in === REVISED_CONTENT === markers."#,
                mode_desc
            )
        }
        _ => {
            format!(
                r#"你是一位修订专家。根据审计反馈修订章节内容。

## 修订模式
{}

## 修订原则
1. 先修复所有 Critical 问题
2. 再修复 Warning 问题
3. 保持原文风格和语气
4. 最小化改动——只修改必要的部分
5. 不要引入新问题

## 输出
返回完整的修订后章节文本，用 === REVISED_CONTENT === 标记包裹。"#,
                mode_desc
            )
        }
    };

    match identity_prefix {
        Some(prefix) if !prefix.is_empty() => format!("{}\n\n{}", prefix, task_prompt),
        _ => task_prompt.to_string(),
    }
}

pub fn build_user_message(
    chapter_number: u32,
    chapter_content: &str,
    audit: &crate::domain::story::AuditResult,
    language: &str,
) -> String {
    let issues_summary: Vec<String> = audit.issues.iter().map(|i| {
        let severity_str = match i.severity {
            crate::domain::story::AuditSeverity::Critical => "CRITICAL",
            crate::domain::story::AuditSeverity::Warning => "WARNING",
            crate::domain::story::AuditSeverity::Info => "INFO",
        };
        format!("[{}] {}: {}", severity_str, i.category, i.description)
    }).collect();

    let heading = if language == "en" {
        format!("Chapter {}", chapter_number)
    } else {
        format!("第{}章", chapter_number)
    };

    format!(
        "## {} 正文\n\n{}\n\n## 审计结果\n\n通过：{}\n分数：{}/100\n\n需要修复的问题：\n{}",
        heading,
        chapter_content,
        audit.passed,
        audit.score,
        issues_summary.join("\n")
    )
}
