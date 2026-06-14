pub mod runner;
pub mod book;
pub mod plan;
pub mod write;
pub mod audit;
pub mod revise;
pub mod observe;

pub use runner::{PipelineConfig, PipelineRunner};

use tokio::sync::Mutex;
use std::sync::Arc;
use crate::errors::AppError;
use crate::infra::llm::ProviderRegistry;
use crate::infra::db::Database;
use crate::domain::harness::{GlobalHarnessConfig, AgentConfigManager};

pub async fn build_runner_with_harness(
    provider_registry: &Mutex<ProviderRegistry>,
    global_harness: &Mutex<GlobalHarnessConfig>,
    agent_configs: &Mutex<AgentConfigManager>,
    workspace_path: std::path::PathBuf,
    db: Arc<Mutex<Database>>,
) -> Result<PipelineRunner, AppError> {
    let registry = provider_registry.lock().await;
    let provider = registry.default()?;
    let model = registry.default_model().to_string();
    let gh = global_harness.lock().await;
    let ac = agent_configs.lock().await;

    let config = PipelineConfig {
        provider,
        model,
        project_root: workspace_path,
        global_harness: gh.clone(),
        agent_configs: ac.clone(),
        db,
    };

    Ok(PipelineRunner::new(config))
}
