use std::sync::Arc;
use crate::infra::data_dir::DataDir;
use crate::infra::db::Database;
use crate::infra::llm::ProviderRegistry;
use crate::infra::skill::SkillManager;
use crate::infra::sandbox::enforce::SandboxEnforcer;
use crate::infra::memory::MemoryStore;
use crate::infra::feedback::FeedbackStore;
use crate::infra::mcp::McpServer;
use crate::domain::pipeline::Scheduler;
use crate::domain::session::{Session, SessionConfig, SessionStatus};

pub struct AppState {
    pub data_dir: DataDir,
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub feedback_db: Arc<tokio::sync::Mutex<Database>>,
    pub provider_registry: tokio::sync::Mutex<ProviderRegistry>,
    pub skill_manager: tokio::sync::Mutex<SkillManager>,
    pub sandbox: tokio::sync::Mutex<SandboxEnforcer>,
    pub memory_store: Arc<MemoryStore>,
    pub feedback_store: Arc<tokio::sync::Mutex<FeedbackStore>>,
    pub mcp_server: tokio::sync::Mutex<McpServer>,
    pub scheduler: tokio::sync::Mutex<Option<Arc<Scheduler>>>,
    pub app_handle: tauri::AppHandle,
    /// Active SQ/EQ sessions keyed by session ID
    pub sessions: tokio::sync::Mutex<std::collections::HashMap<String, Session>>,
}

impl AppState {
    /// Get or create a session for the given session ID.
    ///
    /// If a session already exists and is not shut down, returns true.
    /// Otherwise creates a new session with the current provider/model config.
    pub async fn ensure_session(
        &self,
        session_id: &str,
    ) -> Result<(), crate::errors::AppError> {
        let mut sessions = self.sessions.lock().await;

        // Check if existing session is still alive
        if let Some(session) = sessions.get(session_id) {
            if session.status() != SessionStatus::Shutdown {
                return Ok(());
            }
        }

        // Build session config from current state
        let (provider, model) = {
            let registry = self.provider_registry.lock().await;
            let provider = registry.default()
                .map_err(|e| crate::errors::AppError::provider_not_found(e.to_string()))?;
            let model = registry.default_model().to_string();
            (provider, model)
        };

        let config = SessionConfig {
            provider,
            model,
            project_root: self.data_dir.root().join("workspace"),
            data_dir: self.data_dir.clone(),
            db: self.db.clone(),
            sandbox: Arc::new(tokio::sync::Mutex::new(
                crate::infra::sandbox::enforce::SandboxEnforcer::new(
                    crate::infra::sandbox::policy::SandboxPolicy::restricted(),
                    self.data_dir.root().clone(),
                )
            )),
            memory_store: self.memory_store.clone(),
            feedback_store: self.feedback_store.clone(),
            model_overrides: std::collections::HashMap::new(),
        };

        let session = Session::spawn(config, session_id.to_string());
        sessions.insert(session_id.to_string(), session);
        Ok(())
    }
}
