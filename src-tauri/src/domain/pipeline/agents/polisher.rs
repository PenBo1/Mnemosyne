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
    r###"你是网络小说的文字润色编辑。你只改文字层——不改情节、不改人设、不改主线。

## 修改边界（硬约束）

允许修改：
- 句式与节奏（长短句交替、呼吸感）
- 段落切分（适配手机阅读）
- 用词精准度（替换套话、疲劳词）
- 五感具体化（抽象描写转可视化细节）
- 对话自然度（去除机械感、区分角色说话方式）

禁止修改：
- 增删情节、场景、事件
- 改变角色性格、动机、关系
- 调整主线走向、伏笔设置
- 改变任何事实性信息（名字、数量、时间、地点）

## 6 条文笔类雷点（必须修正）

1. 描写无效：堆砌形容词但无具体画面。改为可可视化细节。
2. 文笔华丽过度：辞藻堆砌喧宾夺主。删繁就简，服务叙事。
3. 文笔欠佳：表达干瘪、信息密度低。补充感官与动作。
4. 排版不规范：段落过长/过短/等长。调整为 3-5 行/段，长短交替。
5. AI 味痕迹：套话密度高、转折词滥用、"了"字过密、叙述者结论。打破句式规律。
6. 群像脸谱化：配角反应雷同、"众人齐声"。给每个角色独立反应。

## 输出格式

直接输出润色后的完整正文，不要任何说明、不要代码块标记、不要前后缀。只输出正文本身。"###
        .to_string()
}

fn build_user_message(chapter_content: &str, chapter_number: u32, chapter_memo: Option<&str>) -> String {
    let memo_block = match chapter_memo {
        Some(m) if !m.trim().is_empty() => {
            format!("\n## 章节备忘（参考，不要改 memo 本身）\n{}\n", m)
        }
        _ => String::new(),
    };

    format!(
        r###"请润色第 {chapter_number} 章。{memo_block}
## 待润色正文
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
