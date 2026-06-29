use serde::{Deserialize, Serialize};

/// Wiki entry categories
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

/// Wiki entry source type
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