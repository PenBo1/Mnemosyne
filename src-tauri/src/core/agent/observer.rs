use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::observer_prompts;
use super::agent_identity::AgentIdentity;

pub struct ObserverAgent;

impl Default for ObserverAgent {
    fn default() -> Self { Self }
}
impl ObserverAgent {
    pub fn new() -> Self { Self }

    /// Extract facts from a chapter
    pub async fn observe_chapter(
        &self,
        ctx: &AgentContext,
        chapter_number: u32,
        title: &str,
        content: &str,
        language: &str,
        data_dir: &DataDir,
    ) -> Result<ObservationOutput, AppError> {
        let identity = AgentIdentity::load(data_dir, "observer");
        let task_query = format!("observe chapter {} and extract facts", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = observer_prompts::build_system_prompt(language, Some(&identity_prefix));
        let user = observer_prompts::build_user_prompt(chapter_number, title, content, language);

        let response = self.chat(ctx, &system, &user).await?;
        let output = parse_observation(&response.content)?;
        Ok(output)
    }
}

#[async_trait]
impl BaseAgent for ObserverAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Observer
    }

    fn name(&self) -> &str {
        "observer"
    }
}

#[derive(Debug)]
pub struct ObservationOutput {
    pub facts: Vec<ExtractedFact>,
    pub hooks_new: Vec<HookAction>,
    pub hooks_advanced: Vec<HookAction>,
    pub chapter_summary: Option<ChapterSummaryRow>,
}

#[derive(Debug)]
pub struct ExtractedFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: String,
}

#[derive(Debug)]
pub struct HookAction {
    pub name: String,
    pub hook_type: String,
    pub status: String,
    pub description: String,
}

#[derive(Debug)]
pub struct ChapterSummaryRow {
    pub chapter: u32,
    pub title: String,
    pub characters: Vec<String>,
    pub events: Vec<String>,
    pub state_changes: Vec<String>,
    pub hook_activity: Vec<String>,
    pub mood: String,
    pub chapter_type: String,
}

fn parse_observation(content: &str) -> Result<ObservationOutput, AppError> {
    // S5.8: 不静默 fallback —— JSON 解析失败时显式报错，符合"no silent fallback"规则。
    let json: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        AppError::internal(format!(
            "Observer output failed JSON parsing: {} | raw content (first 200 chars): {}",
            e,
            &content[..content.len().min(200)]
        ))
    })?;

    let facts = json.get("facts")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|item| {
                Some(ExtractedFact {
                    subject: item.get("subject")?.as_str()?.to_string(),
                    predicate: item.get("predicate")?.as_str()?.to_string(),
                    object: item.get("object")?.as_str()?.to_string(),
                    category: item.get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("other")
                        .to_string(),
                })
            }).collect()
        })
        .unwrap_or_default();

    let hooks_new = parse_hook_actions(json.get("hooks_new"));
    let hooks_advanced = parse_hook_actions(json.get("hooks_advanced"));

    let chapter_summary = json.get("chapter_summary").map(|s| ChapterSummaryRow {
        chapter: s.get("chapter").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        title: s.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        characters: parse_string_array(s.get("characters")),
        events: parse_string_array(s.get("events")),
        state_changes: parse_string_array(s.get("state_changes")),
        hook_activity: parse_string_array(s.get("hook_activity")),
        mood: s.get("mood").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        chapter_type: s.get("chapter_type").and_then(|v| v.as_str()).unwrap_or("other").to_string(),
    });

    Ok(ObservationOutput {
        facts,
        hooks_new,
        hooks_advanced,
        chapter_summary,
    })
}

fn parse_hook_actions(value: Option<&serde_json::Value>) -> Vec<HookAction> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|item| {
                Some(HookAction {
                    name: item.get("name")?.as_str()?.to_string(),
                    hook_type: item.get("type")
                        .or_else(|| item.get("hook_type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("foreshadowing")
                        .to_string(),
                    status: item.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Open")
                        .to_string(),
                    description: item.get("description")
                        .or_else(|| item.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            }).collect()
        })
        .unwrap_or_default()
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

    /// S5.8: 无效 JSON 不再静默 fallback，而是显式报错。
    #[test]
    fn test_parse_observation_invalid_json_returns_error() {
        let result = parse_observation("this is not json");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed JSON parsing"));
    }

    /// S5.8: 空 JSON 对象应正常解析（所有字段缺省）
    #[test]
    fn test_parse_observation_empty_json_object() {
        let result = parse_observation("{}");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.facts.is_empty());
        assert!(output.hooks_new.is_empty());
        assert!(output.hooks_advanced.is_empty());
        assert!(output.chapter_summary.is_none());
    }

    /// S5.8: 完整的合法 JSON 应正确解析
    #[test]
    fn test_parse_observation_valid_json() {
        let json = r#"{
            "facts": [
                {"subject": "主角", "predicate": "获得", "object": "宝剑", "category": "item"}
            ],
            "hooks_new": [
                {"name": "神秘信件", "type": "mystery", "status": "Open", "description": "收到匿名信"}
            ],
            "chapter_summary": {
                "chapter": 3,
                "title": "第三章",
                "characters": ["主角"],
                "events": ["获得宝剑"],
                "state_changes": [],
                "hook_activity": [],
                "mood": "紧张",
                "chapter_type": "advancement"
            }
        }"#;
        let result = parse_observation(json);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.facts.len(), 1);
        assert_eq!(output.facts[0].subject, "主角");
        assert_eq!(output.hooks_new.len(), 1);
        assert_eq!(output.hooks_new[0].name, "神秘信件");
        let summary = output.chapter_summary.unwrap();
        assert_eq!(summary.chapter, 3);
        assert_eq!(summary.mood, "紧张");
    }
}
