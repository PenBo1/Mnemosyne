// ============================================================================
// wiki —— Wiki 知识库共享数据类型
// ============================================================================
//
// 下沉理由：infrastructure/db/wiki_store.rs 需要这些类型做 DB 序列化/反序列化，
// 而它的定义原本在 features/wiki/models.rs。若留在 features/wiki，infra 就
// 要反向依赖 features/wiki，违反分层架构。下沉到 shared/ 后，infra 和
// features/wiki 都依赖 shared，互不依赖。
//
// 仅含纯数据类型 + 格式化/解析 trait（Display/FromStr/Default），无业务逻辑、
// 无 I/O。Display/FromStr 是数据序列化的伴随行为，跟随类型下沉。

use serde::{Deserialize, Serialize};

/// Wiki 条目分类
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiCategory {
    General,
    Character,
    Location,
    Event,
    Concept,
    Reference,
}

impl Default for WikiCategory {
    fn default() -> Self {
        Self::General
    }
}

impl std::fmt::Display for WikiCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WikiCategory::General => write!(f, "general"),
            WikiCategory::Character => write!(f, "character"),
            WikiCategory::Location => write!(f, "location"),
            WikiCategory::Event => write!(f, "event"),
            WikiCategory::Concept => write!(f, "concept"),
            WikiCategory::Reference => write!(f, "reference"),
        }
    }
}

impl std::str::FromStr for WikiCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "general" => Ok(WikiCategory::General),
            "character" => Ok(WikiCategory::Character),
            "location" => Ok(WikiCategory::Location),
            "event" => Ok(WikiCategory::Event),
            "concept" => Ok(WikiCategory::Concept),
            "reference" => Ok(WikiCategory::Reference),
            _ => Err(format!("Unknown wiki category: {}", s)),
        }
    }
}

/// Wiki 条目来源类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiSourceType {
    Manual,
    AiExtracted,
    Imported,
}

impl Default for WikiSourceType {
    fn default() -> Self {
        Self::Manual
    }
}

impl std::fmt::Display for WikiSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WikiSourceType::Manual => write!(f, "manual"),
            WikiSourceType::AiExtracted => write!(f, "ai_extracted"),
            WikiSourceType::Imported => write!(f, "imported"),
        }
    }
}

impl std::str::FromStr for WikiSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manual" => Ok(WikiSourceType::Manual),
            "ai_extracted" => Ok(WikiSourceType::AiExtracted),
            "imported" => Ok(WikiSourceType::Imported),
            _ => Err(format!("Unknown wiki source type: {}", s)),
        }
    }
}
