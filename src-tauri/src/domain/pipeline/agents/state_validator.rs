// StateValidator Agent。
//
// 职责：连续性验证器，检查章节结算前后的状态是否自洽。
// 检查 6 类矛盾：状态变化无叙事支撑、缺失状态变化、时间不可能性、
// Hook 异常、追溯性编辑、跨真相键冲突。FAIL 仅用于硬矛盾。
//
// prompt 策略：保留 6 类矛盾检查 + PASS/FAIL + JSON/行式输出。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

/// 单条验证警告
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub category: String,
    pub description: String,
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub warnings: Vec<ValidationWarning>,
}

/// 验证状态连续性。
pub async fn validate_state(
    engine: &AgentEngine,
    chapter_content: &str,
    chapter_number: u32,
    old_state: &str,
    new_state: &str,
    old_hooks: &str,
    new_hooks: &str,
) -> Result<ValidationResult, AppError> {
    let system_prompt = build_system_prompt();
    let user_message = build_user_message(
        chapter_content,
        chapter_number,
        old_state,
        new_state,
        old_hooks,
        new_hooks,
    );

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_validation_result(&response))
}

fn build_system_prompt() -> String {
    r###"<identity>
You are a continuity validator. Your task is to verify that the chapter's state is self-consistent before and after settlement, surfacing six classes of contradiction.
</identity>

## Check Categories

1. State change without narrative support: new_state contains a change, but no corresponding narrative passage can be found in chapter_content.
2. Missing state change: something happened in chapter_content that new_state fails to record.
3. Temporal impossibility: time flowing backwards, a character in two places at once, illogical duration.
4. Hook anomaly: hook status changes contradict the prose (marked progressing without advance, marked resolved without the prose revealing it, etc.).
5. Retroactive edit: new_state tampers with facts already established in old_state (not an incremental update).
6. Cross-truth-key conflict: current_state contradicts pending_hooks.

## Verdict Rules

- PASS: no hard contradictions; minor inconsistencies may remain (recorded as warnings).
- FAIL: a hard contradiction exists (state completely mismatches, a hook vanishes or appears out of nowhere, timeline breaks).

## Output Format (either is acceptable)

### Format A: JSON

{
  "passed": true,
  "warnings": [
    {"category": "category name", "description": "specific description"}
  ]
}

### Format B: line-based

First line: PASS or FAIL
Each subsequent line is one warning (an optional [category] prefix is allowed):
[Hook Anomaly] H007 status flipped from open to resolved, but the prose never reveals it

Emit only the validation result — no other commentary."###
        .to_string()
}

fn build_user_message(
    chapter_content: &str,
    chapter_number: u32,
    old_state: &str,
    new_state: &str,
    old_hooks: &str,
    new_hooks: &str,
) -> String {
    format!(
        r###"Validate the state continuity of Chapter {chapter_number}.

## Chapter Prose
{chapter_content}

## Old State Card (before settlement)
{old_state}

## New State Card (after settlement)
{new_state}

## Old Hook Pool (before settlement)
{old_hooks}

## New Hook Pool (after settlement)
{new_hooks}"###,
        chapter_number = chapter_number,
        chapter_content = chapter_content,
        old_state = old_state,
        new_state = new_state,
        old_hooks = old_hooks,
        new_hooks = new_hooks,
    )
}

/// 解析验证结果：先尝试 JSON，失败则按行解析
fn parse_validation_result(content: &str) -> ValidationResult {
    // 策略 1: 尝试 JSON 解析
    if let Some(result) = try_parse_json(content) {
        return result;
    }

    // 策略 2: 行式解析
    parse_lines(content)
}

/// 尝试 JSON 解析（直接 JSON 或 ```json 代码块）
fn try_parse_json(content: &str) -> Option<ValidationResult> {
    let trimmed = content.trim();

    // 尝试直接解析
    if trimmed.starts_with('{') {
        if let Ok(result) = serde_json::from_str::<JsonValidationResult>(trimmed) {
            return Some(result.into());
        }
    }

    // 尝试提取 ```json ... ``` 代码块
    if let Some(json_str) = extract_json_block(trimmed) {
        if let Ok(result) = serde_json::from_str::<JsonValidationResult>(json_str) {
            return Some(result.into());
        }
    }

    None
}

/// 行式解析：第一行 PASS/FAIL，后续行 warning
fn parse_lines(content: &str) -> ValidationResult {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return ValidationResult {
            passed: true,
            warnings: Vec::new(),
        };
    }

    // 第一行判断 PASS/FAIL
    let first = lines[0].trim();
    let passed = !first.eq_ignore_ascii_case("FAIL");

    // 后续行解析为 warnings
    let warnings = lines[1..]
        .iter()
        .filter_map(|line| parse_warning_line(line))
        .collect();

    ValidationResult { passed, warnings }
}

/// 解析单行 warning（支持 [category] description 前缀格式）
fn parse_warning_line(line: &str) -> Option<ValidationWarning> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 尝试匹配 [category] description
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let category = rest[..end].trim().to_string();
            let description = rest[end + 1..].trim().to_string();
            if !description.is_empty() {
                return Some(ValidationWarning { category, description });
            }
        }
    }

    // 无前缀，整行作为 description
    Some(ValidationWarning {
        category: "未分类".to_string(),
        description: trimmed.to_string(),
    })
}

/// 从 ```json ... ``` 代码块中提取 JSON
fn extract_json_block(content: &str) -> Option<&str> {
    let start_marker = "```json";
    let start = content.find(start_marker)?;
    let json_start = start + start_marker.len();
    let end = content[json_start..].find("```")?;
    Some(content[json_start..json_start + end].trim())
}

// JSON 解析用的中间结构
#[derive(serde::Deserialize)]
struct JsonValidationResult {
    passed: bool,
    #[serde(default)]
    warnings: Vec<JsonValidationWarning>,
}

#[derive(serde::Deserialize)]
struct JsonValidationWarning {
    #[serde(default)]
    category: String,
    #[serde(default)]
    description: String,
}

impl From<JsonValidationResult> for ValidationResult {
    fn from(j: JsonValidationResult) -> Self {
        ValidationResult {
            passed: j.passed,
            warnings: j.warnings.into_iter().map(|w| w.into()).collect(),
        }
    }
}

impl From<JsonValidationWarning> for ValidationWarning {
    fn from(j: JsonValidationWarning) -> Self {
        ValidationWarning {
            category: j.category,
            description: j.description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pass_result() {
        let content = "PASS\n[Hook 异常] H007 状态变化轻微不一致\n[时间不可能性] 时长略短";
        let result = parse_validation_result(content);
        assert!(result.passed);
        assert_eq!(result.warnings.len(), 2);
        assert_eq!(result.warnings[0].category, "Hook 异常");
        assert_eq!(result.warnings[0].description, "H007 状态变化轻微不一致");
        assert_eq!(result.warnings[1].category, "时间不可能性");
    }

    #[test]
    fn parses_fail_with_warnings() {
        let content = "FAIL\n[状态变化无叙事支撑] 主角受伤但正文未提及\n[缺失状态变化] 正文获得物品但状态卡未记录";
        let result = parse_validation_result(content);
        assert!(!result.passed);
        assert_eq!(result.warnings.len(), 2);
        assert_eq!(result.warnings[0].category, "状态变化无叙事支撑");
        assert_eq!(result.warnings[1].category, "缺失状态变化");
    }

    #[test]
    fn parses_json_format() {
        let content = r#"```json
{
  "passed": false,
  "warnings": [
    {"category": "Hook 异常", "description": "H007 未推进却标 resolved"}
  ]
}
```"#;
        let result = parse_validation_result(content);
        assert!(!result.passed);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].category, "Hook 异常");
        assert_eq!(result.warnings[0].description, "H007 未推进却标 resolved");
    }

    #[test]
    fn parses_pure_json_without_code_block() {
        let content = r#"{"passed": true, "warnings": []}"#;
        let result = parse_validation_result(content);
        assert!(result.passed);
        assert_eq!(result.warnings.len(), 0);
    }

    #[test]
    fn parses_line_without_category_prefix() {
        let content = "PASS\n这是一条无分类的警告";
        let result = parse_validation_result(content);
        assert!(result.passed);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].category, "未分类");
        assert_eq!(result.warnings[0].description, "这是一条无分类的警告");
    }

    #[test]
    fn handles_empty_content() {
        let result = parse_validation_result("");
        assert!(result.passed);
        assert_eq!(result.warnings.len(), 0);
    }
}
