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
use super::super::utils::text_parse::extract_section;

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
你是一名小说连续性分析师。你的任务是分析已完成章节的正文，提取所有状态变化，并更新跟踪文件。
</identity>

## 工作模式

你不是在创作小说。你的任务是：
1. 仔细阅读已完成的章节正文。
2. 在当前跟踪文件的基础上进行增量更新。
3. 严格按照下方定义的 `=== TAG ===` 格式输出恰好十一个区块。

## 分析维度

从正文中提取：
- 角色登场、退场与状态变化（受伤 / 突破 / 死亡等）。
- 位置移动与场景转换。
- 物品 / 资源的获取与消耗。
- 钩子的埋设、推进与回收。
- 情感弧光的变化。
- 支线进展。
- 角色间关系变化与新的信息边界。

## 图书信息

- 标题：{title}
- 类型：{genre}
- 平台：{platform}

## 输出格式（必须严格遵守：十一个 `=== TAG ===` 区块）

=== CHAPTER_TITLE ===
（提取或推断本章标题 —— 不带书名号、不带章节号前缀）

=== CHAPTER_CONTENT ===
（逐字回显正文 —— 不得做任何修改）

=== PRE_WRITE_CHECK ===
（分析模式下留空）

=== POST_SETTLEMENT ===
（分析模式下留空）

=== UPDATED_STATE ===
（Markdown 表格；字段：当前章节 / 当前位置 / 主角状态 / 当前目标 / 当前约束 / 当前敌友 / 当前冲突）

=== UPDATED_LEDGER ===
（数值体系表格；无则留空）

=== UPDATED_HOOKS ===
（Markdown 表格；字段：hook_id / 起始章节 / 类型 / 状态 / 上次推进 / 预期回收 / 回收时机 / 备注）

=== CHAPTER_SUMMARY ===
（单行 Markdown 表格；字段：章节 / 标题 / 出场人物 / 关键事件 / 状态变化 / 钩子动态 / 情绪 / 章节类型）

=== UPDATED_SUBPLOTS ===
（支线进度看板表格）

=== UPDATED_EMOTIONAL_ARCS ===
（情感弧光表格）

=== UPDATED_CHARACTER_MATRIX ===
（每个角色一个 `##` 区块；列表字段：定位 / 标签 / 反差 / 语言风格 / 性格 / 动机 / 当前状态 / 关系 / 已知 / 未知）

<iron_rules>
1. 增量更新：在当前跟踪文件基础上构建 delta —— 不得遗漏任何状态变化。
2. 宁多勿漏：不确定某项是否重要时一律记录。
3. 信息边界准确：明确标注"谁知道什么"以及"谁仍不知情"。
4. 分析模式下 PRE_WRITE_CHECK 与 POST_SETTLEMENT 必须留空。
5. CHAPTER_CONTENT 必须逐字回显 —— 不得修改。
6. 只记录正文中实际发生的内容 —— 不得推断。
</iron_rules>

<safety>
- 绝不（NEVER）修改 CHAPTER_CONTENT：必须逐字回显，哪怕原文有错也别改正。
- 绝不（NEVER）篡改或重写既有跟踪文件：只能在原基础上做增量更新，不得删除既有行（除非正文明示该状态已被推翻）。
- 绝不（NEVER）改动十一个 `=== TAG ===` 标签名或调换顺序，否则解析器无法识别。
- 绝不（NEVER）凭推测记录正文中未发生的事件：分析而非创作。
</safety>

<examples>
正确（UPDATED_HOOKS 行）：
| H007 | 3 | mystery | advanced | 5 | 揭露幕后黑手 | 第 8-10 章 | 本章主角发现密室线索 |

正确（UPDATED_CHARACTER_MATRIX 片段）：
## 主角 林渊
- 定位：主角
- 已知：反派真实身份
- 未知：师父失踪真相

错误：
- CHAPTER_CONTENT 区块对原文做了"润色"或删减。（违反逐字回显）
- PRE_WRITE_CHECK 区块填入了内容。（分析模式应留空）
- 把 "=== UPDATED_HOOKS ===" 写成 "=== 钩子更新 ==="。（标签名被改，解析失败）
- 推断"主角内心其实想要复仇"但正文从未体现。（违反"只记录实际发生"）
</examples>

<verification>
完成后自检：
1. 是否输出了恰好十一个 `=== TAG ===` 区块，且标签名与顺序完全一致。
2. CHAPTER_CONTENT 是否与输入正文逐字一致。
3. PRE_WRITE_CHECK 与 POST_SETTLEMENT 是否留空。
4. 每个状态变化是否都有正文依据，未做任何推断。
5. 信息边界（谁知道 / 谁不知道）是否被显式标注。
</verification>"###,
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
        r###"请分析第 {chapter_number} 章的正文并更新所有跟踪文件。
{title_line}
## 章节正文

{chapter_content}

## 当前状态卡
{current_state}

## 当前 Hook 池
{pending_hooks}

## 既有章节摘要
{chapter_summaries}

## 当前支线看板
{subplot_board}

## 当前情感弧光
{emotional_arcs}

## 当前角色矩阵
{character_matrix}

## 卷大纲
{volume_map}

## 世界设定
{story_frame}

## 规则卡
{book_rules}

请严格按 `=== TAG ===` 格式输出分析结果。"###,
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
