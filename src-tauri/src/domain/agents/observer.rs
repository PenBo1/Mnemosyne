use async_trait::async_trait;
use crate::errors::AppError;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::observer_prompts;

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
    ) -> Result<ObservationOutput, AppError> {
        let system = observer_prompts::build_system_prompt(language);
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

pub struct ObservationOutput {
    pub facts: Vec<ExtractedFact>,
    pub hooks_new: Vec<HookAction>,
    pub hooks_advanced: Vec<HookAction>,
    pub chapter_summary: Option<ChapterSummaryRow>,
}

pub struct ExtractedFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: String,
}

pub struct HookAction {
    pub name: String,
    pub hook_type: String,
    pub status: String,
    pub description: String,
}

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
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
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

        return Ok(ObservationOutput {
            facts,
            hooks_new,
            hooks_advanced,
            chapter_summary,
        });
    }

    // Fallback
    Ok(ObservationOutput {
        facts: Vec::new(),
        hooks_new: Vec::new(),
        hooks_advanced: Vec::new(),
        chapter_summary: None,
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
