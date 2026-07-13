// ContinuityAuditor Agent。
//
// 职责：对写完的章节做 37 维度结构审计，输出 JSON 格式的审计结果。
// 只有 critical 级别问题才判定 passed=false。
//
// prompt 策略：保留 37 个维度标签 + 审稿边界 + JSON 输出格式。
// 精简 fanfic 维度配置、governed context、web search 等高级特性。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::super::types::BookConfig;

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
    let system_prompt = build_system_prompt(book);
    let user_message = build_user_message(book, chapter_number, chapter_title, chapter_content, ctx);

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

fn build_dimension_list() -> String {
    DIMENSION_LABELS
        .iter()
        .map(|(id, zh, _en)| format!("{}. {}", id, zh))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── System Prompt ────────────────────────────────────────────

fn build_system_prompt(book: &BookConfig) -> String {
    let dim_list = build_dimension_list();

    format!(
        r#"你是一位严格的网络小说结构审稿编辑。你只审完成度 + 结构，不审文笔。

## 审稿边界（硬约束）

你不审文笔、不审排版、不审句式——这些归 Polisher。你发现的文笔问题只能以 severity="info" 标注供 Polisher 参考，不计入 passed/overall_score，也绝不可标为 critical。

你审 12 条结构类雷点：开篇拖沓/平淡、世界观模糊脱现实、人设矛盾、视角杂乱、主线偏离/停滞、冲突乏力爽点缺失、节奏失控过渡生硬、人设前后矛盾、人物单薄无反差、情感表达生硬/关系突兀、金手指失衡、设定无落地。同时保留工程维度（OOC、timeline 一致、信息越界、hook-debt、跨章重复、词汇疲劳、章节字数、标题疲劳、段落形状）。

稀疏 memo 是合法状态。喘息章 / 后效章 / 过渡章的 memo 可以只有 goal + 骨架 body——此类 memo 不判 incomplete，也不能因为 memo 没写的段落就扣成稿的分。只按 memo 实际写出来的内容判偏离。

每条 issue 必须给 repair_scope 作为路由提示：
- "local" 表示措辞、段落形状、小重复、句段级小修
- "structural" 表示主线偏离、时间线断裂、场面/回报缺失、人物逻辑崩，或任何需要重写场景/整章的问题
- "unknown" 只有确实无法判断时才写

## 书籍信息
- 标题：{title}
- 目标章数：{target_chapters}章

## 审查维度：
{dim_list}

## 输出格式必须为 JSON：

{{
  "passed": true/false,
  "overall_score": 0-100,
  "issues": [
    {{
      "severity": "critical|warning|info",
      "repair_scope": "local|structural|unknown",
      "category": "审查维度名称",
      "description": "具体问题描述",
      "suggestion": "修改建议"
    }}
  ],
  "summary": "一句话总结审查结论"
}}

只有当存在 critical 级别问题时，passed 才为 false。

overall_score 评分校准：
- 95-100：可直接发布，无明显问题
- 85-94：有小瑕疵但整体流畅可读，读者不会出戏
- 75-84：有明显问题但故事主干完整，需要修但不紧急
- 65-74：多处影响阅读体验的问题，节奏或连续性有断裂
- < 65：结构性问题，需要大幅重写
综合评分，不要因为单一小问题大幅拉低分数。"#,
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
) -> String {
    let prev_block = if ctx.previous_chapter.is_empty() {
        String::new()
    } else {
        format!("\n## 上一章全文（用于衔接检查）\n{}\n", ctx.previous_chapter)
    };

    let memo_block = if ctx.chapter_memo.is_empty() {
        String::new()
    } else {
        format!("\n## 章节备忘（用于 memo 偏离检测）\n{}\n", ctx.chapter_memo)
    };

    format!(
        r#"请审查第 {chapter_number} 章「{chapter_title}」。

## 当前状态卡
{current_state}

## 伏笔池
{pending_hooks}

## 章节摘要（用于节奏检查）
{chapter_summaries}

## 卷纲
{volume_map}

## 世界观设定
{story_frame}

## 规则卡
{book_rules}

## 文风指南
{style_guide}
{memo_block}{prev_block}
## 待审章节内容
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
            "(无文风指南)"
        } else {
            &ctx.style_guide
        },
        memo_block = memo_block,
        prev_block = prev_block,
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
    fn dimension_list_has_37_entries() {
        let list = build_dimension_list();
        let count = list.lines().count();
        assert_eq!(count, 37);
    }
}
