use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub id: String,
    pub name: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptRequest {
    pub name: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePromptRequest {
    pub id: String,
    pub name: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub id: String,
    pub keyword: String,
    pub platform: String,
    pub score: f64,
    pub metadata: serde_json::Value,
    pub scanned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub genre: String,
    pub platform: String,
    pub status: String,
    pub language: String,
    pub word_count: i64,
    pub chapter_count: i64,
    pub target_chapters: i64,
    pub chapter_words: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub novel_id: String,
    pub number: i64,
    pub title: String,
    pub status: String,
    pub word_count: i64,
    pub audit_score: Option<f64>,
    pub revision_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarScan {
    pub id: String,
    pub market_summary: String,
    pub recommendations: Vec<RadarRecommendation>,
    pub raw_rankings: Vec<PlatformRankings>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarRecommendation {
    pub platform: String,
    pub genre: String,
    pub concept: String,
    pub confidence: f64,
    pub reasoning: String,
    pub benchmark_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRankings {
    pub platform: String,
    pub entries: Vec<RankingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEntry {
    pub title: String,
    pub author: String,
    pub category: String,
    pub extra: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarResult {
    pub recommendations: Vec<RadarRecommendation>,
    pub market_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNovelRequest {
    pub workspace_id: String,
    pub title: String,
    pub genre: String,
    pub platform: String,
    pub language: String,
    pub target_chapters: i64,
    pub chapter_words: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNovelRequest {
    pub title: Option<String>,
    pub genre: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub target_chapters: Option<i64>,
    pub chapter_words: Option<i64>,
}
