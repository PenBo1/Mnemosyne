
use serde::{Deserialize, Serialize};

/// Radar 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarError {
    /// 错误信息
    pub message: String,
}