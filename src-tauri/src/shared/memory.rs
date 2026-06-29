// ============================================================================
// memory —— Agent 记忆系统共享数据类型
// ============================================================================
//
// 下沉理由：infrastructure/state_store/memory.rs 需要持久化这些类型，而它
// 的定义原本在 core/agent/base.rs。若留在 core/agent，infra 就要反向依赖
// core/agent，违反分层架构。下沉到 shared/ 后，infra 和 core/agent 都依赖
// shared，互不依赖。
//
// 仅含纯数据类型，无业务逻辑、无 I/O、无 trait 定义。

use serde::{Deserialize, Serialize};

/// 记忆条目 —— 存储在 archival store 中的一条记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub entry_type: MemoryType,
    pub chapter: Option<u32>,
    pub timestamp: String,
    pub tags: Vec<String>,
}

/// 记忆类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MemoryType {
    Character,
    Plot,
    Setting,
    Dialogue,
    Fact,
    Style,
}
