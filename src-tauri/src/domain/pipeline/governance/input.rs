// 输入治理（context-assembly + governed-context + planning-materials + input-governance）。

use std::path::Path;
use crate::shared::error::AppError;
use super::super::types::{BookConfig, Language};
use super::length::LengthSpec;

// ── 核心类型 ──────────

/// 章节意图
/// 结构化意图：goal/conflicts/subplots/arcBeats/emotionalTarget
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChapterIntent {
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub subplots: Vec<String>,
    #[serde(default)]
    pub arc_beats: Vec<String>,
    #[serde(default)]
    pub emotional_target: String,
}

/// 章节备忘
/// planner 产出的稀疏 memo：goal + body（可选骨架）
#[derive(Debug, Clone, Default)]
pub struct ChapterMemo {
    pub goal: String,
    pub body: String,
}

/// 上下文包
/// composer 编译后的上下文 Markdown（受保护 + 可压缩条目）
#[derive(Debug, Clone, Default)]
pub struct ContextPackage {
    pub markdown: String,
}

/// 规则栈
/// 编译后的规则文本（book_rules + style_guide + genre 约束）
#[derive(Debug, Clone, Default)]
pub struct RuleStack {
    pub markdown: String,
}

/// 受治理的章节计划
#[derive(Debug, Clone)]
pub struct GovernedPlan {
    pub intent: ChapterIntent,
    pub intent_markdown: String,
    pub memo: ChapterMemo,
}

/// 受治理的上下文组合
#[derive(Debug, Clone)]
pub struct GovernedComposed {
    pub context_package: ContextPackage,
    pub rule_stack: RuleStack,
}

/// 受治理的章节工件
/// plan + composed 的聚合，writeDraft/auditDraft/reviseDraft 共用
#[derive(Debug, Clone)]
pub struct GovernedArtifacts {
    pub plan: GovernedPlan,
    pub composed: GovernedComposed,
}

// ── context-assembly.ts → compile_context_package ──────────────

/// 组装章节上下文包
/// 从 truth files + outline + recent chapters 组装 Markdown。
pub fn compile_context_package(
    book_dir: &Path,
    chapter_number: u32,
    language: Language,
) -> Result<ContextPackage, AppError> {
    let _ = language; // 保留参数，后续按语言切换标签
    let story_dir = book_dir.join("story");
    let mut sections: Vec<String> = Vec::new();

    // 1. 当前状态卡
    if let Ok(content) = std::fs::read_to_string(story_dir.join("current_state.md")) {
        if !content.trim().is_empty() {
            sections.push(format!("## 当前状态卡\n\n{}", content.trim()));
        }
    }

    // 2. 伏笔池
    if let Ok(content) = std::fs::read_to_string(story_dir.join("pending_hooks.md")) {
        if !content.trim().is_empty() {
            sections.push(format!("## 伏笔池\n\n{}", content.trim()));
        }
    }

    // 3. 章节摘要（最近 10 章）
    if let Ok(content) = std::fs::read_to_string(story_dir.join("chapter_summaries.md")) {
        let trimmed = trim_recent_summaries(&content, 10);
        if !trimmed.is_empty() {
            sections.push(format!("## 章节摘要\n\n{}", trimmed));
        }
    }

    // 4. 卷纲
    let volume_map = read_outline_file(&story_dir, "volume_map.md");
    if !volume_map.is_empty() {
        sections.push(format!("## 卷纲\n\n{}", volume_map));
    }

    // 5. 世界观设定
    let story_frame = read_outline_file(&story_dir, "story_frame.md");
    if !story_frame.is_empty() {
        sections.push(format!("## 世界观设定\n\n{}", story_frame));
    }

    // 6. 上一章全文（衔接检查）
    if chapter_number > 1 {
        if let Some(prev_content) = read_chapter_content(book_dir, chapter_number - 1) {
            let excerpt = truncate_content(&prev_content, 2000);
            sections.push(format!("## 上一章正文（节选）\n\n{}", excerpt));
        }
    }

    let markdown = sections.join("\n\n---\n\n");
    Ok(ContextPackage { markdown })
}

/// 读取大纲文件：优先 outline/{name}（Phase 5+），缺失则回退到 story/{name}（legacy）
fn read_outline_file(story_dir: &Path, name: &str) -> String {
    let outline_path = story_dir.join("outline").join(name);
    if let Ok(content) = std::fs::read_to_string(&outline_path) {
        if !content.trim().is_empty() {
            return content.trim().to_string();
        }
    }
    // legacy 回退
    std::fs::read_to_string(story_dir.join(name))
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// 解析章节摘要 markdown 表格，保留最近 n 条数据行 + 表头
fn trim_recent_summaries(content: &str, n: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut header_lines: Vec<&str> = Vec::new();
    let mut data_lines: Vec<&str> = Vec::new();
    for line in &lines {
        if line.starts_with('|') {
            if header_lines.is_empty() || line.contains("---") {
                header_lines.push(line);
            } else {
                data_lines.push(line);
            }
        }
    }
    if data_lines.is_empty() {
        return String::new();
    }
    let start = data_lines.len().saturating_sub(n);
    let recent = &data_lines[start..];
    let mut result = header_lines.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(&recent.join("\n"));
    result
}

/// 截断内容到 max_chars 字符数，超出则追加省略号
fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }
    let truncated: String = content.chars().take(max_chars).collect();
    format!("{}……", truncated)
}

/// 读取章节正文（去除首行标题），文件名匹配 chapters/{:04}-*.md
fn read_chapter_content(book_dir: &Path, chapter_number: u32) -> Option<String> {
    let chapters_dir = book_dir.join("chapters");
    let padded = format!("{:04}", chapter_number);
    let entries = std::fs::read_dir(&chapters_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&padded) && name.ends_with(".md") {
            let content = std::fs::read_to_string(entry.path()).ok()?;
            // 去除首行标题（# 第X章 ...）
            let without_heading = content.lines().skip(1).collect::<Vec<_>>().join("\n");
            return Some(without_heading.trim().to_string());
        }
    }
    None
}

// ── governed-context.ts → compile_rule_stack ───────────────────

/// 组装规则栈
/// book_rules + style_guide 编译为统一规则 Markdown。
pub fn compile_rule_stack(
    book_dir: &Path,
    language: Language,
) -> Result<RuleStack, AppError> {
    let _ = language; // 保留参数
    let story_dir = book_dir.join("story");
    let mut sections: Vec<String> = Vec::new();

    // 1. book_rules.md
    if let Ok(content) = std::fs::read_to_string(story_dir.join("book_rules.md")) {
        if !content.trim().is_empty() {
            sections.push(format!("## 规则卡\n\n{}", content.trim()));
        }
    }

    // 2. style_guide.md
    if let Ok(content) = std::fs::read_to_string(story_dir.join("style_guide.md")) {
        if !content.trim().is_empty() {
            sections.push(format!("## 文风指南\n\n{}", content.trim()));
        }
    }

    let markdown = sections.join("\n\n---\n\n");
    Ok(RuleStack { markdown })
}

// ── planning-materials.ts → ChapterIntent / ChapterMemo 解析 ───

/// 解析 planner 输出为 ChapterMemo
/// planner 输出格式：
/// ## 章节目标
/// {goal}
/// ## 章节备忘
/// {body}
pub fn parse_chapter_memo(planner_output: &str) -> ChapterMemo {
    let goal = extract_section(planner_output, "章节目标")
        .or_else(|| extract_section(planner_output, "GOAL"))
        .unwrap_or_default();
    let body = extract_section(planner_output, "章节备忘")
        .or_else(|| extract_section(planner_output, "MEMO"))
        .unwrap_or_default();
    ChapterMemo { goal: goal.trim().to_string(), body: body.trim().to_string() }
}

/// 解析 ChapterIntent
/// 从 planner 输出的结构化字段提取意图
pub fn parse_chapter_intent(planner_output: &str) -> ChapterIntent {
    let goal = extract_section(planner_output, "章节目标").unwrap_or_default();
    let conflicts = extract_list_field(planner_output, "冲突");
    let subplots = extract_list_field(planner_output, "支线");
    let arc_beats = extract_list_field(planner_output, "弧线节拍");
    let emotional_target = extract_section(planner_output, "情绪目标").unwrap_or_default();

    ChapterIntent {
        goal: goal.trim().to_string(),
        conflicts,
        subplots,
        arc_beats,
        emotional_target: emotional_target.trim().to_string(),
    }
}

/// 提取 `## {heading}` 到下一个 `## ` 之间的内容
fn extract_section(content: &str, heading: &str) -> Option<String> {
    let marker = format!("## {}", heading);
    let start = content.find(&marker)?;
    let content_start = start + marker.len();
    let remaining = &content[content_start..];
    let end = remaining.find("\n## ").map(|pos| content_start + pos).unwrap_or(content.len());
    Some(content[content_start..end].trim().to_string())
}

/// 从某个 `## {field_name}` 小节提取列表项（`- ` 或 `* ` 开头）
fn extract_list_field(content: &str, field_name: &str) -> Vec<String> {
    let section = extract_section(content, field_name).unwrap_or_default();
    section.lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("- ") || l.starts_with("* "))
        .map(|l| l[2..].trim().to_string())
        .collect()
}

// ── GovernedArtifacts 组装 ─────────────────────────────────────

/// 创建受治理的章节工件
/// 组装 plan + composed，供 writeDraft/auditDraft/reviseDraft 共用
pub fn create_governed_artifacts(
    book: &BookConfig,
    book_dir: &Path,
    chapter_number: u32,
    external_context: Option<&str>,
) -> Result<GovernedArtifacts, AppError> {
    let _ = external_context; // 预留：后续注入外部上下文
    let language = book.language.unwrap_or_default();

    // 1. 读取 planner 输出（runtime/ch{:04}_intent.md）
    let intent_path = book_dir.join("story").join("runtime")
        .join(format!("ch{:04}_intent.md", chapter_number));
    let planner_output = std::fs::read_to_string(&intent_path).unwrap_or_default();

    let intent = if planner_output.is_empty() {
        ChapterIntent::default()
    } else {
        parse_chapter_intent(&planner_output)
    };
    let intent_markdown = if planner_output.is_empty() {
        String::new()
    } else {
        planner_output.clone()
    };
    let memo = if planner_output.is_empty() {
        ChapterMemo::default()
    } else {
        parse_chapter_memo(&planner_output)
    };

    let plan = GovernedPlan { intent, intent_markdown, memo };

    // 2. 组装 context + rule stack
    let context_package = compile_context_package(book_dir, chapter_number, language)?;
    let rule_stack = compile_rule_stack(book_dir, language)?;
    let composed = GovernedComposed { context_package, rule_stack };

    Ok(GovernedArtifacts { plan, composed })
}

// ── 测试 ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chapter_memo_extracts_goal_and_body() {
        let input = "## 章节目标\ngoal here\n## 章节备忘\nbody here";
        let memo = parse_chapter_memo(input);
        assert_eq!(memo.goal, "goal here");
        assert_eq!(memo.body, "body here");
    }

    #[test]
    fn parse_chapter_intent_extracts_fields() {
        let input = "## 章节目标\n主角进城\n## 冲突\n- 与守卫对峙\n- 资金短缺\n## 支线\n- 旧友重逢\n## 弧线节拍\n- 转折\n## 情绪目标\n紧张";
        let intent = parse_chapter_intent(input);
        assert_eq!(intent.goal, "主角进城");
        assert_eq!(intent.conflicts, vec!["与守卫对峙".to_string(), "资金短缺".to_string()]);
        assert_eq!(intent.subplots, vec!["旧友重逢".to_string()]);
        assert_eq!(intent.arc_beats, vec!["转折".to_string()]);
        assert_eq!(intent.emotional_target, "紧张");
    }

    #[test]
    fn extract_section_returns_none_when_missing() {
        let content = "## 其他标题\n一些内容";
        assert_eq!(extract_section(content, "章节目标"), None);
    }

    #[test]
    fn trim_recent_summaries_keeps_last_n() {
        // 构造 15 行数据 + 2 行表头（表头行 + 分隔行）
        let mut lines: Vec<String> = Vec::new();
        lines.push("| 章节 | 标题 | 摘要 |".to_string());
        lines.push("| --- | --- | --- |".to_string());
        for i in 1..=15 {
            lines.push(format!("| {} | 标题{} | 摘要{} |", i, i, i));
        }
        let content = lines.join("\n");
        let trimmed = trim_recent_summaries(&content, 10);
        let trimmed_lines: Vec<&str> = trimmed.lines().collect();
        // 2 行表头 + 10 行数据
        assert_eq!(trimmed_lines.len(), 12);
        // 数据行应是第 6..=15 行（最近 10 条）
        assert!(trimmed_lines[2].contains("标题6"));
        assert!(trimmed_lines[11].contains("标题15"));
    }

    #[test]
    fn truncate_content_adds_ellipsis() {
        // 构造 3000 个字符
        let content: String = "a".repeat(3000);
        let truncated = truncate_content(&content, 2000);
        assert_eq!(truncated.chars().count(), 2002); // 2000 + 省略号 2 字符
        assert!(truncated.ends_with("……"));
    }

    #[test]
    fn compile_rule_stack_handles_missing_files() {
        // 不存在的 book_dir → 空 RuleStack
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let book_dir = tmp.path().join("nonexistent_book");
        let rule_stack = compile_rule_stack(&book_dir, Language::Zh).expect("compile failed");
        assert_eq!(rule_stack.markdown, "");
    }

    #[test]
    fn truncate_content_preserves_short_input() {
        let content = "短文本";
        let result = truncate_content(content, 2000);
        assert_eq!(result, "短文本");
    }

    #[test]
    fn extract_list_field_handles_star_marker() {
        let content = "## 冲突\n* 冲突一\n* 冲突二";
        let list = extract_list_field(content, "冲突");
        assert_eq!(list, vec!["冲突一".to_string(), "冲突二".to_string()]);
    }

    #[test]
    fn read_outline_file_falls_back_to_legacy() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).expect("failed to create story dir");
        // legacy 文件：直接放在 story_dir 下
        std::fs::write(story_dir.join("story_frame.md"), "legacy frame\n").expect("write failed");
        let result = read_outline_file(&story_dir, "story_frame.md");
        assert_eq!(result, "legacy frame");
    }

    #[test]
    fn read_outline_file_prefers_outline_subdir() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let story_dir = tmp.path().join("story");
        let outline_dir = story_dir.join("outline");
        std::fs::create_dir_all(&outline_dir).expect("failed to create outline dir");
        // 同时存在 outline/ 和 legacy 文件，应优先 outline/
        std::fs::write(outline_dir.join("story_frame.md"), "new frame\n").expect("write failed");
        std::fs::write(story_dir.join("story_frame.md"), "legacy frame\n").expect("write failed");
        let result = read_outline_file(&story_dir, "story_frame.md");
        assert_eq!(result, "new frame");
    }
}
