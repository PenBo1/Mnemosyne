// Reflector Agent —— 状态结算专家。
//
// 在 pipeline 的 Reflect 阶段运行（Observer 之后），把 Observer 提取的事实
// 合并为状态增量（SettlementDelta），供调用方合并到 StoryState 并持久化。
//
// 设计参考：inkos 的 Observer → Reflector 两阶段模式。
// - Observer (temperature 0.5)：宁多勿少地提取事实
// - Reflector (temperature 0.3)：保守地把观察结果合并为状态增量
//
// 与 inkos 的差异：
// - inkos 把 Settler 内嵌在 WriterAgent 中（1300+ 行上帝类），Mnemosyne 保持
//   独立模块以符合"职责边界"原则。
// - inkos 用 markdown truth files，Mnemosyne 用 JSON StoryState。
// - inkos 有双轨解析（delta JSON + 全量 markdown fallback），Mnemosyne 只用
//   JSON 路径（符合"no silent fallback"规则，解析失败显式报错）。

use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::features::story::{StoryState, HookRecord, ChapterSummary, StoryFact, HookStatus};
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::settler_prompts;
use super::agent_identity::AgentIdentity;
use super::observer::ObservationOutput;

pub struct ReflectorAgent;

impl Default for ReflectorAgent {
    fn default() -> Self { Self }
}

impl ReflectorAgent {
    pub fn new() -> Self { Self }

    /// 对一章进行状态结算。
    ///
    /// 输入：章节内容、Observer 的提取结果、当前 StoryState。
    /// 输出：SettlementDelta，包含新增/更新的 hooks、本章摘要、新增事实。
    pub async fn reflect_chapter(
        &self,
        ctx: &AgentContext,
        chapter_number: u32,
        title: &str,
        content: &str,
        observations: &ObservationOutput,
        current_state: &StoryState,
        language: &str,
        data_dir: &DataDir,
    ) -> Result<SettlementDelta, AppError> {
        let identity = AgentIdentity::load(data_dir, "reflector");
        let task_query = format!("settle state for chapter {}", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = settler_prompts::build_system_prompt(language, Some(&identity_prefix));
        let observations_text = settler_prompts::format_observations(
            &observations.facts,
            &observations.hooks_new,
            &observations.hooks_advanced,
        );
        let user = settler_prompts::build_user_prompt(
            chapter_number, title, content, &observations_text, current_state, language,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let delta = parse_settlement_delta(&response.content, chapter_number, title)?;
        Ok(delta)
    }
}

#[async_trait]
impl BaseAgent for ReflectorAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Reflector
    }

    fn name(&self) -> &str {
        "reflector"
    }
}

/// 状态结算增量 —— Reflector 的输出。
///
/// 调用方负责把此 delta 合并到 StoryState 并持久化。
#[derive(Debug, Clone, Default)]
pub struct SettlementDelta {
    /// 新增或更新的 hooks（按 hook_id 去重合并到 StoryState.hooks）。
    pub updated_hooks: Vec<HookRecord>,
    /// 本章摘要（追加到 StoryState.summaries）。
    pub chapter_summary: Option<ChapterSummary>,
    /// 新增事实（追加到 StoryState.facts）。
    pub new_facts: Vec<StoryFact>,
    /// 结算备注（LLM 的 post_settlement 说明，用于日志/调试）。
    pub notes: Vec<String>,
}

/// 解析 LLM 输出为 SettlementDelta。
///
/// 失败时显式返回 AppError（不静默 fallback），符合"no silent fallback"规则。
fn parse_settlement_delta(
    content: &str,
    expected_chapter: u32,
    expected_title: &str,
) -> Result<SettlementDelta, AppError> {
    let json = extract_json(content)?;
    let parsed: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        AppError::internal(format!("Reflector output is not valid JSON: {}", e))
    })?;

    let updated_hooks = parse_hooks(parsed.get("updated_hooks"), expected_chapter);
    let chapter_summary = parse_chapter_summary(
        parsed.get("chapter_summary"),
        expected_chapter,
        expected_title,
    );
    let new_facts = parse_facts(parsed.get("new_facts"), expected_chapter);
    let notes = parse_string_array(parsed.get("notes"));

    Ok(SettlementDelta {
        updated_hooks,
        chapter_summary,
        new_facts,
        notes,
    })
}

/// 从 LLM 输出中提取 JSON 字符串。
///
/// 处理两种情况：
/// 1. 整个输出就是 JSON（首选）
/// 2. JSON 被 markdown 代码围栏包裹（```json ... ```）
fn extract_json(content: &str) -> Result<String, AppError> {
    let trimmed = content.trim();

    // 直接解析
    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }

    // 尝试从代码围栏中提取
    if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return Ok(after_fence[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        // 跳过可能的语言标识行
        let after_lang = if after_fence.starts_with('\n') {
            after_fence
        } else if let Some(nl) = after_fence.find('\n') {
            &after_fence[nl + 1..]
        } else {
            after_fence
        };
        if let Some(end) = after_lang.find("```") {
            return Ok(after_lang[..end].trim().to_string());
        }
    }

    // 尝试找到第一个 { 到最后一个 }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    Err(AppError::internal(
        format!("Reflector output does not contain parseable JSON. First 200 chars: {}",
            &trimmed[..trimmed.len().min(200)])
    ))
}

fn parse_hooks(value: Option<&serde_json::Value>, default_chapter: u32) -> Vec<HookRecord> {
    let arr = match value.and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let now = chrono::Utc::now().to_rfc3339();
    arr.iter().filter_map(|item| {
        let hook_id = item.get("hook_id")?.as_str()?.to_string();
        Some(HookRecord {
            hook_id,
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            hook_type: item.get("hook_type").and_then(|v| v.as_str()).unwrap_or("foreshadowing").to_string(),
            start_chapter: item.get("start_chapter").and_then(|v| v.as_u64()).unwrap_or(default_chapter as u64) as u32,
            status: parse_hook_status(item.get("status")),
            expected_payoff: item.get("expected_payoff").and_then(|v| v.as_str()).unwrap_or("mid-arc").to_string(),
            last_advanced_chapter: item.get("last_advanced_chapter").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            core_hook: item.get("core_hook").and_then(|v| v.as_bool()).unwrap_or(false),
            created_at: now.clone(),
            updated_at: now.clone(),
        })
    }).collect()
}

fn parse_hook_status(value: Option<&serde_json::Value>) -> HookStatus {
    match value.and_then(|v| v.as_str()) {
        Some("open") => HookStatus::Open,
        Some("progressing") => HookStatus::Progressing,
        Some("deferred") => HookStatus::Deferred,
        Some("resolved") => HookStatus::Resolved,
        _ => HookStatus::Open,
    }
}

fn parse_chapter_summary(
    value: Option<&serde_json::Value>,
    expected_chapter: u32,
    expected_title: &str,
) -> Option<ChapterSummary> {
    let s = value?;
    if s.is_null() {
        return None;
    }
    let now = chrono::Utc::now().to_rfc3339();
    Some(ChapterSummary {
        chapter: s.get("chapter").and_then(|v| v.as_u64()).unwrap_or(expected_chapter as u64) as u32,
        title: s.get("title").and_then(|v| v.as_str()).unwrap_or(expected_title).to_string(),
        characters: parse_string_array(s.get("characters")),
        events: parse_string_array(s.get("events")),
        state_changes: parse_string_array(s.get("state_changes")),
        hook_activity: parse_string_array(s.get("hook_activity")),
        mood: s.get("mood").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        chapter_type: s.get("chapter_type").and_then(|v| v.as_str()).unwrap_or("other").to_string(),
        created_at: now,
    })
}

fn parse_facts(value: Option<&serde_json::Value>, default_chapter: u32) -> Vec<StoryFact> {
    let arr = match value.and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let now = chrono::Utc::now().to_rfc3339();
    arr.iter().filter_map(|item| {
        let fact_id = item.get("fact_id")?.as_str()?.to_string();
        Some(StoryFact {
            fact_id,
            subject: item.get("subject").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            predicate: item.get("predicate").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            object: item.get("object").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            valid_from_chapter: default_chapter,
            valid_until_chapter: None,
            source_chapter: item.get("source_chapter").and_then(|v| v.as_u64()).unwrap_or(default_chapter as u64) as u32,
            created_at: now.clone(),
        })
    }).collect()
}

fn parse_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_settlement_delta_valid_json() {
        let json = r#"{
            "updated_hooks": [
                {
                    "hook_id": "hook-5-mentor-oath",
                    "name": "导师誓言",
                    "hook_type": "promise",
                    "start_chapter": 5,
                    "status": "progressing",
                    "expected_payoff": "mid-arc",
                    "last_advanced_chapter": 5,
                    "core_hook": true
                }
            ],
            "chapter_summary": {
                "chapter": 5,
                "title": "第五章 启程",
                "characters": ["主角", "导师"],
                "events": ["主角告别导师"],
                "state_changes": ["主角离开家乡"],
                "hook_activity": ["导师誓言推进"],
                "mood": "庄重",
                "chapter_type": "setup"
            },
            "new_facts": [
                {
                    "fact_id": "fact-5-protagonist-departure",
                    "subject": "主角",
                    "predicate": "离开",
                    "object": "家乡",
                    "source_chapter": 5
                }
            ],
            "notes": ["本章建立了主角的启程动机"]
        }"#;
        let delta = parse_settlement_delta(json, 5, "第五章 启程").unwrap();
        assert_eq!(delta.updated_hooks.len(), 1);
        assert_eq!(delta.updated_hooks[0].hook_id, "hook-5-mentor-oath");
        assert!(matches!(delta.updated_hooks[0].status, HookStatus::Progressing));
        assert!(delta.chapter_summary.is_some());
        let summary = delta.chapter_summary.unwrap();
        assert_eq!(summary.chapter, 5);
        assert_eq!(summary.characters, vec!["主角", "导师"]);
        assert_eq!(delta.new_facts.len(), 1);
        assert_eq!(delta.new_facts[0].fact_id, "fact-5-protagonist-departure");
        assert_eq!(delta.notes, vec!["本章建立了主角的启程动机"]);
    }

    #[test]
    fn test_parse_settlement_delta_markdown_fenced_json() {
        let content = "Some preamble text\n\n```json\n{\"updated_hooks\": [], \"chapter_summary\": null, \"new_facts\": [], \"notes\": []}\n```\n\nTrailing text";
        let delta = parse_settlement_delta(content, 1, "").unwrap();
        assert!(delta.updated_hooks.is_empty());
        assert!(delta.chapter_summary.is_none());
    }

    #[test]
    fn test_parse_settlement_delta_invalid_json_returns_error() {
        let content = "this is not json at all";
        let result = parse_settlement_delta(content, 1, "");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not contain parseable JSON"));
    }

    #[test]
    fn test_parse_settlement_delta_empty_arrays() {
        let json = r#"{
            "updated_hooks": [],
            "new_facts": [],
            "notes": []
        }"#;
        let delta = parse_settlement_delta(json, 1, "").unwrap();
        assert!(delta.updated_hooks.is_empty());
        assert!(delta.new_facts.is_empty());
        assert!(delta.chapter_summary.is_none());
    }

    #[test]
    fn test_extract_json_direct() {
        let result = extract_json(r#"{"key": "value"}"#).unwrap();
        assert!(result.contains(r#""key": "value""#));
    }

    #[test]
    fn test_extract_json_from_braces() {
        let content = "Here is the delta: {\"a\": 1} done.";
        let result = extract_json(content).unwrap();
        assert_eq!(result, r#"{"a": 1}"#);
    }
}
