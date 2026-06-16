use std::sync::Arc;
use crate::infra::data_dir::DataDir;
use crate::infra::db::Database;
use crate::infra::llm::ProviderRegistry;
use crate::infra::skill::SkillManager;
use crate::infra::sandbox::enforce::SandboxEnforcer;

pub struct AppState {
    pub data_dir: DataDir,
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub provider_registry: tokio::sync::Mutex<ProviderRegistry>,
    pub skill_manager: tokio::sync::Mutex<SkillManager>,
    pub sandbox: tokio::sync::Mutex<SandboxEnforcer>,
    pub app_handle: tauri::AppHandle,
}
