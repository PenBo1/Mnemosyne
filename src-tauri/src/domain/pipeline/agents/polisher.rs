// Polisher Agent。
//
// 职责：对成稿做纯文字层润色（句式/段落/用词/五感/对话自然度）。
// 禁止增删情节、改变人设、调整主线。输出润色后的完整正文。
//
// prompt 策略：6 条文笔类雷点 + 文字层修改边界 + 纯文本输出。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

/// Polisher 输出
#[derive(Debug, Clone)]
pub struct PolishOutput {
    pub polished_content: String,
    pub changed: bool,
}

/// 润色一章。
pub async fn polish_chapter(
    engine: &AgentEngine,
    chapter_content: &str,
    chapter_number: u32,
    chapter_memo: Option<&str>,
) -> Result<PolishOutput, AppError> {
    let system_prompt = build_system_prompt();
    let user_message = build_user_message(chapter_content, chapter_number, chapter_memo);

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    let polished = strip_code_fence(&response);
    let changed = polished != chapter_content;

    Ok(PolishOutput {
        polished_content: polished,
        changed,
    })
}

fn build_system_prompt() -> String {
    r###"<identity>
You are a prose-polishing editor for web fiction. You touch only the language layer — never plot, never character, never the main line.
</identity>

<edit_boundary>
Allowed edits:
- Sentence structure and rhythm (alternating long and short sentences, a sense of breathing).
- Paragraph splits (optimized for mobile reading).
- Word precision (replace clichés and fatigue words).
- Concretize the five senses (turn abstract description into visualizable detail).
- Dialogue naturalness (remove mechanical feel, differentiate how each character speaks).

Forbidden edits:
- Adding or removing plot, scenes, or events.
- Altering a character's personality, motivation, or relationships.
- Adjusting the main-line direction or hook placement.
- Changing any factual information (names, quantities, times, locations).
</edit_boundary>

## Six Prose-Level Red Flags (must fix)

1. Ineffective description: adjectives piled up without a concrete image. Convert to visualizable detail.
2. Over-ornate prose: purple phrasing that upstages the story. Cut ornament; serve the narrative.
3. Weak prose: arid expression, low information density. Add sensory and action detail.
4. Irregular formatting: paragraphs too long, too short, or uniform. Aim for 3-5 lines per paragraph with alternating lengths.
5. AI-tell traces: high cliché density, transition-word overuse, dense "了" (le) characters, narrator conclusions. Break sentence-pattern regularity.
6. Stereotyped ensemble: side characters react identically, "the crowd gasped in unison." Give each character an independent reaction.

## Output Format

Emit the polished full prose directly — no commentary, no code-fence markers, no prefixes or suffixes. Only the prose itself."###
        .to_string()
}

fn build_user_message(chapter_content: &str, chapter_number: u32, chapter_memo: Option<&str>) -> String {
    let memo_block = match chapter_memo {
        Some(m) if !m.trim().is_empty() => {
            format!("\n## Chapter Memo (for reference — do not modify the memo itself)\n{}\n", m)
        }
        _ => String::new(),
    };

    format!(
        r###"Polish Chapter {chapter_number}.{memo_block}
## Prose to Polish
{chapter_content}"###,
        chapter_number = chapter_number,
        memo_block = memo_block,
        chapter_content = chapter_content,
    )
}

/// 去除可能的 ``` 代码块包裹
fn strip_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    // 去掉开头的 ``` 和可能的语言标识行
    let after_open = &trimmed[3..];
    let inner_start = after_open.find('\n').map(|p| p + 1).unwrap_or(0);
    let inner = &after_open[inner_start..];
    // 去掉结尾的 ```
    let inner = inner.trim_end();
    match inner.strip_suffix("```") {
        Some(rest) => rest.trim().to_string(),
        None => inner.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_code_fence_with_lang() {
        let content = "```markdown\n这是正文内容。\n```";
        let result = strip_code_fence(content);
        assert_eq!(result, "这是正文内容。");
    }

    #[test]
    fn strips_plain_code_fence() {
        let content = "```\n正文内容\n```";
        let result = strip_code_fence(content);
        assert_eq!(result, "正文内容");
    }

    #[test]
    fn keeps_content_without_fence() {
        let content = "这是普通正文，没有代码块包裹。";
        let result = strip_code_fence(content);
        assert_eq!(result, "这是普通正文，没有代码块包裹。");
    }

    #[test]
    fn detects_unchanged_content() {
        let original = "这是一段正文。";
        let polished = strip_code_fence(original);
        assert_eq!(polished, original);
        assert!(!(polished != original));
    }
}
