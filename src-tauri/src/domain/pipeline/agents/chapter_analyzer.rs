// ChapterAnalyzer Agent。
//
// 职责：分析一章已完成的正文，提取所有状态变化，输出 11 个 === TAG === 区块。
//
// 与 writer 的区别：
// - writer 是 3-phase（Creative/Observer/Settler），产出新正文
// - analyzer 是单次 LLM 调用，分析已有正文，输出 11 个 === TAG === 区块
// - PRE_WRITE_CHECK 和 POST_SETTLEMENT 留空（分析模式不需要）

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::super::types::BookConfig;

/// ChapterAnalyzer 输出（11 个 === TAG === 区块）
#[derive(Debug, Clone, Default)]
pub struct AnalyzerOutput {
    pub chapter_title: String,
    pub chapter_content: String,
    pub pre_write_check: String, // 分析模式留空
    pub post_settlement: String, // 分析模式留空
    pub updated_state: String,
    pub updated_ledger: String,
    pub updated_hooks: String,
    pub chapter_summary: String,
    pub updated_subplots: String,
    pub updated_emotional_arcs: String,
    pub updated_character_matrix: String,
}

/// Analyzer 上下文（从前序文件组装）
pub struct AnalyzerContext {
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
    pub volume_map: String,
    pub story_frame: String,
    pub book_rules: String,
    pub subplot_board: String,
    pub emotional_arcs: String,
    pub character_matrix: String,
}

/// 分析一章已完成正文，提取所有状态变化。
///
/// 与 writer 的区别：
/// - writer 是 3-phase（Creative/Observer/Settler），产出新正文
/// - analyzer 是单次 LLM 调用，分析已有正文，输出 11 个 === TAG === 区块
/// - PRE_WRITE_CHECK 和 POST_SETTLEMENT 留空（分析模式不需要）
pub async fn analyze_chapter(
    engine: &AgentEngine,
    book: &BookConfig,
    chapter_number: u32,
    chapter_content: &str,
    chapter_title: Option<&str>,
    ctx: &AnalyzerContext,
) -> Result<AnalyzerOutput, AppError> {
    let system_prompt = build_system_prompt(book);
    let user_message = build_user_message(book, chapter_number, chapter_content, chapter_title, ctx);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_output(&response))
}

// ── System Prompt ──────────────────────────────────────────

fn build_system_prompt(book: &BookConfig) -> String {
    format!(
        r###"<identity>
You are a novel-continuity analyst. Your task is to analyze a finished chapter's prose, extract every state change, and update the tracking files.
</identity>

## Operating Mode

You are not writing fiction. Your task is:
1. Read the finished chapter prose carefully.
2. Apply incremental updates on top of the current tracking files.
3. Emit exactly eleven blocks in the strict `=== TAG ===` format defined below.

## Analysis Dimensions

From the prose, extract:
- Character entrances, exits, and state changes (injury / breakthrough / death, etc.).
- Location moves and scene transitions.
- Acquisition and consumption of items / resources.
- Planting, advancing, and resolving of hooks.
- Emotional-arc shifts.
- Subplot progress.
- Inter-character relationship changes and new information boundaries.

## Book Information

- Title: {title}
- Genre: {genre}
- Platform: {platform}

## Output Format (must follow strictly: eleven `=== TAG ===` blocks)

=== CHAPTER_TITLE ===
(extract or infer this chapter's title — no book-title marks, no chapter-number prefix)

=== CHAPTER_CONTENT ===
(echo the prose verbatim — do not modify anything)

=== PRE_WRITE_CHECK ===
(leave empty in analysis mode)

=== POST_SETTLEMENT ===
(leave empty in analysis mode)

=== UPDATED_STATE ===
(Markdown table; fields: current chapter / current location / protagonist state / current goal / current constraint / current friend-foe / current conflict)

=== UPDATED_LEDGER ===
(numerical-system table; leave empty if none)

=== UPDATED_HOOKS ===
(Markdown table; fields: hook_id / start chapter / type / status / last advanced / expected payoff / payoff timing / notes)

=== CHAPTER_SUMMARY ===
(single-row Markdown table; fields: chapter / title / characters present / key events / state changes / hook activity / mood / chapter type)

=== UPDATED_SUBPLOTS ===
(subplot progress-board table)

=== UPDATED_EMOTIONAL_ARCS ===
(emotional-arc table)

=== UPDATED_CHARACTER_MATRIX ===
(one `##` block per character; bullet-list fields: role / tags / contrast / speech / personality / motivation / current state / relationships / known / unknown)

<iron_rules>
1. Incremental update: build deltas on top of the current tracking files — do not omit any state change.
2. No omission: err on the side of inclusion; when uncertain whether something matters, record it.
3. Information-boundary accuracy: explicitly annotate "who knows what" and "who still does not know."
4. PRE_WRITE_CHECK and POST_SETTLEMENT must be empty in analysis mode.
5. CHAPTER_CONTENT must be echoed verbatim — no modifications.
6. Record only what actually happened in the prose — do not infer.
</iron_rules>"###,
        title = book.title,
        genre = book.genre,
        platform = format!("{:?}", book.platform).to_lowercase(),
    )
}

// ── User Message ───────────────────────────────────────────

fn build_user_message(
    _book: &BookConfig,
    chapter_number: u32,
    chapter_content: &str,
    chapter_title: Option<&str>,
    ctx: &AnalyzerContext,
) -> String {
    let title_line = chapter_title
        .map(|t| format!("Chapter Title: {}\n", t))
        .unwrap_or_default();

    format!(
        r###"Analyze the prose of Chapter {chapter_number} and update all tracking files.
{title_line}
## Chapter Prose

{chapter_content}

## Current State Card
{current_state}

## Current Hook Pool
{pending_hooks}

## Existing Chapter Summaries
{chapter_summaries}

## Current Subplot Board
{subplot_board}

## Current Emotional Arcs
{emotional_arcs}

## Current Character Matrix
{character_matrix}

## Volume Outline
{volume_map}

## World Setting
{story_frame}

## Rule Card
{book_rules}

Emit the analysis strictly in the `=== TAG ===` format."###,
        chapter_number = chapter_number,
        title_line = title_line,
        chapter_content = chapter_content,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        subplot_board = ctx.subplot_board,
        emotional_arcs = ctx.emotional_arcs,
        character_matrix = ctx.character_matrix,
        volume_map = ctx.volume_map,
        story_frame = ctx.story_frame,
        book_rules = ctx.book_rules,
    )
}

// ── 解析输出 ───────────────────────────────────────────────

fn parse_output(content: &str) -> AnalyzerOutput {
    AnalyzerOutput {
        chapter_title: extract_section(content, "CHAPTER_TITLE").unwrap_or_default(),
        chapter_content: extract_section(content, "CHAPTER_CONTENT").unwrap_or_default(),
        pre_write_check: extract_section(content, "PRE_WRITE_CHECK").unwrap_or_default(),
        post_settlement: extract_section(content, "POST_SETTLEMENT").unwrap_or_default(),
        updated_state: extract_section(content, "UPDATED_STATE").unwrap_or_default(),
        updated_ledger: extract_section(content, "UPDATED_LEDGER").unwrap_or_default(),
        updated_hooks: extract_section(content, "UPDATED_HOOKS").unwrap_or_default(),
        chapter_summary: extract_section(content, "CHAPTER_SUMMARY").unwrap_or_default(),
        updated_subplots: extract_section(content, "UPDATED_SUBPLOTS").unwrap_or_default(),
        updated_emotional_arcs: extract_section(content, "UPDATED_EMOTIONAL_ARCS").unwrap_or_default(),
        updated_character_matrix: extract_section(content, "UPDATED_CHARACTER_MATRIX").unwrap_or_default(),
    }
}

/// 从 === TAG === 标记中提取区块内容
fn extract_section(content: &str, tag: &str) -> Option<String> {
    let marker = format!("=== {} ===", tag);
    let start = content.find(&marker)?;
    let content_start = start + marker.len();

    let remaining = &content[content_start..];
    let end = remaining
        .find("\n=== ")
        .map(|pos| content_start + pos)
        .unwrap_or(content.len());

    Some(content[content_start..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_eleven_tags() {
        // 输入包含全部 11 个 === TAG === 区块，验证所有字段都被正确填充
        let response = r#"=== CHAPTER_TITLE ===
暗流

=== CHAPTER_CONTENT ===
这是正文内容。

=== PRE_WRITE_CHECK ===

=== POST_SETTLEMENT ===

=== UPDATED_STATE ===
| 当前章节 | 当前位置 | 主角状态 |
| 1 | 客栈 | 警惕 |

=== UPDATED_LEDGER ===
| 灵石 | 100 |

=== UPDATED_HOOKS ===
| hook_id | 起始章节 | 类型 |
| H001 | 1 | mystery |

=== CHAPTER_SUMMARY ===
| 章节 | 标题 | 出场人物 |
| 1 | 暗流 | 主角,反派 |

=== UPDATED_SUBPLOTS ===
| 支线 | 状态 |
| 师债 | 推进 |

=== UPDATED_EMOTIONAL_ARCS ===
| 角色 | 情绪 |
| 主角 | 紧绷 |

=== UPDATED_CHARACTER_MATRIX ===
## 主角
- 定位：主角
- 标签：修行者"#;
        let output = parse_output(response);
        assert_eq!(output.chapter_title, "暗流");
        assert_eq!(output.chapter_content, "这是正文内容。");
        assert!(output.pre_write_check.is_empty());
        assert!(output.post_settlement.is_empty());
        assert!(output.updated_state.contains("客栈"));
        assert!(output.updated_ledger.contains("灵石"));
        assert!(output.updated_hooks.contains("H001"));
        assert!(output.chapter_summary.contains("暗流"));
        assert!(output.updated_subplots.contains("师债"));
        assert!(output.updated_emotional_arcs.contains("紧绷"));
        assert!(output.updated_character_matrix.contains("主角"));
    }

    #[test]
    fn parses_missing_tags_as_empty() {
        // 输入只有 2 个 tag，其余字段应为空
        let response = "=== CHAPTER_TITLE ===\n标题\n=== CHAPTER_CONTENT ===\n正文\n";
        let output = parse_output(response);
        assert_eq!(output.chapter_title, "标题");
        assert_eq!(output.chapter_content, "正文");
        assert!(output.pre_write_check.is_empty());
        assert!(output.post_settlement.is_empty());
        assert!(output.updated_state.is_empty());
        assert!(output.updated_ledger.is_empty());
        assert!(output.updated_hooks.is_empty());
        assert!(output.chapter_summary.is_empty());
        assert!(output.updated_subplots.is_empty());
        assert!(output.updated_emotional_arcs.is_empty());
        assert!(output.updated_character_matrix.is_empty());
    }

    #[test]
    fn extract_section_handles_no_marker() {
        // tag 未找到时返回 None
        let content = "没有任何标记的普通文本";
        assert_eq!(extract_section(content, "CHAPTER_TITLE"), None);
        assert_eq!(extract_section(content, "UPDATED_STATE"), None);
    }

    #[test]
    fn extract_section_handles_last_block() {
        // 最后一个 tag 后没有尾部 ===，应提取到文本结尾
        let content = "=== CHAPTER_TITLE ===\n暗流";
        let result = extract_section(content, "CHAPTER_TITLE");
        assert_eq!(result, Some("暗流".to_string()));
    }
}
