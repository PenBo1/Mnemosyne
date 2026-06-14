use serde::{Deserialize, Serialize};

// ── Agent Roles ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Architect,
    Planner,
    Composer,
    Writer,
    Normalizer,
    Auditor,
    Reviser,
    Observer,
    Reflector,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architect => write!(f, "architect"),
            Self::Planner => write!(f, "planner"),
            Self::Composer => write!(f, "composer"),
            Self::Writer => write!(f, "writer"),
            Self::Normalizer => write!(f, "normalizer"),
            Self::Auditor => write!(f, "auditor"),
            Self::Reviser => write!(f, "reviser"),
            Self::Observer => write!(f, "observer"),
            Self::Reflector => write!(f, "reflector"),
        }
    }
}

// ── Agent Input/Output ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentInput {
    CreateBook {
        title: String,
        genre: String,
        brief: Option<String>,
    },
    PlanChapter {
        chapter: u32,
    },
    ComposeChapter {
        chapter: u32,
    },
    WriteChapter {
        chapter: u32,
        target_words: Option<u32>,
    },
    NormalizeChapter {
        chapter: u32,
    },
    SettleChapter {
        chapter: u32,
    },
    AuditChapter {
        chapter: u32,
    },
    ReviseChapter {
        chapter: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentOutput {
    BookCreated {
        book_id: String,
    },
    ChapterIntent {
        chapter: u32,
        intent: ChapterIntent,
    },
    ContextPackage {
        chapter: u32,
        package: ContextPackage,
    },
    ChapterDraft {
        chapter: u32,
        title: String,
        content: String,
        word_count: u32,
    },
    NormalizedDraft {
        chapter: u32,
        content: String,
        word_count: u32,
    },
    RuntimeStateDelta {
        chapter: u32,
        delta: RuntimeStateDelta,
    },
    AuditResult {
        chapter: u32,
        result: AuditResult,
    },
    RevisedDraft {
        chapter: u32,
        content: String,
        word_count: u32,
    },
}

// ── Chapter Intent (Plan output) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterIntent {
    pub chapter: u32,
    pub goal: String,
    pub must_keep: Vec<String>,
    pub must_avoid: Vec<String>,
    pub focus_points: Vec<String>,
    pub hook_agenda: HookAgenda,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookAgenda {
    pub open: Vec<HookAction>,
    pub advance: Vec<HookAction>,
    pub resolve: Vec<HookAction>,
    pub defer: Vec<HookAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAction {
    pub name: String,
    pub description: String,
}

// ── Context Package (Compose output) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub chapter: u32,
    pub selected_context: Vec<ContextSource>,
    pub rule_stack: RuleStack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    pub source: String,
    pub reason: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStack {
    pub layers: Vec<RuleLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleLayer {
    pub name: String,
    pub rules: Vec<String>,
}

// ── Runtime State Delta (Settle output) ──────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeStateDelta {
    pub hook_ops: Vec<HookOp>,
    pub facts_new: Vec<Fact>,
    pub summary_new: Option<ChapterSummary>,
    pub chapter: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOp {
    pub op: HookOpType,
    pub name: String,
    #[serde(rename = "type")]
    pub hook_type: Option<String>,
    pub status: Option<HookStatus>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOpType {
    Upsert,
    Mention,
    Resolve,
    Defer,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HookStatus {
    #[default]
    Open,
    Progressing,
    Deferred,
    Resolved,
}

// ── Facts (temporal triples) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: FactCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FactCategory {
    Character,
    Location,
    Resource,
    Relationship,
    Emotion,
    Information,
    Hook,
    Time,
    Physical,
}

// ── Chapter Summary ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub chapter: u32,
    pub title: String,
    pub characters: Vec<String>,
    pub events: Vec<String>,
    pub state_changes: Vec<String>,
    pub mood: String,
}

// ── Audit Result ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub passed: bool,
    pub score: f64,
    pub issues: Vec<AuditIssue>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIssue {
    pub severity: Severity,
    pub category: String,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

// ── Book Config ──────────────────────────────────────────────────

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BookStatus {
    #[default]
    Drafting,
    Writing,
    Reviewing,
    Completed,
    Paused,
}

// ── Story State ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoryState {
    pub current_chapter: u32,
    pub total_words: u32,
    pub hooks: Vec<HookRecord>,
    pub facts: Vec<TemporalFact>,
    pub summaries: Vec<ChapterSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRecord {
    pub hook_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub hook_type: String,
    pub start_chapter: u32,
    pub status: HookStatus,
    pub expected_payoff: String,
    pub last_advanced_chapter: u32,
    pub core_hook: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFact {
    pub fact_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: FactCategory,
    pub valid_from_chapter: u32,
    pub valid_until_chapter: Option<u32>,
    pub source_chapter: u32,
    pub created_at: String,
}

// ── Genre Profile ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreProfile {
    pub id: String,
    pub name: String,
    pub language: String,
    pub fatigue_words: Vec<String>,
    pub pacing_rule: String,
    pub chapter_types: Vec<String>,
    pub numerical_system: bool,
    pub power_scaling: bool,
    pub audit_dimensions: Vec<u32>,
}

// ── Book Rules ───────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookRules {
    pub protagonist: Option<ProtagonistLock>,
    pub prohibitions: Vec<String>,
    pub style_rules: Vec<String>,
    pub pacing_rules: Vec<String>,
    pub forbidden_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtagonistLock {
    pub name: String,
    pub personality_traits: Vec<String>,
    pub behavioral_constraints: Vec<String>,
}

// ── Chapter Meta ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMeta {
    pub number: u32,
    pub title: String,
    pub status: ChapterStatus,
    pub word_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterStatus {
    #[default]
    Drafting,
    Drafted,
    Auditing,
    AuditPassed,
    AuditFailed,
    Revising,
    Revised,
    ReadyForReview,
    Approved,
    Published,
}

// ── Length Governance ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthSpec {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
}

impl LengthSpec {
    pub fn from_chapter_words(words: u32) -> Self {
        let target = words;
        let variance = (words as f64 * 0.15) as u32;
        let hard_variance = (words as f64 * 0.3) as u32;
        Self {
            target,
            soft_min: target.saturating_sub(variance),
            soft_max: target + variance,
            hard_min: target.saturating_sub(hard_variance),
            hard_max: target + hard_variance,
        }
    }

    pub fn check(&self, word_count: u32) -> LengthCheck {
        if word_count < self.hard_min {
            LengthCheck::TooShort
        } else if word_count > self.hard_max {
            LengthCheck::TooLong
        } else if word_count < self.soft_min || word_count > self.soft_max {
            LengthCheck::OutsideSoft
        } else {
            LengthCheck::Ok
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LengthCheck {
    Ok,
    OutsideSoft,
    TooShort,
    TooLong,
}
