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

// ── Loop Engineering ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopState {
    pub id: String,
    pub novel_id: String,
    pub pattern_id: String,
    pub status: String,
    pub readiness_level: String,
    pub state_payload: serde_json::Value,
    pub config: serde_json::Value,
    pub token_usage_today: i64,
    pub token_cap_daily: i64,
    pub last_run_at: Option<String>,
    pub last_run_result: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoopStateRequest {
    pub pattern_id: String,
    pub readiness_level: Option<String>,
    pub config: Option<serde_json::Value>,
    pub token_cap_daily: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateLoopStateRequest {
    pub status: Option<String>,
    pub readiness_level: Option<String>,
    pub config: Option<serde_json::Value>,
    pub token_cap_daily: Option<i64>,
    pub last_run_at: Option<String>,
    pub last_run_result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRunLog {
    pub id: String,
    pub loop_state_id: String,
    pub pattern_id: String,
    pub status: String,
    pub phase_results: Vec<serde_json::Value>,
    pub tokens_used: i64,
    pub duration_ms: i64,
    pub findings: Vec<String>,
    pub actions_taken: Vec<String>,
    pub escalations: Vec<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub goal: String,
    pub cadence: String,
    pub risk_level: String,
    pub phases: Vec<serde_json::Value>,
    pub human_gates: Vec<String>,
    pub cost_config: serde_json::Value,
    pub skills_required: Vec<String>,
    pub state_schema: serde_json::Value,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertLoopPatternRequest {
    pub name: String,
    pub description: Option<String>,
    pub goal: Option<String>,
    pub cadence: Option<String>,
    pub risk_level: Option<String>,
    pub phases: Option<Vec<serde_json::Value>>,
    pub human_gates: Option<Vec<String>>,
    pub cost_config: Option<serde_json::Value>,
    pub skills_required: Option<Vec<String>>,
    pub state_schema: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}
