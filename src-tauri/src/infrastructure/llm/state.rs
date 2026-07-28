//! ═══════════════════════════════════════════════════════════════════════════
//! LLM 状态 - Tauri 状态管理
//! ═══════════════════════════════════════════════════════════════════════════

use tokio::sync::Mutex;
use super::registry::ProviderRegistry;
use crate::infrastructure::fs::data_dir::DataDir;

/// LLM 模块状态
pub struct LlmState {
    /// 提供商注册表
    pub registry: Mutex<ProviderRegistry>,
    /// 数据目录
    pub data_dir: DataDir,
}

impl LlmState {
    /// 创建 LLM 状态
    pub fn new(data_dir: DataDir) -> Self {
        let provider_registry = ProviderRegistry::new(&data_dir);
        tracing::info!(count = provider_registry.list_providers().len(), "Providers loaded");
        Self {
            registry: Mutex::new(provider_registry),
            data_dir,
        }
    }
}