use std::sync::Arc;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::llm_client::ProviderRegistry;
use crate::features::skill_manager::SkillManager;
use crate::infrastructure::sandbox::enforce::SandboxEnforcer;
use crate::infrastructure::state_store::memory::MemoryStore;
use crate::infrastructure::state_store::feedback::FeedbackStore;
use crate::infrastructure::ai_services::mcp::McpServer;
use crate::core::agent::pipeline::Scheduler;
use crate::infrastructure::db::Database;
use crate::features::session::{Session, SessionConfig, SessionStatus};

/// 每个 session 的 agent 取消状态。
pub struct AgentSessionState {
    pub cancelled: Arc<tokio::sync::RwLock<bool>>,
}

/// 每个 session 的 main agent 状态，包含 channel 句柄。
pub struct MainAgentSessionState {
    pub progress_rx: tokio::sync::mpsc::UnboundedReceiver<crate::core::agent::ProgressUpdate>,
    pub confirmation_tx: tokio::sync::mpsc::UnboundedSender<crate::core::agent::main_agent::ConfirmationResponse>,
    pub cancelled: Arc<tokio::sync::RwLock<bool>>,
}

pub struct AppState {
    pub data_dir: DataDir,
    pub db: Database,
    pub provider_registry: tokio::sync::Mutex<ProviderRegistry>,
    pub skill_manager: tokio::sync::Mutex<SkillManager>,
    pub sandbox: tokio::sync::Mutex<SandboxEnforcer>,
    pub memory_store: Arc<MemoryStore>,
    pub feedback_store: Arc<tokio::sync::Mutex<FeedbackStore>>,
    pub mcp_server: tokio::sync::Mutex<McpServer>,
    pub scheduler: tokio::sync::Mutex<Option<Arc<Scheduler>>>,
    pub app_handle: tauri::AppHandle,
    /// 按 session ID 索引的活跃 SQ/EQ session
    pub sessions: tokio::sync::Mutex<std::collections::HashMap<String, Session>>,
    /// 每个 session 的 agent 取消标志（替代静态 AGENT_STATES）
    pub agent_states: tokio::sync::Mutex<std::collections::HashMap<String, AgentSessionState>>,
    /// 每个 session 的 main agent 状态（替代静态 MAIN_AGENT_STATES）
    pub main_agent_states: tokio::sync::Mutex<std::collections::HashMap<String, MainAgentSessionState>>,
}

impl AppState {
    /// 获取或创建指定 session ID 的 session。
    ///
    /// 如果 session 已存在且未关闭，则直接返回。
    /// 否则使用当前的 provider/model 配置创建新 session。
    pub async fn ensure_session(
        &self,
        session_id: &str,
    ) -> Result<(), crate::shared::errors::AppError> {
        let mut sessions = self.sessions.lock().await;

        // 检查现有 session 是否仍然存活
        if let Some(session) = sessions.get(session_id) {
            if session.status() != SessionStatus::Shutdown {
                return Ok(());
            }
        }

        // 从当前状态构建 session 配置
        let (provider, model) = {
            let registry = self.provider_registry.lock().await;
            let provider = registry.default()
                .map_err(|e| crate::shared::errors::AppError::provider_not_found(e.to_string()))?;
            let model = registry.default_model().to_string();
            (provider, model)
        };

        let config = SessionConfig {
            provider,
            model,
            project_root: self.data_dir.root().join("workspace"),
            data_dir: self.data_dir.clone(),
            db: Arc::new(tokio::sync::Mutex::new(self.db.clone())),
            sandbox: Arc::new(tokio::sync::Mutex::new(
                crate::infrastructure::sandbox::enforce::SandboxEnforcer::new(
                    crate::infrastructure::sandbox::policy::SandboxPolicy::restricted(),
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
