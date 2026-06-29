use serde::{Deserialize, Serialize};

// WikiCategory 与 WikiSourceType 已下沉到 crate::shared::wiki（修复 infra →
// features/wiki 反向依赖）。这里通过 re-export 保持
// `crate::features::wiki::WikiCategory` 路径兼容。
pub use crate::shared::wiki::{WikiCategory, WikiSourceType};

/// Wiki entry - a knowledge base article for a novel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntry {
    pub id: String,
    pub novel_id: String,
    pub title: String,
    pub content: String,
    pub category: WikiCategory,
    pub source_type: WikiSourceType,
    pub source_chapter: Option<u32>,
    pub tags: Vec<String>,
    pub importance: u32,
    pub word_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// Wiki entity link - a relationship between two wiki entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntityLink {
    pub id: String,
    pub novel_id: String,
    pub source_entry_id: String,
    pub target_entry_id: String,
    pub relation_type: String,
    pub relation_desc: String,
    pub weight: u32,
    pub source_chapter: Option<u32>,
    pub created_at: String,
}

/// Wiki graph node for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiGraphNode {
    pub id: String,
    pub title: String,
    pub category: String,
    pub importance: u32,
}

/// Wiki graph edge for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiGraphEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: u32,
}

/// Wiki graph view - nodes and edges for knowledge graph visualization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiGraphView {
    pub nodes: Vec<WikiGraphNode>,
    pub edges: Vec<WikiGraphEdge>,
}

/// Create wiki entry request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWikiEntryRequest {
    pub novel_id: String,
    pub title: String,
    pub content: String,
    pub category: WikiCategory,
    pub tags: Vec<String>,
    pub source_chapter: Option<u32>,
    pub importance: Option<u32>,
}

/// Update wiki entry request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWikiEntryRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<WikiCategory>,
    pub tags: Option<Vec<String>>,
    pub importance: Option<u32>,
}

/// Create wiki link request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWikiLinkRequest {
    pub novel_id: String,
    pub source_entry_id: String,
    pub target_entry_id: String,
    pub relation_type: String,
    pub relation_desc: String,
    pub weight: Option<u32>,
    pub source_chapter: Option<u32>,
}

/// Wiki entry summary for AI context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntrySummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub excerpt: String,
    pub importance: u32,
}