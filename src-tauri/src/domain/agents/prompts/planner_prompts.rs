pub fn build_system_prompt(language: &str) -> String {
    match language {
        "en" => {
            r#"You are this novel's editor-in-chief. Your job is to produce a chapter_memo for the next chapter. You do NOT write prose — you plan what this chapter must accomplish.

## Output format (strict)

# Chapter N memo

## Chapter goal
<one sentence, <= 50 chars>

## Thread refs
- H0XX

## Current task
<concrete action the protagonist must complete>

## What the reader is waiting for right now
<what the reader expects, what this chapter does with that expectation>

## To pay off / to keep buried
- Pay off: X
- Keep buried: Y

## Required end-of-chapter change
<1-3 concrete changes>

## Hook ledger for this chapter
open:
- [new] description
advance:
- H0XX description
resolve:
- H0XX description
defer:
- H0XX description

## Do not
<2-4 hard prohibitions>"#
        }
        _ => {
            r#"你是这本小说的创作总编，职责是为下一章产生一份 chapter_memo。你不写正文——你只规划这章要完成什么、兑现什么、不要做什么。

## 输出格式（严格遵守）

# 第 N 章 memo

## 本章目标
<一句话，不超过 50 字>

## 关联线索
- H0XX

## 当前任务
<本章主角要完成的具体动作>

## 读者此刻在等什么
<读者期待什么，本章对这个期待做什么>

## 该兑现的 / 暂不掀的
- 该兑现：X
- 暂不掀：Y

## 章尾必须发生的改变
<1-3条具体改变>

## 本章 hook 账
open:
- [new] 新钩子描述
advance:
- H0XX 描述
resolve:
- H0XX 描述
defer:
- H0XX 描述

## 不要做
<2-4条硬约束>"#
        }
    }.to_string()
}

pub fn build_user_message(
    book_dir: &std::path::Path,
    chapter_number: u32,
    external_context: Option<&str>,
    _language: &str,
) -> Result<String, crate::errors::AppError> {
    let story_dir = book_dir.join("story");

    let read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let outline_dir = story_dir.join("outline");
    let volume_map = {
        let primary = read_safe(&outline_dir.join("volume_map.md"));
        if primary.is_empty() {
            read_safe(&story_dir.join("volume_outline.md"))
        } else {
            primary
        }
    };

    let current_state = read_safe(&story_dir.join("current_state.md"));
    let pending_hooks = read_safe(&story_dir.join("pending_hooks.md"));
    let chapter_summaries = read_safe(&story_dir.join("chapter_summaries.md"));
    let book_rules = read_safe(&story_dir.join("book_rules.md"));
    let author_intent = read_safe(&story_dir.join("author_intent.md"));
    let current_focus = read_safe(&story_dir.join("current_focus.md"));

    let mut parts = vec![
        format!("当前状态：第{}章", chapter_number),
        format!("书级规则：\n{}", book_rules),
        format!("作者意图：\n{}", author_intent),
        format!("当前焦点：\n{}", current_focus),
        format!("当前状态卡：\n{}", current_state),
        format!("伏笔池：\n{}", pending_hooks),
        format!("章节摘要：\n{}", chapter_summaries),
        format!("卷纲：\n{}", volume_map),
    ];

    if let Some(ctx) = external_context {
        parts.push(format!("外部指令：\n{}", ctx));
    }

    Ok(parts.join("\n\n"))
}
