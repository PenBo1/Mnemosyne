use std::sync::Arc;
use crate::infra::data_dir::DataDir;
use crate::infra::db::Database;
use crate::infra::llm::ProviderRegistry;
use crate::infra::skill::SkillManager;
use crate::domain::tools::ToolRegistry;
use crate::infra::sandbox::enforce::SandboxEnforcer;
use crate::domain::harness::{GlobalHarnessConfig, AgentConfigManager};
use crate::domain::agent::Submission;

pub struct AgentHandle {
    pub tx_sub: tokio::sync::mpsc::Sender<Submission>,
}

pub struct AppState {
    pub data_dir: DataDir,
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub provider_registry: tokio::sync::Mutex<ProviderRegistry>,
    pub skill_manager: tokio::sync::Mutex<SkillManager>,
    pub tool_registry: Arc<ToolRegistry>,
    pub sandbox: tokio::sync::Mutex<SandboxEnforcer>,
    pub global_harness: tokio::sync::Mutex<GlobalHarnessConfig>,
    pub agent_configs: tokio::sync::Mutex<AgentConfigManager>,
    pub agent_handle: tokio::sync::Mutex<Option<AgentHandle>>,
    pub app_handle: tauri::AppHandle,
}
