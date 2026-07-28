//! ═══════════════════════════════════════════════════════════════════════════
//! StateValidator Agent - 状态验证代理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：连续性验证器，检查章节结算前后的状态是否自洽。
//! 检查 6 类矛盾：状态变化无叙事支撑、缺失状态变化、时间不可能性、
//! Hook 异常、追溯性编辑、跨真相键冲突。FAIL 仅用于硬矛盾。

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

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
    let start = Instant::now();
    tracing::info!(function = "validate_state", chapter_number, "入口");

    let system_prompt = build_system_prompt();
    let user_message = build_user_message(
        chapter_content,
        chapter_number,
        old_state,
        new_state,
        old_hooks,
        new_hooks,
    );

    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let result = parse_validation_result(&response);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "validate_state", chapter_number, duration_ms, passed = result.passed, warning_count = result.warnings.len(), "出口");
            Ok(result)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "validate_state", chapter_number, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

fn build_system_prompt() -> String {
    r###"<identity>
你是一名连续性验证器。你的任务是验证章节在结算前后的状态是否自洽，发现六类矛盾。
</identity>

## 检查类别

1. 状态变化无叙事支撑：new_state 中存在某项变化，但在 chapter_content 中找不到对应的叙事段落。
2. 缺失状态变化：chapter_content 中发生了某事，但 new_state 未予记录。
3. 时间不可能性：时间倒流、同一角色同时出现在两处、时长不合逻辑等。
4. Hook 异常：钩子状态变化与正文相矛盾（如标记为推进但正文未推进、标记为已解决但正文未揭示等）。
5. 追溯性编辑：new_state 篡改了 old_state 中已确立的事实（非增量更新）。
6. 跨真相键冲突：current_state 与 pending_hooks 相互矛盾。

## 判定规则

- PASS：不存在硬矛盾；可能残留轻微不一致（记录为 warning）。
- FAIL：存在硬矛盾（状态完全错配、钩子凭空消失或出现、时间线断裂等）。

<safety>
- 绝不（NEVER）将"风格偏好"或"可读性问题"判定为 FAIL：FAIL 仅用于硬矛盾。
- 绝不（NEVER）编造正文里不存在的剧情来合理化状态卡的变化；找不到叙事支撑就如实标记为 warning。
- 绝不（NEVER）在输出中附加任何评审建议、修改方案或主观评论，只输出验证结果本身。
</safety>

<examples>
正确（JSON 格式）：
{
  "passed": false,
  "warnings": [
    {"category": "Hook 异常", "description": "H007 未推进却标记为 resolved"}
  ]
}

正确（行式格式）：
FAIL
[状态变化无叙事支撑] 主角受伤但正文未提及任何战斗
[缺失状态变化] 正文获得"寒霜剑"但状态卡未记录

错误：
PASS（判定正确）+ "建议作者补充……"
（附加了评审建议，违反"只输出验证结果"）
</examples>

<verification>
完成后自检：
1. 是否只输出了验证结果，无任何额外评论或修改建议。
2. PASS/FAIL 判定是否仅基于六类硬矛盾，而非主观偏好。
3. 每条 warning 是否包含类别与具体描述，且能对应到正文或状态卡的具体位置。
4. 若使用 JSON 格式，字段名是否为 `passed` 与 `warnings`（不可改名）。
</verification>

## 输出格式（两种任选其一）

### 格式 A：JSON

{
  "passed": true,
  "warnings": [
    {"category": "类别名", "description": "具体描述"}
  ]
}

### 格式 B：行式

第一行：PASS 或 FAIL
其后每一行一条 warning（可加可选的 [类别] 前缀）：
[Hook 异常] H007 状态从 open 翻转为 resolved，但正文从未揭示其推进

只输出验证结果 —— 不要附加任何其他评论。"###
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
        r###"请验证第 {chapter_number} 章的状态连续性。

## 章节正文
{chapter_content}

## 旧状态卡（结算前）
{old_state}

## 新状态卡（结算后）
{new_state}

## 旧 Hook 池（结算前）
{old_hooks}

## 新 Hook 池（结算后）
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

// extract_json_block 已收口到 crate::shared::utils::json::extract_json_block，见上方 use 声明。

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
