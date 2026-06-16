use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    pub id: String,
    pub title: String,
    pub genre: String,
    pub platform: String,
    pub status: BookStatus,
    pub language: String,
    pub chapter_words: u32,
    pub target_chapters: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BookStatus {
    Drafting,
    Writing,
    Reviewing,
    Completed,
    Paused,
}

impl Default for BookStatus {
    fn default() -> Self {
        Self::Drafting
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMeta {
    pub number: u32,
    pub title: String,
    pub status: ChapterStatus,
    pub word_count: u32,
    pub audit_passed: bool,
    pub audit_score: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterStatus {
    Draft,
    AuditPassed,
    AuditFailed,
    Revised,
    Finalized,
}

impl Default for ChapterStatus {
    fn default() -> Self {
        Self::Draft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterContent {
    pub number: u32,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRecord {
    pub hook_id: String,
    pub name: String,
    pub hook_type: String,
    pub start_chapter: u32,
    pub status: HookStatus,
    pub expected_payoff: String,
    pub last_advanced_chapter: u32,
    pub core_hook: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HookStatus {
    Open,
    Progressing,
    Deferred,
    Resolved,
}

impl Default for HookStatus {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub chapter: u32,
    pub title: String,
    pub characters: Vec<String>,
    pub events: Vec<String>,
    pub state_changes: Vec<String>,
    pub hook_activity: Vec<String>,
    pub mood: String,
    pub chapter_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryFact {
    pub fact_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from_chapter: u32,
    pub valid_until_chapter: Option<u32>,
    pub source_chapter: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoryState {
    pub current_chapter: u32,
    pub total_words: u32,
    pub hooks: Vec<HookRecord>,
    pub summaries: Vec<ChapterSummary>,
    pub facts: Vec<StoryFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIssue {
    pub severity: AuditSeverity,
    pub category: String,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub passed: bool,
    pub score: f64,
    pub issues: Vec<AuditIssue>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterIntent {
    pub chapter_number: u32,
    pub must_keep: Vec<String>,
    pub must_avoid: Vec<String>,
    pub focus_points: Vec<String>,
    pub context_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub chapter_number: u32,
    pub book_rules: String,
    pub author_intent: String,
    pub current_focus: String,
    pub relevant_facts: Vec<StoryFact>,
    pub active_hooks: Vec<HookRecord>,
    pub recent_summaries: Vec<ChapterSummary>,
    pub previous_chapter_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub chapter_number: u32,
    pub title: String,
    pub content: String,
    pub word_count: u32,
    pub audit: AuditResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSnapshot {
    pub chapter_number: u32,
    pub state: StoryState,
    pub timestamp: String,
}
