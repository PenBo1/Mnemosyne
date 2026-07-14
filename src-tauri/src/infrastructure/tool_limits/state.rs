use std::sync::RwLock;

use super::config::ToolLimitsConfig;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

/// ToolLimitsState —— 缓存 ToolLimitsConfig 的 Tauri State。
///
/// 设计:
/// - 启动时从 <data_dir>/tool_limits.json 加载(不存在则用默认值)
/// - update 时先持久化到磁盘,再刷新内存缓存
/// - get 直接读内存,避免每次都 hit 文件系统
///
/// 线程安全:使用 RwLock,读多写少场景适合。
#[derive(Clone)]
pub struct ToolLimitsState {
    config: std::sync::Arc<RwLock<ToolLimitsConfig>>,
    data_dir: DataDir,
}

impl ToolLimitsState {
    pub fn new(data_dir: DataDir) -> Self {
        let config = ToolLimitsConfig::load(&data_dir).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load tool_limits.json, using default");
            ToolLimitsConfig::default()
        });
        Self {
            config: std::sync::Arc::new(RwLock::new(config)),
            data_dir,
        }
    }

    /// 读取当前配置(内存缓存)。
    pub fn get(&self) -> ToolLimitsConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// 更新配置并持久化到磁盘。
    ///
    /// 失败策略(对齐 "no silent fallback"):
    /// - validate 失败 → 返回 Err,不修改内存或磁盘
    /// - 写盘失败 → 返回 Err,但内存已被修改(为了简单,这里先 validate,再写盘,再更新内存)
    pub fn update(&self, new_config: ToolLimitsConfig) -> Result<(), AppError> {
        new_config.validate()?;
        new_config.save(&self.data_dir)?;
        if let Ok(mut guard) = self.config.write() {
            *guard = new_config;
        }
        Ok(())
    }

    /// 重置为默认值并持久化。
    pub fn reset(&self) -> Result<(), AppError> {
        self.update(ToolLimitsConfig::default())
    }
}
