//! ═══════════════════════════════════════════════════════════════════════════
//! 雷达类型 - 数据模型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// Radar 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarError {
    /// 错误信息
    pub message: String,
}

/// 数据源元信息(用于 radar_list_sources 命令返回)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarSourceInfo {
    /// 数据源标识
    pub name: String,
    /// 展示名称
    pub label: String,
    /// 类型: builtin(自动抓取) / text(用户提供文本)
    pub kind: String,
}