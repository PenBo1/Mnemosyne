
//! ═══════════════════════════════════════════════════════════════════════════
//! Types - 会话类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 会话错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionError {
    /// 错误信息
    pub message: String,
}

/// 会话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息 ID
    pub id: String,
    /// 角色（user/assistant/system）
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: String,
}