use crate::errors::AppError;
use crate::domain::harness::ContextBuilder;

use super::PipelineRunner;

impl PipelineRunner {
    pub async fn plan_chapter(
        &self,
        book_id: &str,
        context: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!(book_id, "Planning chapter");
        let sm = self.story_manager();
        let state = sm.load_state(book_id)?;

        let agent_config = self.get_agent_config("planner");
        let ctx = self.build_context(book_id, "planner");
        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let author_intent = sm.load_control_doc(book_id, "author_intent.md")
            .unwrap_or(None).unwrap_or_default();
        let current_focus = sm.load_control_doc(book_id, "current_focus.md")
            .unwrap_or(None).unwrap_or_default();
        let book_rules = sm.load_control_doc(book_id, "book_rules.md")
            .unwrap_or(None).unwrap_or_default();

        let user = format!(
            "当前状态：第{}章，共{}字\n\n作者意图：\n{}\n\n当前焦点：\n{}\n\n书级规则：\n{}\n\n{}",
            state.current_chapter,
            state.total_words,
            author_intent,
            current_focus,
            book_rules,
            context.unwrap_or("无额外上下文")
        );

        let response = self.call_llm(&system, &user).await?;
        let json: serde_json::Value = serde_json::from_str(&response)
            .unwrap_or(serde_json::json!({"must_keep": [], "must_avoid": [], "focus_points": [], "context_notes": response}));

        let intent_path = sm.story_dir(book_id).join("state").join(format!("chapter_{:04}_intent.json", state.current_chapter + 1));
        if let Some(parent) = intent_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&intent_path, serde_json::to_string_pretty(&json).unwrap_or_default()).ok();

        tracing::info!(book_id, chapter = state.current_chapter + 1, "Chapter planned");

        Ok(json)
    }

    pub async fn compose_chapter(
        &self,
        book_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        tracing::info!(book_id, "Composing chapter context");
        let sm = self.story_manager();
        let state = sm.load_state(book_id)?;

        let agent_config = self.get_agent_config("composer");
        let ctx = self.build_context(book_id, "composer");
        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let intent_path = sm.story_dir(book_id).join("state").join(format!("chapter_{:04}_intent.json", state.current_chapter + 1));
        let intent = if intent_path.exists() {
            std::fs::read_to_string(&intent_path).unwrap_or_default()
        } else {
            "{}".to_string()
        };

        let author_intent = sm.load_control_doc(book_id, "author_intent.md")
            .unwrap_or(None).unwrap_or_default();
        let current_focus = sm.load_control_doc(book_id, "current_focus.md")
            .unwrap_or(None).unwrap_or_default();
        let book_rules = sm.load_control_doc(book_id, "book_rules.md")
            .unwrap_or(None).unwrap_or_default();

        let user = format!(
            "章节意图：{}\n\n当前状态：第{}章，共{}字，{}个活跃伏笔，{}个事实\n\n作者意图：\n{}\n\n当前焦点：\n{}\n\n书级规则：\n{}",
            intent,
            state.current_chapter,
            state.total_words,
            state.hooks.iter().filter(|h| h.status == crate::domain::story::HookStatus::Open).count(),
            state.facts.len(),
            author_intent,
            current_focus,
            book_rules
        );

        let response = self.call_llm(&system, &user).await?;
        let json: serde_json::Value = serde_json::from_str(&response)
            .unwrap_or(serde_json::json!({"context_notes": response}));

        let context_path = sm.story_dir(book_id).join("state").join(format!("chapter_{:04}_context.json", state.current_chapter + 1));
        if let Some(parent) = context_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&context_path, serde_json::to_string_pretty(&json).unwrap_or_default())
            .map_err(|e| AppError::internal(format!("Failed to save context: {}", e)))?;

        Ok(json)
    }
}
