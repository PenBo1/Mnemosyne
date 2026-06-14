use crate::errors::AppError;
use crate::domain::harness::{GlobalHarnessConfig, AgentConfigManager, ConstraintEngine, FeedbackLoop, QualityGateEvaluator};
use crate::domain::harness::agent_configs::AgentConfig;
use crate::domain::harness::types::MergedHarnessContext;
use crate::infra::llm::{Message as LlmMessage, Provider};
use crate::infra::db::Database;
use crate::domain::story::StoryManager;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PipelineConfig {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: std::path::PathBuf,
    pub global_harness: GlobalHarnessConfig,
    pub agent_configs: AgentConfigManager,
    pub db: Arc<Mutex<Database>>,
}

pub struct PipelineRunner {
    pub(crate) config: PipelineConfig,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub(crate) fn story_manager(&self) -> StoryManager {
        StoryManager::new(self.config.project_root.clone())
    }

    fn load_novel_harness(&self, book_id: &str) -> Option<crate::domain::harness::types::NovelHarness> {
        let config_dir = self.story_manager().book_dir(book_id).join("config");
        let harness_path = config_dir.join("novel_harness.json");
        if harness_path.exists() {
            let content = std::fs::read_to_string(&harness_path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    pub(crate) fn build_context(&self, book_id: &str, agent_role: &str) -> MergedHarnessContext {
        let _agent_config = self.config.agent_configs.get(agent_role)
            .cloned()
            .unwrap_or_else(|| panic!("No config for agent: {}", agent_role));

        let novel = self.load_novel_harness(book_id);
        let project = self.config.global_harness.project();

        let lessons = match self.config.db.try_lock() {
            Ok(db) => FeedbackLoop::get_active_lessons(book_id, &db)
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        ConstraintEngine::merge(project, novel.as_ref(), &lessons)
    }

    pub(crate) fn get_agent_config(&self, role: &str) -> &AgentConfig {
        self.config.agent_configs.get(role)
            .expect("Agent config not found")
    }

    pub(crate) async fn call_llm(&self, system: &str, user: &str) -> Result<String, AppError> {
        tracing::debug!(
            model = %self.config.model,
            system_len = system.len(),
            user_len = user.len(),
            "LLM call initiated"
        );
        let start = std::time::Instant::now();

        let messages = vec![
            LlmMessage {
                role: "user".to_string(),
                content: user.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let response = self.config.provider.complete(
            &self.config.model,
            system,
            &messages,
        ).await?;

        let elapsed = start.elapsed().as_millis();
        tracing::info!(
            model = %self.config.model,
            response_len = response.len(),
            elapsed_ms = elapsed,
            "LLM call completed"
        );

        Ok(response)
    }

    pub fn evaluate_gates(
        &self,
        content: &str,
        word_count: Option<u32>,
        audit_result: Option<&crate::domain::story::AuditResult>,
        book_id: &str,
    ) -> crate::domain::harness::types::GateEvaluation {
        let ctx = self.build_context(book_id, "writer");
        QualityGateEvaluator::evaluate_stage(
            content,
            word_count,
            audit_result,
            &ctx.quality_gates,
        )
    }

    pub fn record_feedback(
        &self,
        novel_id: &str,
        chapter_number: u32,
        agent_role: &str,
        error_type: &str,
        dimension: Option<&str>,
        severity: &str,
        description: &str,
        suggestion: Option<&str>,
        rules: &[crate::domain::harness::types::FeedbackRule],
        db: &crate::infra::db::Database,
    ) -> Result<Option<crate::domain::harness::types::ConstraintLesson>, AppError> {
        FeedbackLoop::record_error(
            novel_id,
            chapter_number,
            agent_role,
            error_type,
            dimension,
            severity,
            description,
            suggestion,
            rules,
            db,
        )
    }
}
