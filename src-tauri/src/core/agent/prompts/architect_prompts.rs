use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};

pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let task_prompt = match language {
        "en" => {
            r#"You are a story architecture specialist. Given the author's brief and genre, create the foundational story structure.

## Output format

=== STORY_FRAME ===
<4 prose sections: Theme / Conflict / World Rules + Texture / Resolution Direction>

=== VOLUME_MAP ===
<5 prose sections + rhythm principles tail>

=== ROLES ===
<One card per character; protagonist card carries full arc>

=== BOOK_RULES ===
<Markdown rules card>

=== PENDING_HOOKS ===
<13-column table; may contain seed rows with startChapter=0>

## Rules
- World must be internally consistent
- Characters must have depth, avoid stereotypes
- Book rules must be concrete and executable
- No more than 5 main characters"#
        }
        _ => {
            r#"你是一位小说架构师。你的职责是根据创作简报创建小说的基础设定，包括世界观、主要角色、故事主线和核心冲突。

## 输出格式

=== STORY_FRAME ===
<4 段散文：主题 / 冲突 / 世界观铁律+质感 / 终局方向>

=== VOLUME_MAP ===
<5 段散文 + 尾段「节奏原则」>

=== ROLES ===
<一人一卡；主角卡承载完整弧线>

=== BOOK_RULES ===
<Markdown 规则卡>

=== PENDING_HOOKS ===
<13 列伏笔表；可含 startChapter=0 种子行>

## 规则
- 世界观必须内部自洽
- 角色必须有深度，避免脸谱化
- 书级规则必须具体可执行
- 主要角色不超过 5 人"#
        }
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
}

pub fn build_user_prompt(
    book: &crate::features::story::BookConfig,
    external_context: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("书名：{}", book.title),
        format!("题材：{}", book.genre),
        format!("平台：{}", book.platform),
        format!("语言：{}", book.language),
        format!("目标章数：{}", book.target_chapters),
        format!("每章字数：{}", book.chapter_words),
    ];

    if let Some(ctx) = external_context {
        parts.push(format!("\n创作方向/外部指令：\n{}", ctx));
    }

    parts.join("\n")
}
