use crate::errors::AppError;
use crate::domain::harness::ContextBuilder;

use super::PipelineRunner;

impl PipelineRunner {
    pub async fn observe_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!(book_id, chapter = chapter_number, "Observing chapter");
        let sm = self.story_manager();

        let agent_config = self.get_agent_config("observer");
        let ctx = self.build_context(book_id, "observer");

        let chapter = sm.load_chapter(book_id, chapter_number)?
            .ok_or_else(|| AppError::not_found(format!("Chapter {} not found", chapter_number)))?;

        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let user = format!(
            "请从以下章节提取结构化事实：\n\n章节标题：{}\n章节内容：\n{}",
            chapter.title,
            chapter.content
        );

        let response = self.call_llm(&system, &user).await?;
        let json: serde_json::Value = serde_json::from_str(&response)
            .unwrap_or(serde_json::json!({"facts": [], "hooks_new": [], "hooks_advanced": [], "chapter_summary": {}}));

        Ok(json)
    }

    pub async fn reflect_chapter(
        &self,
        book_id: &str,
        chapter_number: u32,
    ) -> Result<(), AppError> {
        tracing::info!(book_id, chapter = chapter_number, "Reflecting chapter");
        let sm = self.story_manager();
        let mut state = sm.load_state(book_id)?;

        sm.save_snapshot(book_id, chapter_number, &state)?;

        let agent_config = self.get_agent_config("reflector");
        let ctx = self.build_context(book_id, "reflector");

        let observation = self.observe_chapter(book_id, chapter_number).await?;

        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let user = format!(
            "观察结果：{}\n\n当前状态：{}",
            serde_json::to_string_pretty(&observation).unwrap_or_default(),
            serde_json::to_string_pretty(&state).unwrap_or_default()
        );

        let response = self.call_llm(&system, &user).await?;
        let delta: serde_json::Value = serde_json::from_str(&response)
            .unwrap_or(serde_json::json!({}));

        if let Some(hooks_new) = delta.get("hooks_new").and_then(|v| v.as_array()) {
            for hook in hooks_new {
                if let (Some(name), Some(hook_type)) = (
                    hook.get("name").and_then(|v| v.as_str()),
                    hook.get("type").and_then(|v| v.as_str()),
                ) {
                    state.hooks.push(crate::domain::story::HookRecord {
                        hook_id: uuid::Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        hook_type: hook_type.to_string(),
                        start_chapter: chapter_number,
                        status: Default::default(),
                        expected_payoff: hook.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        last_advanced_chapter: chapter_number,
                        core_hook: false,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }

        if let Some(hooks_advanced) = delta.get("hooks_advanced").and_then(|v| v.as_array()) {
            for adv in hooks_advanced {
                if let Some(name) = adv.get("name").and_then(|v| v.as_str()) {
                    if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == name) {
                        if let Some(status_str) = adv.get("status").and_then(|v| v.as_str()) {
                            hook.status = match status_str {
                                "Open" => crate::domain::story::HookStatus::Open,
                                "Progressing" => crate::domain::story::HookStatus::Progressing,
                                "Deferred" => crate::domain::story::HookStatus::Deferred,
                                "Resolved" => crate::domain::story::HookStatus::Resolved,
                                _ => hook.status.clone(),
                            };
                        }
                        hook.last_advanced_chapter = chapter_number;
                        hook.updated_at = chrono::Utc::now().to_rfc3339();
                        if let Some(desc) = adv.get("description").and_then(|v| v.as_str()) {
                            if !desc.is_empty() {
                                hook.expected_payoff = desc.to_string();
                            }
                        }
                    }
                }
            }
        }

        if let Some(summary) = delta.get("summary_new") {
            if let Ok(s) = serde_json::from_value::<crate::domain::story::ChapterSummary>(summary.clone()) {
                state.summaries.push(s);
            }
        }

        if let Some(facts_new) = delta.get("facts_new").and_then(|v| v.as_array()) {
            for fact in facts_new {
                if let (Some(subject), Some(predicate), Some(object)) = (
                    fact.get("subject").and_then(|v| v.as_str()),
                    fact.get("predicate").and_then(|v| v.as_str()),
                    fact.get("object").and_then(|v| v.as_str()),
                ) {
                    state.facts.push(crate::domain::story::StoryFact {
                        fact_id: uuid::Uuid::new_v4().to_string(),
                        subject: subject.to_string(),
                        predicate: predicate.to_string(),
                        object: object.to_string(),
                        valid_from_chapter: chapter_number,
                        valid_until_chapter: None,
                        source_chapter: chapter_number,
                        created_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }

        sm.save_state(book_id, &state)?;
        Ok(())
    }
}
