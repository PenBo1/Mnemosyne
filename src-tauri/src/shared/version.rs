// ============================================================================
// version —— 章节版本共享数据类型
// ============================================================================
//
// 下沉理由：infrastructure/db/version_store.rs 需要这些类型做 DB 序列化/反序列化，
// 而它的定义原本在 features/version/models.rs。若留在 features/version，infra
// 就要反向依赖 features/version，违反分层架构。下沉到 shared/ 后，infra 和
// features/version 都依赖 shared，互不依赖。
//
// 仅含纯数据类型 + 格式化/解析 trait（Display/FromStr/Default），无业务逻辑、
// 无 I/O。Display/FromStr 是数据序列化的伴随行为，跟随类型下沉。

use serde::{Deserialize, Serialize};

/// 章节修订模式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionMode {
    Auto,
    Polish,
    Rewrite,
    Rework,
    SpotFix,
    Manual,
}

impl Default for RevisionMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl std::fmt::Display for RevisionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RevisionMode::Auto => write!(f, "auto"),
            RevisionMode::Polish => write!(f, "polish"),
            RevisionMode::Rewrite => write!(f, "rewrite"),
            RevisionMode::Rework => write!(f, "rework"),
            RevisionMode::SpotFix => write!(f, "spot_fix"),
            RevisionMode::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for RevisionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(RevisionMode::Auto),
            "polish" => Ok(RevisionMode::Polish),
            "rewrite" => Ok(RevisionMode::Rewrite),
            "rework" => Ok(RevisionMode::Rework),
            "spot_fix" => Ok(RevisionMode::SpotFix),
            "manual" => Ok(RevisionMode::Manual),
            _ => Err(format!("Unknown revision mode: {}", s)),
        }
    }
}

/// 章节版本 —— 修订后的章节内容快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVersion {
    pub id: String,
    pub novel_id: String,
    pub chapter_number: u32,
    pub version_number: u32,
    pub content: String,
    pub content_hash: String,
    pub word_count: u32,
    pub revision_reason: String,
    pub revision_mode: RevisionMode,
    pub created_at: String,
}

/// 创建版本请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub novel_id: String,
    pub chapter_number: u32,
    pub content: String,
    pub revision_mode: RevisionMode,
    pub revision_reason: String,
}
