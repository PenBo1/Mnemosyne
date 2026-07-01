use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::features::story::{AuditResult, AuditIssue, AuditSeverity};
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::utils::hook_ledger_validator;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::auditor_prompts;
use super::agent_identity::AgentIdentity;
use super::audit_dimensions::{
    AuditDimensionContext, FanficMode, build_dimension_list, parse_repair_scope,
};
use super::genre_profile::read_genre_profile;

pub struct ContinuityAuditor;

impl Default for ContinuityAuditor {
    fn default() -> Self { Self }
}
impl ContinuityAuditor {
    pub fn new() -> Self { Self }

    /// Audit a chapter across the dynamically-composed dimension set.
    ///
    /// 维度组合由 `build_audit_dimension_context` 根据 book_dir 中的文件决定；
    /// LLM 输出通过 `parse_audit_result` 做 4 级降级解析，解析失败时
    /// `parse_failed=true`，调用方不得据此自动修稿。
    pub async fn audit_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        data_dir: &DataDir,
    ) -> Result<AuditResult, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let chapter_content = read_chapter_content(book_dir, chapter_number)?;

        let dim_ctx = build_audit_dimension_context(book_dir, &language);
        let dimensions = build_dimension_list(&dim_ctx, &language);

        let identity = AgentIdentity::load(data_dir, "auditor");
        let task_query = format!("audit chapter {} for continuity issues", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = auditor_prompts::build_system_prompt(&language, &dimensions, Some(&identity_prefix));
        let user = auditor_prompts::build_user_message(
            chapter_number,
            &chapter_content,
            book_dir,
            &language,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let mut result = parse_audit_result(&response.content)?;

        // Hook 账落实硬规则：planner 在 memo 的 `## 本章 hook 账` 段落声明
        // advance/resolve，validator 检查正文是否有对应落点 + 揭 1 埋 1 规则。
        // 失败时只记 warning 不影响 LLM 评分，避免与 LLM 审计维度重复计分。
        if let Some(memo_body) = read_planner_memo(book_dir, chapter_number) {
            let violations = hook_ledger_validator::validate_hook_ledger(&memo_body, &chapter_content);
            for v in violations {
                result.issues.push(AuditIssue {
                    severity: v.severity,
                    category: v.category,
                    description: v.description,
                    suggestion: v.suggestion,
                    repair_scope: None,
                });
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl BaseAgent for ContinuityAuditor {
    fn role(&self) -> AgentRole {
        AgentRole::Auditor
    }

    fn name(&self) -> &str {
        "continuity-auditor"
    }
}

/// 根据 book_dir 中的文件构建维度组合上下文。
///
/// S7.4: 引入题材规则 profile（GenreProfile）作为题材级配置源真相。
///
/// 加载顺序与覆盖规则：
/// 1. `parent_canon.md` 存在 → `has_parent_canon = true`（激活番外维度 28-31）
/// 2. 从 `book.json` 读取 `genre` 字段 → `read_genre_profile(book_dir, genre_id)`
///    - 加载成功：填充 `genre_dimensions` / `era_research` / `fatigue_words` / `satisfaction_types`
///    - 加载失败（book.json 缺失 / genre 未知 / profile 解析失败）：留空，使用书级 fallback
/// 3. 从 `story/book_rules.md` 读取书级补充配置：
///    - `fanfic_mode:` → `fanfic_mode`（题材 profile 无此字段，只在书级设置）
///    - `era_research: true` → 强制启用（书级可加严，不可放宽）
///    - `fatigue_words:` → 追加到题材列表后（书级补充）
///    - `satisfaction_types:` → 追加到题材列表后
fn build_audit_dimension_context(book_dir: &std::path::Path, _language: &str) -> AuditDimensionContext {
    let mut ctx = AuditDimensionContext::default();

    // 1. parent_canon.md 存在 → 启用番外维度 28-31
    if book_dir.join("story").join("parent_canon.md").exists() {
        ctx.has_parent_canon = true;
    }

    // 2. 从题材规则 profile 加载题材级基线
    if let Some(genre_id) = read_book_genre(book_dir) {
        if let Ok(parsed) = read_genre_profile(book_dir, &genre_id) {
            ctx.genre_dimensions = parsed.profile.audit_dimensions.clone();
            ctx.era_research = parsed.profile.era_research;
            ctx.fatigue_words = parsed.profile.fatigue_words.clone();
            ctx.satisfaction_types = parsed.profile.satisfaction_types.clone();
        }
    }

    // 3. 从 book_rules.md 读取书级补充（覆盖/追加）
    if let Ok(rules) = std::fs::read_to_string(book_dir.join("story").join("book_rules.md")) {
        for line in rules.lines() {
            let trimmed = line.trim();
            if let Some(val) = trimmed.strip_prefix("fanfic_mode:") {
                ctx.fanfic_mode = parse_fanfic_mode(val.trim());
            } else if let Some(val) = trimmed.strip_prefix("era_research:") {
                // 书级 era_research: true 可加严（题材 false → 书级 true）
                // 但不可反向放宽（题材 true → 书级 false 无效）
                if val.trim().eq_ignore_ascii_case("true") {
                    ctx.era_research = true;
                }
            } else if let Some(val) = trimmed.strip_prefix("fatigue_words:") {
                // 书级 fatigue_words 追加到题材列表后
                ctx.fatigue_words.extend(parse_csv_strings(val.trim()));
            } else if let Some(val) = trimmed.strip_prefix("satisfaction_types:") {
                // 书级 satisfaction_types 追加到题材列表后
                ctx.satisfaction_types.extend(parse_csv_strings(val.trim()));
            }
        }
    }

    ctx
}

/// 从 `{book_dir}/book.json` 读取 `genre` 字段。
///
/// 只解析 `genre` 单字段（避免完整 BookConfig 反序列化的字段约束），
/// 文件缺失或字段缺失返回 None（调用方走 other.md fallback）。
fn read_book_genre(book_dir: &std::path::Path) -> Option<String> {
    let book_json_path = book_dir.join("book.json");
    let content = std::fs::read_to_string(&book_json_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value.get("genre")?.as_str().map(|s| s.to_string())
}

fn parse_fanfic_mode(value: &str) -> Option<FanficMode> {
    match value.trim().to_lowercase().as_str() {
        "canon" => Some(FanficMode::Canon),
        "ooc" => Some(FanficMode::Ooc),
        "au" => Some(FanficMode::AU),
        _ => None,
    }
}

fn parse_csv_strings(value: &str) -> Vec<String> {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed.split([',', '、']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// 解析审计 LLM 输出为 AuditResult。
///
/// 4 级降级（对齐 inkos `parseAuditResult`）：
/// 1. 直接 `serde_json::from_str`（最快路径，输出规范时命中）
/// 2. `extract_balanced_json`：从首个 `{` 开始平衡括号提取子串
/// 3. `extract_json_code_block`：从 ```json ... ``` 代码块提取
/// 4. 正则单字段兜底：尽力提取 passed/score/issues 字段
/// 全部失败时返回 `parse_failed=true` 的空结果，调用方不得据此自动修稿。
fn parse_audit_result(content: &str) -> Result<AuditResult, AppError> {
    // Level 1: 直接解析
    if let Some(result) = try_parse_audit_json(content) {
        return Ok(result);
    }

    // Level 2: 平衡括号提取
    if let Some(extracted) = extract_balanced_json(content) {
        if let Some(result) = try_parse_audit_json(&extracted) {
            return Ok(result);
        }
    }

    // Level 3: ```json 代码块提取
    if let Some(extracted) = extract_json_code_block(content) {
        if let Some(result) = try_parse_audit_json(&extracted) {
            return Ok(result);
        }
    }

    // Level 4: 正则单字段兜底（只尽力抢救 passed/score，issues 留空）
    if let Some(result) = try_parse_single_fields(content) {
        return Ok(result);
    }

    // 全部失败：parse_failed=true，调用方不得据此自动修稿
    Ok(AuditResult {
        passed: false,
        score: 0.0,
        issues: Vec::new(),
        summary: content.to_string(),
        parse_failed: true,
    })
}

/// 把 JSON 文本解析为 AuditResult。失败时返回 None（让调用方继续降级）。
fn try_parse_audit_json(json_str: &str) -> Option<AuditResult> {
    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;

    let passed = json.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
    let score = json.get("overall_score")
        .or_else(|| json.get("score"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let summary = json.get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let issues = json.get("issues")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|item| parse_audit_issue(item)).collect()
        })
        .unwrap_or_default();

    Some(AuditResult {
        passed,
        score,
        issues,
        summary,
        parse_failed: false,
    })
}

/// 解析单个 issue 对象。字段缺失或类型错误时返回 None（跳过该 issue）。
fn parse_audit_issue(item: &serde_json::Value) -> Option<AuditIssue> {
    let severity = match item.get("severity")?.as_str()?.to_lowercase().as_str() {
        "critical" => AuditSeverity::Critical,
        "warning" => AuditSeverity::Warning,
        _ => AuditSeverity::Info,
    };
    let repair_scope = item.get("repair_scope")
        .and_then(|v| v.as_str())
        .and_then(parse_repair_scope);

    Some(AuditIssue {
        severity,
        category: item.get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        description: item.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        suggestion: item.get("suggestion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        repair_scope,
    })
}

/// 从文本中提取首个平衡的 JSON 对象（{ ... }）。
///
/// 从第一个 `{` 开始扫描，遇到 `{` 计数+1，`}` 计数-1；
/// 计数归零时返回对应的子串。字符串字面量内的括号被忽略（避免 `"{"` 干扰）。
fn extract_balanced_json(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let bytes = content.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        let c = b as char;
        if in_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(content[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// 从文本中提取 ```json ... ``` 代码块内容。
///
/// 支持无 fence、```json、``` 三种格式。返回代码块内部文本（不含 fence）。
fn extract_json_code_block(content: &str) -> Option<String> {
    let start_marker = content.find("```json")?;
    let after_marker = start_marker + "```json".len();
    let end_marker = content[after_marker..].find("```")?;
    Some(content[after_marker..after_marker + end_marker].trim().to_string())
}

/// 单字段兜底：用简单扫描尽力提取 passed / overall_score 字段。
///
/// 当 LLM 输出严重畸形但仍含 `"passed": true` / `"overall_score": 88` 之类片段时，
/// 抢救出这两个核心字段；issues 留空（无法可靠解析）。parse_failed=false
/// 表示「至少抢救到核心评分」，调用方可据此决定是否跳过修订。
fn try_parse_single_fields(content: &str) -> Option<AuditResult> {
    let passed = scan_bool_field(content, "passed")?;
    let score = scan_number_field(content, "overall_score")
        .or_else(|| scan_number_field(content, "score"))
        .unwrap_or(0.0);

    Some(AuditResult {
        passed,
        score,
        issues: Vec::new(),
        summary: content.to_string(),
        // 抢救到核心字段算解析成功；issues 缺失不阻塞修订决策
        parse_failed: false,
    })
}

/// 扫描 `"field": true|false` 形式的布尔字段。
fn scan_bool_field(content: &str, field: &str) -> Option<bool> {
    let lower = content.to_lowercase();
    let pattern = format!("\"{}\"", field.to_lowercase());
    let idx = lower.find(&pattern)?;
    let after = &content[idx + pattern.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    if after.starts_with("true") {
        Some(true)
    } else if after.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// 扫描 `"field": <number>` 形式的数值字段。
fn scan_number_field(content: &str, field: &str) -> Option<f64> {
    let lower = content.to_lowercase();
    let pattern = format!("\"{}\"", field.to_lowercase());
    let idx = lower.find(&pattern)?;
    let after = &content[idx + pattern.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();

    // 收集数字字符（含小数点）
    let num_str: String = after.chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_str.parse::<f64>().ok()
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
}

fn read_chapter_content(book_dir: &std::path::Path, chapter_number: u32) -> Result<String, AppError> {
    let chapters_dir = book_dir.join("chapters");
    let prefix = format!("{:04}_", chapter_number);

    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let content = std::fs::read_to_string(entry.path())?;
                return Ok(content);
            }
        }
    }

    Err(AppError::not_found(format!("Chapter {} not found", chapter_number)))
}

/// 读取 planner 为某章生成的 memo（含 `## 本章 hook 账` 段落）。
///
/// Planner 把 intent + memo 渲染为 markdown 保存到
/// `<book_dir>/story/runtime/chapter_XXXX_intent.md`。
/// 文件不存在时返回 None（跳过 hook 账验证，不阻塞审计流程）。
fn read_planner_memo(book_dir: &std::path::Path, chapter_number: u32) -> Option<String> {
    let intent_path = book_dir
        .join("story")
        .join("runtime")
        .join(format!("chapter_{:04}_intent.md", chapter_number));
    std::fs::read_to_string(&intent_path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::story::RepairScope;

    // ── try_parse_audit_json ──────────────────────────────────

    #[test]
    fn test_parse_audit_result_level1_direct_json() {
        let raw = r#"{"passed": true, "overall_score": 88, "issues": [], "summary": "ok"}"#;
        let result = parse_audit_result(raw).unwrap();
        assert!(result.passed);
        assert_eq!(result.score, 88.0);
        assert!(result.issues.is_empty());
        assert!(!result.parse_failed);
    }

    #[test]
    fn test_parse_audit_result_with_repair_scope() {
        let raw = r#"{
            "passed": false,
            "overall_score": 60,
            "issues": [
                {"severity":"critical","repair_scope":"structural","category":"OOC","description":"x","suggestion":"y"},
                {"severity":"warning","repair_scope":"local","category":"套话密度","description":"a","suggestion":"b"},
                {"severity":"info","repair_scope":"unknown","category":"misc","description":"c","suggestion":"d"}
            ],
            "summary": "bad"
        }"#;
        let result = parse_audit_result(raw).unwrap();
        assert!(!result.passed);
        assert_eq!(result.score, 60.0);
        assert_eq!(result.issues.len(), 3);
        assert_eq!(result.issues[0].repair_scope, Some(RepairScope::Structural));
        assert_eq!(result.issues[1].repair_scope, Some(RepairScope::Local));
        assert_eq!(result.issues[2].repair_scope, Some(RepairScope::Unknown));
    }

    #[test]
    fn test_parse_audit_result_invalid_repair_scope_defaults_none() {
        let raw = r#"{"passed": true, "overall_score": 90, "issues": [
            {"severity":"warning","repair_scope":"bogus","category":"x","description":"","suggestion":""}
        ], "summary": ""}"#;
        let result = parse_audit_result(raw).unwrap();
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].repair_scope, None);
    }

    #[test]
    fn test_parse_audit_result_score_alias() {
        // 旧字段 `score` 应作为 `overall_score` 的回退
        let raw = r#"{"passed": true, "score": 75, "issues": [], "summary": ""}"#;
        let result = parse_audit_result(raw).unwrap();
        assert_eq!(result.score, 75.0);
    }

    // ── Level 2: extract_balanced_json ────────────────────────

    #[test]
    fn test_parse_audit_result_level2_balanced_json_with_prefix() {
        let raw = "审计结果如下：\n{\"passed\": true, \"overall_score\": 90, \"issues\": [], \"summary\": \"\"}\n以上。";
        let result = parse_audit_result(raw).unwrap();
        assert!(result.passed);
        assert_eq!(result.score, 90.0);
        assert!(!result.parse_failed);
    }

    #[test]
    fn test_parse_audit_result_level2_balanced_with_nested_braces() {
        // 字符串字面量内的 `}` 不应破坏平衡
        let raw = r#"前缀文字 {"passed": false, "overall_score": 50, "issues": [{"severity":"critical","category":"x","description":"has } char","suggestion":""}], "summary": ""} 后缀"#;
        let result = parse_audit_result(raw).unwrap();
        assert!(!result.passed);
        assert_eq!(result.score, 50.0);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].description, "has } char");
    }

    #[test]
    fn test_extract_balanced_json_simple() {
        let s = "prefix {\"a\":1} suffix";
        let extracted = extract_balanced_json(s).unwrap();
        assert_eq!(extracted, "{\"a\":1}");
    }

    #[test]
    fn test_extract_balanced_json_nested() {
        let s = "x {\"a\": {\"b\": 2}, \"c\": 3} y";
        let extracted = extract_balanced_json(s).unwrap();
        assert_eq!(extracted, "{\"a\": {\"b\": 2}, \"c\": 3}");
    }

    #[test]
    fn test_extract_balanced_json_no_brace() {
        assert_eq!(extract_balanced_json("no braces here"), None);
    }

    #[test]
    fn test_extract_balanced_json_unbalanced() {
        // 缺少闭合括号 → None
        assert_eq!(extract_balanced_json("{\"a\": 1"), None);
    }

    // ── Level 3: extract_json_code_block ──────────────────────

    #[test]
    fn test_parse_audit_result_level3_code_block() {
        let raw = "分析完成。结果如下：\n```json\n{\"passed\": true, \"overall_score\": 92, \"issues\": [], \"summary\": \"\"}\n```\n请查阅。";
        let result = parse_audit_result(raw).unwrap();
        assert!(result.passed);
        assert_eq!(result.score, 92.0);
        assert!(!result.parse_failed);
    }

    #[test]
    fn test_extract_json_code_block_simple() {
        let s = "```json\n{\"a\":1}\n```";
        let extracted = extract_json_code_block(s).unwrap();
        assert_eq!(extracted, "{\"a\":1}");
    }

    #[test]
    fn test_extract_json_code_block_no_block() {
        assert_eq!(extract_json_code_block("no code block"), None);
    }

    // ── Level 4: try_parse_single_fields ──────────────────────

    #[test]
    fn test_parse_audit_result_level4_single_fields() {
        // 严重畸形但保留了 passed/score 字段
        let raw = " auditing... \"passed\": true, some garbage, \"overall_score\": 88 ... end";
        let result = parse_audit_result(raw).unwrap();
        assert!(result.passed);
        assert_eq!(result.score, 88.0);
        assert!(result.issues.is_empty());
        // Level 4 抢救成功，parse_failed=false（调用方可跳过修订）
        assert!(!result.parse_failed);
    }

    #[test]
    fn test_scan_bool_field_true() {
        let s = "garbage \"passed\": true end";
        assert_eq!(scan_bool_field(s, "passed"), Some(true));
    }

    #[test]
    fn test_scan_bool_field_false_with_spaces() {
        let s = "x \"passed\"  :  false y";
        assert_eq!(scan_bool_field(s, "passed"), Some(false));
    }

    #[test]
    fn test_scan_number_field() {
        let s = "x \"overall_score\": 88.5 y";
        assert_eq!(scan_number_field(s, "overall_score"), Some(88.5));
    }

    #[test]
    fn test_scan_number_field_negative() {
        let s = "\"score\": -10";
        assert_eq!(scan_number_field(s, "score"), Some(-10.0));
    }

    // ── Level 5: parse_failed fallback ────────────────────────

    #[test]
    fn test_parse_audit_result_fully_garbage_returns_parse_failed() {
        let raw = "完全无法解析的文本，没有任何 JSON 字段";
        let result = parse_audit_result(raw).unwrap();
        assert!(result.parse_failed);
        assert!(!result.passed);
        assert_eq!(result.score, 0.0);
        assert!(result.issues.is_empty());
        assert_eq!(result.summary, raw);
    }

    #[test]
    fn test_parse_audit_result_missing_passed_defaults_false_not_failed() {
        // 有效 JSON 但缺 passed 字段 → Level 1 解析成功，passed 默认 false（安全默认），parse_failed=false
        // 这是安全行为：不自动判定通过，但仍允许使用 score/issues
        let raw = "{\"summary\": \"no passed field\"}";
        let result = parse_audit_result(raw).unwrap();
        assert!(!result.parse_failed, "valid JSON should not be marked parse_failed");
        assert!(!result.passed, "missing passed field should default to false");
    }

    // ── build_audit_dimension_context ────────────────────────

    #[test]
    fn test_build_audit_dimension_context_default_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert!(!ctx.has_parent_canon);
        assert!(ctx.fanfic_mode.is_none());
        assert!(!ctx.era_research);
        assert!(ctx.fatigue_words.is_empty());
        assert!(ctx.satisfaction_types.is_empty());
    }

    #[test]
    fn test_build_audit_dimension_context_parent_canon_file() {
        let tmp = tempfile::tempdir().unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("parent_canon.md"), "# parent canon").unwrap();
        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert!(ctx.has_parent_canon);
    }

    #[test]
    fn test_build_audit_dimension_context_book_rules_fanfic_canon() {
        let tmp = tempfile::tempdir().unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("book_rules.md"),
            "# Book Rules\nfanfic_mode: canon\nera_research: true\nfatigue_words: 果然, 似乎\nsatisfaction_types: 打脸、逆袭\n"
        ).unwrap();
        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.fanfic_mode, Some(FanficMode::Canon));
        assert!(ctx.era_research);
        assert_eq!(ctx.fatigue_words, vec!["果然".to_string(), "似乎".to_string()]);
        assert_eq!(ctx.satisfaction_types, vec!["打脸".to_string(), "逆袭".to_string()]);
    }

    #[test]
    fn test_build_audit_dimension_context_book_rules_fanfic_au() {
        let tmp = tempfile::tempdir().unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("book_rules.md"), "fanfic_mode: au\n").unwrap();
        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.fanfic_mode, Some(FanficMode::AU));
    }

    #[test]
    fn test_build_audit_dimension_context_invalid_fanfic_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("book_rules.md"), "fanfic_mode: bogus\n").unwrap();
        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert!(ctx.fanfic_mode.is_none());
    }

    #[test]
    fn test_parse_csv_strings_empty() {
        assert!(parse_csv_strings("").is_empty());
        assert!(parse_csv_strings("\"\"").is_empty());
    }

    #[test]
    fn test_parse_csv_strings_mixed_separators() {
        let v = parse_csv_strings("打脸、逆袭, 升级");
        assert_eq!(v, vec!["打脸".to_string(), "逆袭".to_string(), "升级".to_string()]);
    }

    // ── End-to-end: AuditDimensionContext → build_dimension_list ──

    #[test]
    fn test_audit_chapter_dimension_composition_with_fanfic() {
        // 当 fanfic_mode 设置时，应启用 34-37 而非 28-31
        let tmp = tempfile::tempdir().unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("parent_canon.md"), "x").unwrap();
        std::fs::write(story_dir.join("book_rules.md"), "fanfic_mode: canon\n").unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();

        assert!(ids.contains(&34), "fanfic mode should activate 34-37");
        assert!(ids.contains(&37));
        assert!(!ids.contains(&28), "fanfic mode should NOT activate 28-31");
    }

    // ── S7.4: read_book_genre ────────────────────────────────

    #[test]
    fn test_read_book_genre_returns_genre_from_book_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan","language":"zh"}"#,
        ).unwrap();
        assert_eq!(read_book_genre(tmp.path()).as_deref(), Some("xuanhuan"));
    }

    #[test]
    fn test_read_book_genre_returns_none_when_book_json_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(read_book_genre(tmp.path()).is_none());
    }

    #[test]
    fn test_read_book_genre_returns_none_when_genre_field_missing() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x"}"#,
        ).unwrap();
        assert!(read_book_genre(tmp.path()).is_none());
    }

    #[test]
    fn test_read_book_genre_returns_none_when_book_json_malformed() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("book.json"), "not json").unwrap();
        assert!(read_book_genre(tmp.path()).is_none());
    }

    // ── S7.4: build_audit_dimension_context 与 genre profile 集成 ──

    #[test]
    fn test_build_ctx_loads_genre_profile_from_book_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan","language":"zh"}"#,
        ).unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");

        // xuanhuan.md 的 auditDimensions 有 21 项
        assert_eq!(ctx.genre_dimensions.len(), 21);
        assert!(ctx.genre_dimensions.contains(&4));
        assert!(ctx.genre_dimensions.contains(&5));
        assert!(!ctx.era_research);
        assert_eq!(ctx.fatigue_words.len(), 12);
        assert!(ctx.fatigue_words.contains(&"冷笑".to_string()));
        assert_eq!(ctx.satisfaction_types.len(), 6);
        assert!(ctx.satisfaction_types.contains(&"打脸".to_string()));
    }

    #[test]
    fn test_build_ctx_unknown_genre_falls_back_to_other() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"totally-unknown"}"#,
        ).unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.genre_dimensions.len(), 18);
        assert!(!ctx.era_research);
    }

    #[test]
    fn test_build_ctx_no_book_json_leaves_genre_dimensions_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert!(ctx.genre_dimensions.is_empty());
        assert!(ctx.fatigue_words.is_empty());
        assert!(ctx.satisfaction_types.is_empty());
        assert!(!ctx.era_research);
    }

    #[test]
    fn test_build_ctx_book_rules_era_research_can_strictify_genre() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"other"}"#,
        ).unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("book_rules.md"), "era_research: true\n").unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert!(ctx.era_research, "书级 era_research: true 应加严题材 false");
    }

    #[test]
    fn test_build_ctx_book_rules_era_research_false_cannot_relax_genre() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"urban"}"#,
        ).unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("book_rules.md"), "era_research: false\n").unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert!(ctx.era_research, "题材 urban 启用 eraResearch，书级 false 不可放宽");
    }

    #[test]
    fn test_build_ctx_book_rules_fatigue_words_appended_to_genre() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan"}"#,
        ).unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(
            story_dir.join("book_rules.md"),
            "fatigue_words: 书级词1, 书级词2\n",
        ).unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.fatigue_words.len(), 14);
        assert!(ctx.fatigue_words.contains(&"书级词1".to_string()));
        assert!(ctx.fatigue_words.contains(&"书级词2".to_string()));
        assert!(ctx.fatigue_words.contains(&"冷笑".to_string()));
    }

    #[test]
    fn test_build_ctx_book_rules_satisfaction_types_appended_to_genre() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan"}"#,
        ).unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(
            story_dir.join("book_rules.md"),
            "satisfaction_types: 额外爽点\n",
        ).unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.satisfaction_types.len(), 7);
        assert!(ctx.satisfaction_types.contains(&"额外爽点".to_string()));
        assert!(ctx.satisfaction_types.contains(&"打脸".to_string()));
    }

    #[test]
    fn test_build_ctx_book_rules_fanfic_mode_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan"}"#,
        ).unwrap();
        let story_dir = tmp.path().join("story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("book_rules.md"), "fanfic_mode: au\n").unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.fanfic_mode, Some(FanficMode::AU));
    }

    #[test]
    fn test_build_ctx_project_level_genre_overrides_builtin() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan"}"#,
        ).unwrap();
        let genres_dir = tmp.path().join("genres");
        std::fs::create_dir_all(&genres_dir).unwrap();
        std::fs::write(
            genres_dir.join("xuanhuan.md"),
            "---\nname: 项目玄幻\nid: xuanhuan\nchapterTypes: [\"x\"]\nfatigueWords: [\"项目词\"]\nnumericalSystem: false\npowerScaling: false\neraResearch: true\npacingRule: \"x\"\nsatisfactionTypes: [\"项目爽点\"]\nauditDimensions: [1,2,3]\n---\nbody\n",
        ).unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        assert_eq!(ctx.genre_dimensions, vec![1, 2, 3]);
        assert!(ctx.era_research);
        assert_eq!(ctx.fatigue_words, vec!["项目词".to_string()]);
        assert_eq!(ctx.satisfaction_types, vec!["项目爽点".to_string()]);
    }

    #[test]
    fn test_build_ctx_dimension_list_uses_genre_profile_dimensions() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"id":"b1","title":"x","genre":"xuanhuan"}"#,
        ).unwrap();

        let ctx = build_audit_dimension_context(tmp.path(), "zh");
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();

        // xuanhuan.md auditDimensions (21) + 永久 32/33 = 23 个
        assert_eq!(ids.len(), 23);
        assert!(ids.contains(&4));
        assert!(ids.contains(&5));
        assert!(ids.contains(&32));
        assert!(ids.contains(&33));
        assert!(!ids.contains(&12));
        assert!(!ids.contains(&28));
    }
}
