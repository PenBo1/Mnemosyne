
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Wiki 文档分类
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiCategory {
    /// 角色（人物）
    Character,
    /// 地点（场景）
    Location,
    /// 事件（剧情节点）
    Event,
    /// 概念（抽象元素）
    Concept,
    /// 物品（道具）
    Object,
    /// 关系（人物/事件关联）
    Relationship,
    /// 时间线（时间轴）
    Timeline,
    /// 主题（核心议题）
    Theme,
    /// 通用（未分类）
    General,
}

impl std::fmt::Display for WikiCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WikiCategory::Character => write!(f, "character"),
            WikiCategory::Location => write!(f, "location"),
            WikiCategory::Event => write!(f, "event"),
            WikiCategory::Concept => write!(f, "concept"),
            WikiCategory::Object => write!(f, "object"),
            WikiCategory::Relationship => write!(f, "relationship"),
            WikiCategory::Timeline => write!(f, "timeline"),
            WikiCategory::Theme => write!(f, "theme"),
            WikiCategory::General => write!(f, "general"),
        }
    }
}

impl FromStr for WikiCategory {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "character" => Ok(WikiCategory::Character),
            "location" => Ok(WikiCategory::Location),
            "event" => Ok(WikiCategory::Event),
            "concept" => Ok(WikiCategory::Concept),
            "object" => Ok(WikiCategory::Object),
            "relationship" => Ok(WikiCategory::Relationship),
            "timeline" => Ok(WikiCategory::Timeline),
            "theme" => Ok(WikiCategory::Theme),
            "general" => Ok(WikiCategory::General),
            _ => Err(format!("Invalid wiki category: {}", s)),
        }
    }
}

/// Wiki 文档来源类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiSourceType {
    /// 作者定义（手动创建）
    AuthorDefined,
    /// 从文本提取（自动分析）
    ExtractedFromText,
    /// Agent 推断（智能推理）
    InferredByAgent,
    /// 手动录入
    Manual,
}

impl std::fmt::Display for WikiSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WikiSourceType::AuthorDefined => write!(f, "author_defined"),
            WikiSourceType::ExtractedFromText => write!(f, "extracted_from_text"),
            WikiSourceType::InferredByAgent => write!(f, "inferred_by_agent"),
            WikiSourceType::Manual => write!(f, "manual"),
        }
    }
}

impl FromStr for WikiSourceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "author_defined" => Ok(WikiSourceType::AuthorDefined),
            "extracted_from_text" => Ok(WikiSourceType::ExtractedFromText),
            "inferred_by_agent" => Ok(WikiSourceType::InferredByAgent),
            "manual" => Ok(WikiSourceType::Manual),
            _ => Err(format!("Invalid wiki source type: {}", s)),
        }
    }
}

impl Default for WikiCategory {
    fn default() -> Self { WikiCategory::Concept }
}

impl Default for WikiSourceType {
    fn default() -> Self { WikiSourceType::AuthorDefined }
}
