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

pub struct AppState {
    pub data_dir: DataDir,
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub feedback_db: Arc<tokio::sync::Mutex<Database>>,
    pub provider_registry: tokio::sync::Mutex<ProviderRegistry>,
    pub skill_manager: tokio::sync::Mutex<SkillManager>,
    pub sandbox: tokio::sync::Mutex<SandboxEnforcer>,
    pub memory_store: Arc<MemoryStore>,
    pub feedback_store: tokio::sync::Mutex<FeedbackStore>,
    pub mcp_server: tokio::sync::Mutex<McpServer>,
    pub scheduler: tokio::sync::Mutex<Option<Arc<Scheduler>>>,
    pub app_handle: tauri::AppHandle,
}
