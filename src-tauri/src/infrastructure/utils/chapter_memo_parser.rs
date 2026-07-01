//! Chapter memo parser — S5 严格解析。
//!
//! 移植自 inkos `chapter-memo-parser.ts`，对齐 8 个必需 H2 小节 + minContentChars
//! 阈值的契约。失败时返回 `AppError::internal`，错误信息遵循 inkos 的
//! `PlannerParseError` 语义（missing sections / empty sections / empty goal）。
//!
//! ## 与原版的差异
//!
//! - 返回 `Result<ChapterMemo, AppError>` 而非抛 `PlannerParseError`
//! - Zod schema 校验由 serde 反序列化 + 字段约束检查等价实现
//! - 错误信息中保留 "PlannerParseError" 前缀以方便日志聚合

use crate::core::agent::governance::ChapterMemo;
use crate::shared::errors::AppError;

/// 必需 H2 小节定义。
///
/// `zh` / `en` 是同义标题，解析时择一匹配。`min_content_chars` 是该节
/// payload 的最小字符数（去空白后），低于此值视为"空壳"。
struct RequiredSection {
    zh: &'static str,
    en: &'static str,
    min_content_chars: usize,
}

const REQUIRED_SECTIONS: &[RequiredSection] = &[
    RequiredSection { zh: "## 当前任务", en: "## Current task", min_content_chars: 20 },
    RequiredSection { zh: "## 读者此刻在等什么", en: "## What the reader is waiting for right now", min_content_chars: 20 },
    RequiredSection { zh: "## 该兑现的 / 暂不掀的", en: "## To pay off / to keep buried", min_content_chars: 20 },
    RequiredSection { zh: "## 日常/过渡承担什么任务", en: "## What the slow / transitional beats carry", min_content_chars: 20 },
    RequiredSection { zh: "## 关键抉择过三连问", en: "## Three-question check on the key choice", min_content_chars: 20 },
    RequiredSection { zh: "## 章尾必须发生的改变", en: "## Required end-of-chapter change", min_content_chars: 20 },
    RequiredSection { zh: "## 本章 hook 账", en: "## Hook ledger for this chapter", min_content_chars: 20 },
    RequiredSection { zh: "## 不要做", en: "## Do not", min_content_chars: 1 },
];

const GOAL_HEADINGS: &[&str] = &["## 本章目标", "## Chapter goal"];
const THREAD_HEADINGS: &[&str] = &["## 关联线索", "## Thread refs", "## Related threads"];

const GOAL_MAX_DISPLAY_CHARS: usize = 50;

/// 严格解析 planner 产出的 markdown memo。
///
/// 输入：LLM 原始输出（可能含 ```markdown fence 或助理寒暄）
/// 输出：`ChapterMemo` 或 `AppError::internal`（含 PlannerParseError 语义）
pub fn parse_chapter_memo(
    raw: &str,
    expected_chapter: u32,
    is_golden_opening: bool,
) -> Result<ChapterMemo, AppError> {
    let markdown = drop_leading_prose(&strip_wrapping_fence(raw));
    let goal = extract_goal(&markdown);
    let body = extract_memo_body(&markdown);
    let thread_refs = extract_thread_refs(&markdown);

    // 1. goal 非空检查
    if goal.is_empty() {
        return Err(AppError::internal(
            "PlannerParseError: goal must be a non-empty string".to_string()
        ));
    }
    let display_goal = make_display_goal(&goal);

    // 2. 必需小节存在性检查
    let missing: Vec<&str> = REQUIRED_SECTIONS.iter()
        .filter(|s| !body.contains(s.zh) && !body.contains(s.en))
        .map(|s| s.zh)
        .collect();
    if !missing.is_empty() {
        return Err(AppError::internal(format!(
            "PlannerParseError: missing sections: {}", missing.join(", ")
        )));
    }

    // 3. 必需小节内容非空检查（≥ min_content_chars）
    let empty: Vec<String> = REQUIRED_SECTIONS.iter()
        .filter_map(|s| {
            let heading = if body.contains(s.zh) { s.zh } else { s.en };
            let content = extract_section_content(&body, heading);
            if content.chars().count() < s.min_content_chars {
                Some(format!("{} (need ≥ {} chars)", s.zh, s.min_content_chars))
            } else {
                None
            }
        })
        .collect();
    if !empty.is_empty() {
        return Err(AppError::internal(format!(
            "PlannerParseError: empty sections: {}", empty.join(", ")
        )));
    }

    // 4. 构造 ChapterMemo（display goal 截断时把完整 goal 前置到 body）
    let final_body = prepend_full_goal_if_needed(&markdown, &body, &goal, &display_goal);

    Ok(ChapterMemo {
        chapter: expected_chapter,
        goal: display_goal,
        is_golden_opening,
        body: final_body,
        thread_refs,
    })
}

/// 提取某 heading 到下一个 `## ` 之间的内容，去除多余空白。
fn extract_section_content(body: &str, heading: &str) -> String {
    let start = match body.find(heading) {
        Some(idx) => idx,
        None => return String::new(),
    };
    let after = &body[start + heading.len()..];
    // 找下一个 H2 标题（独占一行，以 \n## 开头）
    let next = after.find("\n## ").unwrap_or(after.len());
    let section_raw = &after[..next];
    // 把所有空白（含换行）压缩成单空格
    let mut result = String::with_capacity(section_raw.len());
    let mut prev_ws = false;
    for ch in section_raw.chars() {
        if ch.is_whitespace() {
            if !prev_ws && !result.is_empty() {
                result.push(' ');
            }
            prev_ws = true;
        } else {
            result.push(ch);
            prev_ws = false;
        }
    }
    result.trim_end().to_string()
}

/// 剥离 markdown code fence（```markdown ... ```）。
fn strip_wrapping_fence(raw: &str) -> String {
    let trimmed = raw.trim();
    // 匹配 ```md / ```markdown / ``` 包裹
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        // 跳过 fence 后的语言标记（md / markdown 等）
        let lang_end = after_fence.find('\n').unwrap_or(after_fence.len());
        let lang = &after_fence[..lang_end].trim();
        if lang.is_empty() || lang.eq_ignore_ascii_case("md") || lang.eq_ignore_ascii_case("markdown") {
            // 寻找闭合 fence
            let body_start = lang_end + 1;
            if let Some(body) = trimmed.get(body_start..) {
                if let Some(end) = body.rfind("```") {
                    return body[..end].trim().to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

/// 剥离 LLM 助理寒暄（"好的，下面是..."等），保留从第一个 memo 标题开始的内容。
fn drop_leading_prose(raw: &str) -> String {
    let mut markers: Vec<&str> = Vec::new();
    markers.extend_from_slice(GOAL_HEADINGS);
    markers.extend_from_slice(THREAD_HEADINGS);
    for s in REQUIRED_SECTIONS {
        markers.push(s.zh);
        markers.push(s.en);
    }
    markers.extend_from_slice(&["# 第 ", "# Chapter "]);

    let mut first: Option<usize> = None;
    for marker in markers {
        if let Some(idx) = raw.find(marker) {
            first = Some(match first {
                None => idx,
                Some(prev) => prev.min(idx),
            });
        }
    }
    match first {
        Some(idx) => raw[idx..].trim().to_string(),
        None => raw.trim().to_string(),
    }
}

/// 从任一 heading 提取内容，返回第一个匹配的。
fn extract_any_heading(body: &str, headings: &[&str]) -> String {
    for heading in headings {
        let content = extract_section_content(body, heading);
        if !content.is_empty() {
            return content;
        }
    }
    String::new()
}

/// 从 `## 本章目标` 或 `## Chapter goal` 提取首行作为 goal。
fn extract_goal(markdown: &str) -> String {
    let explicit = extract_any_heading(markdown, GOAL_HEADINGS);
    if explicit.is_empty() {
        return String::new();
    }
    // 取首句（按 \n / 。 / ". " 分隔）
    let first_sentence = explicit
        .split(|c| c == '\n' || c == '。')
        .next()
        .unwrap_or("")
        .trim();
    // 处理 ". " 分隔（英文句号 + 空格）
    first_sentence.split(". ").next().unwrap_or("").trim().to_string()
}

/// 从 `## 关联线索` 提取 hook id（格式 `[A-Za-z][A-Za-z0-9_-]*\d+`）。
fn extract_thread_refs(markdown: &str) -> Vec<String> {
    let block = extract_any_heading(markdown, THREAD_HEADINGS);
    if block.is_empty() {
        return Vec::new();
    }
    let trimmed = block.trim();
    // "无" / "none" / "n/a" / "—" 等表示无关联线索
    if matches!(trimmed.to_lowercase().as_str(), "无" | "none" | "n/a" | "na" | "—" | "-" | "(none)") {
        return Vec::new();
    }
    // 匹配 hook id：字母开头，含字母数字下划线连字符，必须含至少一个数字。
    // 关键：使用 is_ascii_alphanumeric() —— is_alphanumeric() 对中文字符返回 true，
    // 会导致 "H007日记新角色" 被当作单个 token。inkos 原版用 regex \b[A-Za-z]...\b
    // 只匹配 ASCII，这里对齐其行为。
    let mut refs = Vec::new();
    let mut current = String::new();
    let mut has_alpha = false;
    let mut has_digit = false;
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            if ch.is_ascii_alphabetic() {
                has_alpha = true;
            } else if ch.is_ascii_digit() {
                has_digit = true;
            }
            current.push(ch);
        } else {
            if has_alpha && has_digit && !current.is_empty() {
                if !refs.contains(&current) {
                    refs.push(current.clone());
                }
            }
            current.clear();
            has_alpha = false;
            has_digit = false;
        }
    }
    if has_alpha && has_digit && !current.is_empty() && !refs.contains(&current) {
        refs.push(current);
    }
    refs
}

/// 从第一个 REQUIRED_SECTIONS 标题开始截取 body（剥除前置寒暄和 goal/thread 段）。
fn extract_memo_body(markdown: &str) -> String {
    let mut first: Option<usize> = None;
    for s in REQUIRED_SECTIONS {
        for heading in [s.zh, s.en] {
            if let Some(idx) = markdown.find(heading) {
                first = Some(match first {
                    None => idx,
                    Some(prev) => prev.min(idx),
                });
            }
        }
    }
    match first {
        Some(idx) => markdown[idx..].trim().to_string(),
        None => markdown.trim().to_string(),
    }
}

/// goal 截断为 ≤ 50 字符，超长加省略号。
fn make_display_goal(goal: &str) -> String {
    let char_count = goal.chars().count();
    if char_count <= GOAL_MAX_DISPLAY_CHARS {
        return goal.to_string();
    }
    // 取前 47 字符 + "..."（中英文统一用 char count）
    let truncated: String = goal.chars().take(47).collect();
    format!("{}...", truncated.trim_end())
}

/// 如果 display goal 被截断，把完整 goal 前置到 body 顶部。
fn prepend_full_goal_if_needed(
    markdown: &str,
    body: &str,
    full_goal: &str,
    display_goal: &str,
) -> String {
    if full_goal == display_goal {
        return body.to_string();
    }
    let heading = if markdown.contains("## Chapter goal") {
        "## Chapter goal"
    } else {
        "## 本章目标"
    };
    format!("{}\n{}\n\n{}", heading, full_goal, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_memo_body() -> &'static str {
        "## 当前任务\n主角发现地图上的标记指向废弃教堂，决定独自前往调查，避免牵连同伴。\n\n\
         ## 读者此刻在等什么\n读者期待主角进入教堂后的发现，以及是否能找到父亲的线索。\n\n\
         ## 该兑现的 / 暂不掀的\n兑现：父亲失踪前留下的密码线索；暂不掀：幕后组织的真实身份。\n\n\
         ## 日常/过渡承担什么任务\n通过主角整理背包的过程，展现他对父亲的感情和调查决心。\n\n\
         ## 关键抉择过三连问\n1. 为何独自前往？2. 是否该信任旧友？3. 万一被困如何脱身？\n\n\
         ## 章尾必须发生的改变\n主角在教堂地下室发现一本日记，揭开父亲失踪当夜的行踪。\n\n\
         ## 关联线索\nH001（父亲密码线索）, H007（日记新角色）\n\n\
         ## 本章 hook 账\nadvance H001：父亲密码首次破解；open H007：日记中提到的新角色。\n\n\
         ## 不要做\n不要让主角直接遇到幕后组织成员，避免过早暴露主线。"
    }

    #[test]
    fn test_parse_valid_memo() {
        let raw = format!("## 本章目标\n揭露废弃教堂的秘密\n\n{}", valid_memo_body());
        let memo = parse_chapter_memo(&raw, 5, false).unwrap();
        assert_eq!(memo.chapter, 5);
        assert_eq!(memo.goal, "揭露废弃教堂的秘密");
        assert!(!memo.is_golden_opening);
        assert_eq!(memo.thread_refs, vec!["H001".to_string(), "H007".to_string()]);
        // body 包含完整 8 个小节
        assert!(memo.body.contains("## 当前任务"));
        assert!(memo.body.contains("## 不要做"));
    }

    #[test]
    fn test_strip_wrapping_fence() {
        let raw = format!("```markdown\n## 本章目标\n测试目标\n\n{}\n```", valid_memo_body());
        let memo = parse_chapter_memo(&raw, 1, true).unwrap();
        assert_eq!(memo.goal, "测试目标");
        assert!(memo.is_golden_opening); // chapter 1 是黄金开篇
    }

    #[test]
    fn test_drop_leading_prose() {
        let raw = format!(
            "好的，下面是第 5 章的 memo：\n\n## 本章目标\n揭露秘密\n\n{}",
            valid_memo_body()
        );
        let memo = parse_chapter_memo(&raw, 5, false).unwrap();
        assert_eq!(memo.goal, "揭露秘密");
    }

    #[test]
    fn test_empty_goal_errors() {
        let raw = format!("## 本章目标\n\n{}", valid_memo_body());
        let err = parse_chapter_memo(&raw, 5, false).unwrap_err();
        assert!(err.to_string().contains("goal must be a non-empty string"));
    }

    #[test]
    fn test_missing_section_errors() {
        // 移除"## 章尾必须发生的改变"小节
        let incomplete = valid_memo_body().replace(
            "## 章尾必须发生的改变\n主角在教堂地下室发现一本日记，揭开父亲失踪当夜的行踪。\n\n",
            "",
        );
        let raw = format!("## 本章目标\n揭露秘密\n\n{}", incomplete);
        let err = parse_chapter_memo(&raw, 5, false).unwrap_err();
        assert!(err.to_string().contains("missing sections: ## 章尾必须发生的改变"));
    }

    #[test]
    fn test_empty_section_payload_errors() {
        // "## 当前任务" 后跟空内容
        let body = valid_memo_body().replace(
            "## 当前任务\n主角发现地图上的标记指向废弃教堂，决定独自前往调查，避免牵连同伴。",
            "## 当前任务\n短",
        );
        let raw = format!("## 本章目标\n揭露秘密\n\n{}", body);
        let err = parse_chapter_memo(&raw, 5, false).unwrap_err();
        assert!(err.to_string().contains("empty sections"));
        assert!(err.to_string().contains("## 当前任务 (need ≥ 20 chars)"));
    }

    #[test]
    fn test_do_not_section_allows_minimal() {
        // "## 不要做" 只需要 1 字符，"无" 是合法的
        let body = valid_memo_body().replace(
            "不要让主角直接遇到幕后组织成员，避免过早暴露主线。",
            "无",
        );
        let raw = format!("## 本章目标\n揭露秘密\n\n{}", body);
        let memo = parse_chapter_memo(&raw, 5, false).unwrap();
        assert!(memo.body.contains("## 不要做\n无"));
    }

    #[test]
    fn test_long_goal_truncated() {
        // 51 个字符（超过 50 字符上限）
        let long_goal = "这是一段非常非常长的章节目标描述用来测试 goal 截断逻辑是否能正确在五十个字符处截断并添加省略号再加几个字";
        let raw = format!("## 本章目标\n{}\n\n{}", long_goal, valid_memo_body());
        let memo = parse_chapter_memo(&raw, 5, false).unwrap();
        assert!(memo.goal.ends_with("..."), "goal should end with ..., got: {}", memo.goal);
        assert!(memo.goal.chars().count() <= 50);
        // 完整 goal 应该被前置到 body
        assert!(memo.body.contains(long_goal));
    }

    #[test]
    fn test_english_headings() {
        let en_body = "## Current task\nThe protagonist discovers a map marker pointing to an abandoned church.\n\n\
            ## What the reader is waiting for right now\nReaders expect what the protagonist finds inside the church.\n\n\
            ## To pay off / to keep buried\nPay off: father's password clue; keep buried: antagonist identity.\n\n\
            ## What the slow / transitional beats carry\nShow protagonist's emotional bond with father through packing.\n\n\
            ## Three-question check on the key choice\n1. Why go alone? 2. Trust old friend? 3. Escape if trapped?\n\n\
            ## Required end-of-chapter change\nProtagonist discovers a diary in the church basement.\n\n\
            ## Thread refs\nH001 (father's code), H007 (new character)\n\n\
            ## Hook ledger for this chapter\nadvance H001: father's code decoded; open H007: new character mentioned.\n\n\
            ## Do not\nNone";
        let raw = format!("## Chapter goal\nReveal the church secret\n\n{}", en_body);
        let memo = parse_chapter_memo(&raw, 7, false).unwrap();
        assert_eq!(memo.goal, "Reveal the church secret");
        assert_eq!(memo.thread_refs, vec!["H001".to_string(), "H007".to_string()]);
    }

    #[test]
    fn test_thread_refs_deduplication() {
        // 把 ## 关联线索 的内容替换为含重复 H001/H007 的列表，验证去重
        let body = valid_memo_body().replace(
            "H001（父亲密码线索）, H007（日记新角色）",
            "H001, H007, H001, H007, H001",
        );
        let raw = format!("## 本章目标\n揭露秘密\n\n{}", body);
        let memo = parse_chapter_memo(&raw, 5, false).unwrap();
        // H001 应该只出现一次（去重）
        assert_eq!(memo.thread_refs.iter().filter(|r| r.as_str() == "H001").count(), 1);
        assert_eq!(memo.thread_refs.iter().filter(|r| r.as_str() == "H007").count(), 1);
    }

    #[test]
    fn test_thread_refs_none_keyword() {
        // ## 关联线索 写 "无" 应返回空 thread_refs（与 inkos 行为对齐）
        let body = valid_memo_body().replace(
            "H001（父亲密码线索）, H007（日记新角色）",
            "无",
        );
        let raw = format!("## 本章目标\n揭露秘密\n\n{}", body);
        let memo = parse_chapter_memo(&raw, 5, false).unwrap();
        assert!(memo.thread_refs.is_empty());
    }

    #[test]
    fn test_is_golden_opening_chapter_3() {
        let raw = format!("## 本章目标\n开篇秘密\n\n{}", valid_memo_body());
        let memo = parse_chapter_memo(&raw, 3, true).unwrap();
        assert!(memo.is_golden_opening);
    }

    #[test]
    fn test_is_golden_opening_chapter_4_false() {
        let raw = format!("## 本章目标\n后续章节\n\n{}", valid_memo_body());
        let memo = parse_chapter_memo(&raw, 4, false).unwrap();
        assert!(!memo.is_golden_opening);
    }
}
