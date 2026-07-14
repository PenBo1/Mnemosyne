// ContinuityAuditor Agent。
//
// 职责：对写完的章节做 37 维度结构审计，输出 JSON 格式的审计结果。
// 只有 critical 级别问题才判定 passed=false。
//
// 维度激活规则：
// - 默认激活 1-27 + 32-33（基础结构维度）
// - 当 story/parent_canon.md 存在且非 fanfic 模式 → 激活 28-31（番外审查维度）
// - 当 book.fanfic_mode 存在 → 激活 34-37（同人审查维度），并按模式覆盖严重度
//   - canon: 34/35/37 critical, 36 warning
//   - au:    34 critical, 35/37 info, 36 warning
//   - ooc:   34 info, 35/36 warning, 37 info；同时把维度 1 (OOC) 降级为 info
//   - cp:    36 critical, 34/35 warning, 37 info

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::super::types::{BookConfig, FanficMode};

/// 审计问题严重级别
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

/// 修复范围（路由提示）
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RepairScope {
    Local,
    Structural,
    Unknown,
}

/// 审计问题
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AuditIssue {
    pub severity: IssueSeverity,
    #[serde(default)]
    pub repair_scope: Option<RepairScope>,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub suggestion: String,
}

/// 审计结果
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct AuditResult {
    pub passed: bool,
    #[serde(default)]
    pub overall_score: Option<f32>,
    #[serde(default)]
    pub issues: Vec<AuditIssue>,
    #[serde(default)]
    pub summary: String,
    /// 解析失败标记：当 LLM 输出无法解析为 JSON 时设为 true
    #[serde(skip)]
    pub parse_failed: bool,
}

/// Auditor 上下文
pub struct AuditorContext {
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
    pub volume_map: String,
    pub story_frame: String,
    pub book_rules: String,
    pub style_guide: String,
    pub chapter_memo: String,
    pub previous_chapter: String,
    /// 正传正典全文（story/parent_canon.md）。空字符串表示不存在。
    /// 仅当非 fanfic 模式时注入到 user prompt。
    pub parent_canon: String,
    /// 同人正典全文（story/fanfic_canon.md）。空字符串表示不存在。
    /// 仅当 book.fanfic_mode 为 Some 时注入到 user prompt。
    pub fanfic_canon: String,
}

/// 审计一章。
pub async fn audit_chapter(
    engine: &AgentEngine,
    book: &BookConfig,
    chapter_number: u32,
    chapter_title: &str,
    chapter_content: &str,
    ctx: &AuditorContext,
) -> Result<AuditResult, AppError> {
    let fanfic_mode = book.fanfic_mode;
    let has_parent_canon = !ctx.parent_canon.is_empty() && fanfic_mode.is_none();
    let has_fanfic_canon = !ctx.fanfic_canon.is_empty() && fanfic_mode.is_some();

    let system_prompt = build_system_prompt(book, fanfic_mode, has_parent_canon);
    let user_message =
        build_user_message(book, chapter_number, chapter_title, chapter_content, ctx, has_parent_canon, has_fanfic_canon);

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_audit_result(&response))
}

// ── 37 审计维度（中英文标签） ────────────────────────────────

/// 维度标签：id → (中文, 英文)
const DIMENSION_LABELS: &[(u32, &str, &str)] = &[
    (1, "OOC检查", "OOC Check"),
    (2, "时间线检查", "Timeline Check"),
    (3, "设定冲突", "Lore Conflict Check"),
    (4, "战力崩坏", "Power Scaling Check"),
    (5, "数值检查", "Numerical Consistency Check"),
    (6, "伏笔检查", "Hook Check"),
    (7, "节奏检查", "Pacing Check"),
    (8, "文风检查", "Style Check"),
    (9, "信息越界", "Information Boundary Check"),
    (10, "词汇疲劳", "Lexical Fatigue Check"),
    (11, "利益链断裂", "Incentive Chain Check"),
    (12, "年代考据", "Era Accuracy Check"),
    (13, "配角降智", "Side Character Competence Check"),
    (14, "配角工具人化", "Side Character Instrumentalization Check"),
    (15, "爽点虚化", "Payoff Dilution Check"),
    (16, "台词失真", "Dialogue Authenticity Check"),
    (17, "流水账", "Chronicle Drift Check"),
    (18, "知识库污染", "Knowledge Base Pollution Check"),
    (19, "视角一致性", "POV Consistency Check"),
    (20, "段落等长", "Paragraph Uniformity Check"),
    (21, "套话密度", "Cliche Density Check"),
    (22, "公式化转折", "Formulaic Twist Check"),
    (23, "列表式结构", "List-like Structure Check"),
    (24, "支线停滞", "Subplot Stagnation Check"),
    (25, "弧线平坦", "Arc Flatline Check"),
    (26, "节奏单调", "Pacing Monotony Check"),
    (27, "敏感词检查", "Sensitive Content Check"),
    (28, "正传事件冲突", "Mainline Canon Event Conflict"),
    (29, "未来信息泄露", "Future Knowledge Leak Check"),
    (30, "世界规则跨书一致性", "Cross-Book World Rule Check"),
    (31, "番外伏笔隔离", "Spinoff Hook Isolation Check"),
    (32, "读者期待管理", "Reader Expectation Check"),
    (33, "章节备忘偏离", "Chapter Memo Drift Check"),
    (34, "角色还原度", "Character Fidelity Check"),
    (35, "世界规则遵守", "World Rule Compliance Check"),
    (36, "关系动态", "Relationship Dynamics Check"),
    (37, "正典事件一致性", "Canon Event Consistency Check"),
];

// ── Fanfic 维度配置 ─────────────────────────────

/// Fanfic 维度的严重度级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FanficSeverity {
    Critical,
    Warning,
    Info,
}

impl FanficSeverity {
    /// 中文严重度标签。
    fn label_zh(self) -> &'static str {
        match self {
            FanficSeverity::Critical => "（严格检查）",
            FanficSeverity::Warning => "（警告级别）",
            FanficSeverity::Info => "（仅记录，不判定失败）",
        }
    }
}

/// Fanfic 维度配置：激活的维度 id + 严重度覆盖
struct FanficDimensionConfig {
    /// 激活的同人维度 id（始终是 34-37）
    active_ids: &'static [u32],
    /// 维度 id → 严重度。调用方按 baseNote + severity.label_zh() 拼 note。
    severity_overrides: &'static [(u32, FanficSeverity)],
}

/// Fanfic 维度的基础说明（中文）
const FANFIC_DIMENSION_BASE_NOTES: &[(u32, &str)] = &[
    (34, "检查角色的语癖、说话风格、行为模式是否与 fanfic_canon.md 角色档案一致。偏离必须有情境驱动。"),
    (35, "检查章节内容是否违反 fanfic_canon.md 中的世界规则（地理、力量体系、阵营关系）。"),
    (36, "检查角色之间的关系互动是否合理，是否与 fanfic_canon.md 中标注的关键关系一致或有合理发展。"),
    (37, "检查章节是否与 fanfic_canon.md 关键事件时间线矛盾。"),
];

/// 番外维度（28-31）说明，仅当 parent_canon.md 存在且非 fanfic 模式时激活。
const SPINOFF_DIMENSION_NOTES: &[(u32, &str)] = &[
    (28, "检查番外事件是否与正典约束表矛盾"),
    (29, "检查角色是否引用了分歧点之后才揭示的信息（参照信息边界表）"),
    (30, "检查番外是否违反正传世界规则（力量体系、地理、阵营）"),
    (31, "检查番外是否越权回收正传伏笔（warning级别）"),
];

/// 返回 fanfic 模式对应的维度配置。
fn get_fanfic_dimension_config(mode: FanficMode) -> FanficDimensionConfig {
    // SEVERITY_MAP: mode → { dim_id → severity }
    // canon: 34 critical, 35 critical, 36 warning, 37 critical
    // au:    34 critical, 35 info,     36 warning, 37 info
    // ooc:   34 info,     35 warning,  36 warning, 37 info
    // cp:    34 warning,  35 warning,  36 critical, 37 info
    let severity_overrides: &'static [(u32, FanficSeverity)] = match mode {
        FanficMode::Canon => &[
            (34, FanficSeverity::Critical),
            (35, FanficSeverity::Critical),
            (36, FanficSeverity::Warning),
            (37, FanficSeverity::Critical),
        ],
        FanficMode::Au => &[
            (34, FanficSeverity::Critical),
            (35, FanficSeverity::Info),
            (36, FanficSeverity::Warning),
            (37, FanficSeverity::Info),
        ],
        FanficMode::Ooc => &[
            (34, FanficSeverity::Info),
            (35, FanficSeverity::Warning),
            (36, FanficSeverity::Warning),
            (37, FanficSeverity::Info),
        ],
        FanficMode::Cp => &[
            (34, FanficSeverity::Warning),
            (35, FanficSeverity::Warning),
            (36, FanficSeverity::Critical),
            (37, FanficSeverity::Info),
        ],
    };

    FanficDimensionConfig {
        active_ids: &[34, 35, 36, 37],
        severity_overrides,
    }
}

/// 构建维度列表（带说明），根据 fanfic_mode 与 has_parent_canon 条件激活维度。
///
/// 输出每行格式：`<id>. <名称>（<说明>）`，无说明时省略括号。
fn build_dimension_list_with_notes(
    fanfic_mode: Option<FanficMode>,
    has_parent_canon: bool,
) -> String {
    // 确定激活的维度 id 集合
    let mut active_ids: Vec<u32> = (1..=27).chain([32, 33].into_iter()).collect();

    // 番外维度：parent_canon 存在且非 fanfic → 激活 28-31
    if has_parent_canon && fanfic_mode.is_none() {
        active_ids.extend([28, 29, 30, 31]);
    }

    // 同人维度：fanfic_mode 存在 → 激活 34-37
    let fanfic_config = fanfic_mode.map(get_fanfic_dimension_config);
    if let Some(cfg) = &fanfic_config {
        active_ids.extend_from_slice(cfg.active_ids);
    }

    active_ids.sort_unstable();

    // 构建 id → note 查找表
    let mut notes_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();

    // 番外维度说明
    if has_parent_canon && fanfic_mode.is_none() {
        for (id, note) in SPINOFF_DIMENSION_NOTES {
            notes_map.insert(*id, (*note).to_string());
        }
    }

    // 同人维度说明 = baseNote + 严重度标签
    if let Some(cfg) = &fanfic_config {
        for (id, severity) in cfg.severity_overrides {
            let base = FANFIC_DIMENSION_BASE_NOTES
                .iter()
                .find(|(bid, _)| bid == id)
                .map(|(_, n)| *n)
                .unwrap_or("");
            notes_map.insert(*id, format!("{} {}", base, severity.label_zh()));
        }

        // OOC 维度 1 的模式特定说明
        match fanfic_mode {
            Some(FanficMode::Ooc) => {
                notes_map.insert(
                    1,
                    "OOC模式下角色可偏离性格底色，此维度仅记录不判定失败。参照 fanfic_canon.md 角色档案评估偏离程度。".to_string(),
                );
            }
            Some(FanficMode::Canon) => {
                notes_map.insert(
                    1,
                    "原作向同人：角色必须严格遵守性格底色。参照 fanfic_canon.md 角色档案中的性格底色和行为模式。".to_string(),
                );
            }
            _ => {}
        }
    }

    DIMENSION_LABELS
        .iter()
        .filter(|(id, _, _)| active_ids.contains(id))
        .map(|(id, zh, _en)| {
            match notes_map.get(id) {
                Some(note) if !note.is_empty() => format!("{}. {}（{}）", id, zh, note),
                _ => format!("{}. {}", id, zh),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ── System Prompt ────────────────────────────────────────────

fn build_system_prompt(
    book: &BookConfig,
    fanfic_mode: Option<FanficMode>,
    has_parent_canon: bool,
) -> String {
    let dim_list = build_dimension_list_with_notes(fanfic_mode, has_parent_canon);

    format!(
        r#"<identity>
You are a strict structural editor for web fiction. You audit completeness and structure only — never prose style.
</identity>

<audit_boundary>
You do not audit prose, typography, or sentence construction — those belong to the Polisher. Any prose-style issue you happen to notice may only be flagged with severity="info" for the Polisher's reference; it must not influence passed/overall_score and must never be marked critical.
</audit_boundary>

<responsibilities>
You audit twelve structural red-flag categories: sluggish/flat openings, vague or reality-detached worldbuilding, contradictory character setups, chaotic POV, main-line drift or stall, weak conflict or missing payoffs, broken pacing or jarring transitions, before/after character inconsistency, thin characters lacking contrast, stiff emotion or abrupt relationships, unbalanced cheat/golden-finger mechanics, and ungrounded settings. You also retain the engineering dimensions: OOC, timeline consistency, information boundary, hook debt, cross-chapter repetition, lexical fatigue, chapter word count, title fatigue, paragraph shape.

A sparse chapter memo is a legitimate state. Breather / aftermath / transition chapters may have a memo containing only goal + a skeletal body — such memos are not flagged as incomplete, and you must not penalize a finished chapter for paragraphs the memo never specified. Judge drift only against what the memo actually committed to.
</responsibilities>

<repair_scope_rules>
Every issue must carry a repair_scope value as a routing hint:
- "local" — wording, paragraph shape, minor repetition, sentence-level small fixes.
- "structural" — main-line drift, timeline break, missing scene or payoff, character-logic collapse, or any problem requiring a scene or whole-chapter rewrite.
- "unknown" — only when you genuinely cannot tell.
</repair_scope_rules>

## Book Information
- Title: {title}
- Target chapter count: {target_chapters} chapters

## Audit Dimensions:
{dim_list}

## Output Format (must be JSON):

{{
  "passed": true/false,
  "overall_score": 0-100,
  "issues": [
    {{
      "severity": "critical|warning|info",
      "repair_scope": "local|structural|unknown",
      "category": "audit dimension name",
      "description": "specific problem description",
      "suggestion": "revision suggestion"
    }}
  ],
  "summary": "one-sentence summary of the audit verdict"
}}

`passed` is false only when at least one critical-level issue exists.

<scoring_calibration>
- 95-100: ready to publish, no noticeable issues.
- 85-94: minor flaws but overall smooth and readable; readers will not be pulled out of the story.
- 75-84: noticeable problems but the story spine is intact; revision needed but not urgent.
- 65-74: multiple problems harming the reading experience; pacing or continuity has fractures.
- < 65: structural problems requiring substantial rewrite.
Score holistically — do not crater the score over a single minor issue.
</scoring_calibration>"#,
        title = book.title,
        target_chapters = book.target_chapters,
        dim_list = dim_list,
    )
}

// ── User Message ─────────────────────────────────────────────

fn build_user_message(
    _book: &BookConfig,
    chapter_number: u32,
    chapter_title: &str,
    chapter_content: &str,
    ctx: &AuditorContext,
    has_parent_canon: bool,
    has_fanfic_canon: bool,
) -> String {
    let prev_block = if ctx.previous_chapter.is_empty() {
        String::new()
    } else {
        format!("\n## Previous Chapter Full Text (for continuity check)\n{}\n", ctx.previous_chapter)
    };

    let memo_block = if ctx.chapter_memo.is_empty() {
        String::new()
    } else {
        format!("\n## Chapter Memo (for memo drift detection)\n{}\n", ctx.chapter_memo)
    };

    // 正传正典参照块：仅当 has_parent_canon 时注入（番外审查专用）
    let parent_canon_block = if has_parent_canon {
        format!("\n## 正传正典参照（番外审查专用）\n{}\n", ctx.parent_canon)
    } else {
        String::new()
    };

    // 同人正典参照块：仅当 has_fanfic_canon 时注入（同人审查专用）
    let fanfic_canon_block = if has_fanfic_canon {
        format!("\n## 同人正典参照（同人审查专用）\n{}\n", ctx.fanfic_canon)
    } else {
        String::new()
    };

    format!(
        r#"Audit Chapter {chapter_number} "{chapter_title}".

## Current State Card
{current_state}

## Hook Pool
{pending_hooks}

## Chapter Summaries (for pacing check)
{chapter_summaries}

## Volume Outline
{volume_map}

## World Setting
{story_frame}

## Rule Card
{book_rules}

## Style Guide
{style_guide}
{memo_block}{prev_block}{parent_canon_block}{fanfic_canon_block}
## Chapter Content to Audit
{chapter_content}"#,
        chapter_number = chapter_number,
        chapter_title = chapter_title,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        volume_map = ctx.volume_map,
        story_frame = ctx.story_frame,
        book_rules = ctx.book_rules,
        style_guide = if ctx.style_guide.is_empty() {
            "(no style guide)"
        } else {
            &ctx.style_guide
        },
        memo_block = memo_block,
        prev_block = prev_block,
        parent_canon_block = parent_canon_block,
        fanfic_canon_block = fanfic_canon_block,
        chapter_content = chapter_content,
    )
}

// ── JSON 解析（多策略） ──────────────────────────────────────

/// 解析审计结果。
/// 多策略 JSON 提取，兼容小模型输出。
pub fn parse_audit_result(content: &str) -> AuditResult {
    // 策略 1: 尝试整个内容作为 JSON
    let trimmed = content.trim();
    if trimmed.starts_with('{') {
        if let Ok(result) = serde_json::from_str::<AuditResult>(trimmed) {
            return result;
        }
    }

    // 策略 2: 提取 ```json ... ``` 代码块
    if let Some(json_str) = extract_json_block(content) {
        if let Ok(result) = serde_json::from_str::<AuditResult>(json_str) {
            return result;
        }
    }

    // 策略 3: 提取第一个平衡的 JSON 对象
    if let Some(json_str) = extract_balanced_json(content) {
        if let Ok(result) = serde_json::from_str::<AuditResult>(&json_str) {
            return result;
        }
    }

    // 策略 4: 正则提取关键字段（最后兜底）
    if let Some(result) = extract_fields_fallback(content) {
        return result;
    }

    // 全部失败：返回 parse_failed 标记
    AuditResult {
        passed: false,
        overall_score: None,
        issues: Vec::new(),
        summary: format!("审计结果解析失败，原始内容: {}", &content[..content.len().min(200)]),
        parse_failed: true,
    }
}

/// 从 ```json ... ``` 代码块中提取 JSON
fn extract_json_block(content: &str) -> Option<&str> {
    let start_marker = "```json";
    let start = content.find(start_marker)?;
    let json_start = start + start_marker.len();
    let end = content[json_start..].find("```")?;
    Some(content[json_start..json_start + end].trim())
}

/// 提取第一个平衡的 JSON 对象（不贪婪）
fn extract_balanced_json(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let bytes = content.as_bytes();

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        let c = b as char;
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(content[start..=i].to_string());
            }
        }
    }
    None
}

/// 正则兜底：提取 passed / summary / issues
fn extract_fields_fallback(content: &str) -> Option<AuditResult> {
    let pos = content.find(r#""passed""#)?;
    let after = &content[pos + 9..];
    let passed = after.trim_start_matches([':', ' ']).starts_with("true");

    let summary = extract_json_string_field(content, "summary").unwrap_or_default();

    Some(AuditResult {
        passed,
        overall_score: None,
        issues: Vec::new(),
        summary,
        parse_failed: false,
    })
}

/// 提取 JSON 字符串字段值（简化版）
fn extract_json_string_field(content: &str, field: &str) -> Option<String> {
    let pattern = format!(r#""{}""#, field);
    let start = content.find(&pattern)?;
    let after = &content[start + pattern.len()..];
    let colon = after.find(':')?;
    let after_colon = &after[colon + 1..];
    let quote_start = after_colon.find('"')?;
    let after_quote = &after_colon[quote_start + 1..];
    let quote_end = after_quote.find('"')?;
    Some(after_quote[..quote_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pure_json() {
        let json = r#"{"passed": true, "overall_score": 88, "issues": [], "summary": "通过"}"#;
        let result = parse_audit_result(json);
        assert!(result.passed);
        assert_eq!(result.overall_score, Some(88.0));
        assert_eq!(result.summary, "通过");
    }

    #[test]
    fn parses_json_code_block() {
        let content = r#"审计结果如下：
```json
{"passed": false, "overall_score": 45, "issues": [{"severity":"critical","repair_scope":"structural","category":"OOC检查","description":"主角人设崩","suggestion":"重写"}], "summary": "人设崩坏"}
```
"#;
        let result = parse_audit_result(content);
        assert!(!result.passed);
        assert_eq!(result.overall_score, Some(45.0));
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].severity, IssueSeverity::Critical);
        assert_eq!(result.issues[0].repair_scope, Some(RepairScope::Structural));
    }

    #[test]
    fn handles_parse_failure() {
        let content = "这不是 JSON，只是普通文本";
        let result = parse_audit_result(content);
        assert!(result.parse_failed);
        assert!(!result.passed);
    }

    #[test]
    fn parses_balanced_json_with_nested() {
        let content = r#"前文
{
  "passed": true,
  "issues": [
    {"severity": "info", "category": "节奏检查", "description": "可优化", "suggestion": "微调"}
  ],
  "summary": "通过"
}
后文"#;
        let result = parse_audit_result(content);
        assert!(result.passed);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].severity, IssueSeverity::Info);
    }

    #[test]
    fn dimension_list_default_has_29_entries() {
        // 默认场景：1-27 + 32-33 = 29 维度
        let list = build_dimension_list_with_notes(None, false);
        let count = list.lines().count();
        assert_eq!(count, 29);
    }

    #[test]
    fn dimension_list_with_parent_canon_adds_spinoff_dims() {
        // parent_canon 存在且非 fanfic → 1-27 + 28-31 + 32-33 = 33 维度
        let list = build_dimension_list_with_notes(None, true);
        let count = list.lines().count();
        assert_eq!(count, 33);
        assert!(list.contains("28. 正传事件冲突"));
        assert!(list.contains("31. 番外伏笔隔离"));
    }

    #[test]
    fn dimension_list_with_fanfic_mode_adds_fanfic_dims() {
        // fanfic_mode 存在 → 1-27 + 32-33 + 34-37 = 33 维度
        let list = build_dimension_list_with_notes(Some(FanficMode::Canon), false);
        let count = list.lines().count();
        assert_eq!(count, 33);
        assert!(list.contains("34. 角色还原度"));
        assert!(list.contains("37. 正典事件一致性"));
        // canon 模式下 34/35/37 应为严格检查
        assert!(list.contains("34. 角色还原度（检查角色的语癖、说话风格、行为模式是否与 fanfic_canon.md 角色档案一致。偏离必须有情境驱动。 （严格检查））"));
    }

    #[test]
    fn dimension_list_fanfic_ooc_mode_relaxes_ooc_dim() {
        let list = build_dimension_list_with_notes(Some(FanficMode::Ooc), false);
        // 第一行应是 OOC 检查，且带 OOC 模式说明
        let first_line = list.lines().next().unwrap();
        assert!(first_line.starts_with("1. OOC检查"));
        assert!(first_line.contains("OOC模式下角色可偏离性格底色"));
    }

    #[test]
    fn dimension_list_fanfic_overrides_parent_canon() {
        // fanfic_mode + parent_canon 同时存在时：fanfic 优先，不激活番外维度
        let list = build_dimension_list_with_notes(Some(FanficMode::Au), true);
        let count = list.lines().count();
        // 1-27 + 32-33 + 34-37 = 33（不含 28-31）
        assert_eq!(count, 33);
        assert!(!list.contains("28. 正传事件冲突"));
        assert!(list.contains("34. 角色还原度"));
    }
}
