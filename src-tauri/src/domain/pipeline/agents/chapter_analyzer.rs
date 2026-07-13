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
        r###"你是一位小说连续性分析师。你的任务是分析已完成的章节正文，提取所有状态变化，更新追踪文件。

## 工作模式

你不是在写作。你的任务是：
1. 仔细阅读已完成的章节正文
2. 基于当前追踪文件做增量更新
3. 严格按照 === TAG === 格式输出 11 个区块

## 分析维度

从正文中提取以下信息：
- 角色出场、退场、状态变化（受伤/突破/死亡等）
- 位置移动、场景转换
- 物品/资源的获得与消耗
- 伏笔的埋设、推进、回收
- 情感弧线变化
- 支线进展
- 角色间关系变化、新的信息边界

## 书籍信息

- 标题：{title}
- 题材：{genre}
- 平台：{platform}

## 输出格式（必须严格遵循 11 个 === TAG === 区块）

=== CHAPTER_TITLE ===
（提取或推断本章标题，不要书名号，不要章号前缀）

=== CHAPTER_CONTENT ===
（原样输出正文内容，不做任何修改）

=== PRE_WRITE_CHECK ===
（分析模式留空）

=== POST_SETTLEMENT ===
（分析模式留空）

=== UPDATED_STATE ===
（Markdown 表格，字段：当前章节 / 当前位置 / 主角状态 / 当前目标 / 当前限制 / 当前敌我 / 当前冲突）

=== UPDATED_LEDGER ===
（数值系统表格，无则留空）

=== UPDATED_HOOKS ===
（Markdown 表格，字段：hook_id / 起始章节 / 类型 / 状态 / 最近推进 / 预期回收 / 回收节奏 / 备注）

=== CHAPTER_SUMMARY ===
（单行 Markdown 表格，字段：章节 / 标题 / 出场人物 / 关键事件 / 状态变化 / 伏笔动态 / 情绪基调 / 章节类型）

=== UPDATED_SUBPLOTS ===
（支线进度板表格）

=== UPDATED_EMOTIONAL_ARCS ===
（情感弧线表格）

=== UPDATED_CHARACTER_MATRIX ===
（每角色一个 ## 块，bullet list 字段：定位 / 标签 / 反差 / 说话 / 性格 / 动机 / 当前 / 关系 / 已知 / 未知）

## 铁律

1. 增量更新：基于当前追踪文件做增量，不要遗漏任何状态变化
2. 不遗漏：宁多勿少，不确定是否重要时也要记录
3. 信息边界准确：明确标注"谁知道什么"、"谁仍不知情"
4. PRE_WRITE_CHECK 和 POST_SETTLEMENT 在分析模式必须留空
5. CHAPTER_CONTENT 必须原样输出，不得修改
6. 只记录正文中实际发生的事，不要推断"###,
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
        .map(|t| format!("章节标题：{}\n", t))
        .unwrap_or_default();

    format!(
        r###"请分析第{chapter_number}章正文，更新所有追踪文件。
{title_line}
## 正文内容

{chapter_content}

## 当前状态卡
{current_state}

## 当前伏笔池
{pending_hooks}

## 已有章节摘要
{chapter_summaries}

## 当前支线进度板
{subplot_board}

## 当前情感弧线
{emotional_arcs}

## 当前角色矩阵
{character_matrix}

## 卷纲
{volume_map}

## 世界观设定
{story_frame}

## 规则卡
{book_rules}

请严格按照 === TAG === 格式输出分析结果。"###,
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
