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
use crate::shared::utils::json::{extract_json_block, match_braces};

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
    let mut active_ids: Vec<u32> = (1..=27).chain([32, 33]).collect();

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
你是一名网文的结构编辑（structural editor）。你只审计完整性与结构——绝不审计文笔风格。
</identity>

<audit_boundary>
你不审计文笔、排版、句式构造——那些归 Polisher 处理。任何你恰好注意到的文笔风格问题，只能以 severity="info" 标记供 Polisher 参考；它不得影响 passed / overall_score，也绝不可被标为 critical。
</audit_boundary>

<responsibilities>
你审计十二类结构红旗：开局疲软/平坦、世界观虚浮或脱离现实、角色设定自相矛盾、视角混乱、主线漂移或停滞、冲突乏力或缺兑现、节奏断裂或转折突兀、角色前后不一致、人物单薄缺乏反差、情绪僵硬或关系突变、金手指/外挂机制失衡、设定悬浮不接地气。同时保留工程维度：OOC、时间线一致性、信息边界、伏笔账本、跨章重复、词汇疲劳、章节字数、标题疲劳、段落形态。

memo 简略是合法状态。喘息章 / 余波章 / 过渡章的 memo 可能只有目标 + 骨架正文——这种 memo 不算"不完整"，你不能因为 memo 没要求的段落去惩罚已完成章节。只在 memo 真正承诺的范围内判定 drift。
</responsibilities>

<repair_scope_rules>
每一条 issue 必须携带 repair_scope 作为路由提示：
- "local" —— 措辞、段落形态、轻微重复、句子级小修。
- "structural" —— 主线漂移、时间线断裂、缺场景或缺兑现、角色逻辑崩坏，或任何需要场景或整章重写的问题。
- "unknown" —— 仅当你真的判断不了时使用。
</repair_scope_rules>

## Book Information
- 书名：{title}
- 目标章数：{target_chapters} 章

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
      "category": "审计维度名称",
      "description": "具体问题描述",
      "suggestion": "修订建议"
    }}
  ],
  "summary": "一句话总结审计结论"
}}

`passed` 仅在至少存在一条 critical 级别问题时才为 false。

<scoring_calibration>
- 95-100：可发布，无明显问题。
- 85-94：有小瑕疵但整体流畅可读；读者不会被踢出故事。
- 75-84：有明显问题但故事骨架完整；需要修订但不紧急。
- 65-74：多个问题损害阅读体验；节奏或连续性出现裂缝。
- < 65：结构问题严重，需要大幅重写。
整体打分——不要因为单个 minor 问题就把分数砸到底。
</scoring_calibration>

<safety>
- NEVER 把文笔风格问题标为 critical——文笔问题归 Polisher，最多标 info。
- NEVER 因为 memo 简略就判 failed——喘息章/过渡章允许 memo 只有目标+骨架。
- NEVER 编造原文中不存在的矛盾——所有 issue 必须能在原文中找到具体证据。
</safety>

<examples>
✅ Good（结构问题 + 可定位 + 路由提示）：
- {{ "severity":"critical", "repair_scope":"structural", "category":"时间线检查", "description":"第 8 章末尾是清晨，本章开篇却写'夕阳西下'且未交代时间跳跃", "suggestion":"补充时间过渡或调整场景时间" }}

❌ Bad（文笔问题标 critical / 缺 repair_scope）：
- {{ "severity":"critical", "category":"文风检查", "description":"形容词过多" }}（文笔问题不可标 critical；缺 repair_scope）
- {{ "severity":"warning", "category":"节奏检查", "description":"感觉有点慢" }}（"感觉"无原文证据，不可主观臆断）
</examples>

<verification>
完成审计后请自检：
1. 输出是否为合法 JSON（无 Markdown 包裹、无自然语言注释）？
2. 每一条 issue 是否都携带 severity 与 repair_scope 两个字段？
3. 是否有任意一条文笔类问题被标为 critical？若有，降级为 info。
4. passed 是否仅在存在 critical 问题时才为 false？
5. overall_score 是否综合打分（未因单个 minor 问题砸底）？
若任一项不通过，重新输出。
</verification>"#,
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
        r#"请审计第 {chapter_number} 章 "{chapter_title}"。

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
            "（无风格指南）"
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
    if let Some(json_str) = match_braces(content) {
        if let Ok(result) = serde_json::from_str::<AuditResult>(json_str) {
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

// extract_json_block / extract_balanced_json 已收口到 crate::shared::utils::json
// （extract_json_block / match_braces），见上方 use 声明。

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
