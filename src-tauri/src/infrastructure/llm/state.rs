
use tokio::sync::Mutex;
use super::registry::ProviderRegistry;
use crate::infrastructure::fs::data_dir::DataDir;

pub struct LlmState {
    pub registry: Mutex<ProviderRegistry>,
    pub data_dir: DataDir,
}

impl LlmState {
    pub fn new(data_dir: DataDir) -> Self {
        let provider_registry = ProviderRegistry::new(&data_dir);
        tracing::info!(count = provider_registry.list_providers().len(), "Providers loaded");
        Self {
            registry: Mutex::new(provider_registry),
            data_dir,
        }
    }
}