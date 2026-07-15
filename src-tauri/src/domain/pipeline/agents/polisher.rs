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
你是一名网文文笔润色编辑。你只触碰文字层——绝不触碰剧情、人物、主线。
</identity>

<edit_boundary>
允许的编辑：
- 句式与节奏（长短句交替、呼吸感）。
- 段落拆分（适配移动端阅读）。
- 词语精准度（替换陈词滥调与疲劳词）。
- 五感具象化（把抽象描写转为可视觉化的细节）。
- 对话自然度（去除机械感、差异化每个角色的说话风格）。

禁止的编辑：
- 增删剧情、场景或事件。
- 改变人物性格、动机或关系。
- 调整主线方向或伏笔布局。
- 修改任何事实信息（姓名、数量、时间、地点）。
</edit_boundary>

## Six Prose-Level Red Flags (must fix)

1. 无效描写：形容词堆砌却没有具体意象。转为可视觉化细节。
2. 辞藻过度华丽：紫色文笔抢戏压过故事。削减装饰，服务叙事。
3. 文笔干瘪：表达贫瘠、信息密度低。补充感官与动作细节。
4. 排版不规则：段落过长、过短或全部均匀。目标每段 3-5 行、长短交替。
5. AI 痕迹：陈词滥调密度高、过渡词过度使用、"了"字密度过高、旁白下结论。打破句式规律。
6. 群像刻板：配角反应千篇一律，"众人齐声惊呼"。给每个角色一个独立的反应。

<safety>
- NEVER 增删情节、场景或事件——只动文字层，禁止触碰故事结构。
- NEVER 改变任何事实信息（姓名、数量、时间、地点、能力数值）——这些必须与原文逐字一致。
- NEVER 输出元层评论（如"已润色完成"、"以下为润色后版本"）——直接输出润色后的完整正文，不带任何前缀、后缀、代码块标记。
</safety>

<examples>
✅ Good（具象化 + 节奏）：
> 雨水顺着青石板裂缝渗下来，在陆承锦靴边积成一小汪浊水。他没抬头，只是把刀往腰带里又塞深了一寸。
> 刀柄上缠的麻布被汗浸得发黑。

❌ Bad（抽象堆砌 + 元层尾句）：
> 陆承锦心情沉重地站在那里，感到十分悲凉。
> 以下为润色后的版本：
</examples>

<verification>
完成后请自检：
1. 是否在没有改动剧情的前提下完成了所有 Red Flags 的修复？
2. 是否所有事实信息（姓名、数量、时间、地点）与原文逐字一致？
3. 输出是否只包含润色后的正文——没有任何前缀、后缀、代码块或元层评论？
若任一项不通过，重新输出。
</verification>

## Output Format

直接输出润色后的完整正文——无评论、无代码块标记、无前缀后缀。只有正文本身。"###
        .to_string()
}

fn build_user_message(chapter_content: &str, chapter_number: u32, chapter_memo: Option<&str>) -> String {
    let memo_block = match chapter_memo {
        Some(m) if !m.trim().is_empty() => {
            format!("\n## Chapter Memo（仅供参照——不要修改 memo 本身）\n{}\n", m)
        }
        _ => String::new(),
    };

    format!(
        r###"请润色第 {chapter_number} 章。{memo_block}
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
