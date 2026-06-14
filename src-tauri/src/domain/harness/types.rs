use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHarness {
    pub version: String,
    pub agent_constraints: AgentConstraints,
    pub tool_constraints: ToolConstraints,
    pub pipeline_config: PipelineHarnessConfig,
    pub quality_gates: Vec<QualityGate>,
    pub context_engine: ContextEngineConfig,
    pub feedback_rules: Vec<FeedbackRule>,
    pub gc_policy: GcPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConstraints {
    pub max_turns_per_session: u32,
    pub max_tool_calls_per_turn: u32,
    pub required_output_format: OutputFormat,
    pub forbidden_patterns: Vec<String>,
    pub role_isolation: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConstraints {
    pub tool_permissions: HashMap<String, ToolPermission>,
    pub rate_limits: HashMap<String, RateLimit>,
    pub approval_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub allowed_agents: Option<Vec<String>>,
    pub max_calls_per_turn: Option<u32>,
    pub requires_novel_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub max_calls: u32,
    pub window_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineHarnessConfig {
    pub stage_order: Vec<String>,
    pub required_stages: Vec<String>,
    pub conditional_stages: Vec<ConditionalStage>,
    pub max_revision_rounds: u32,
    pub auto_revise_threshold: f64,
    pub gate_config: HashMap<String, StageGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalStage {
    pub stage: String,
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageGate {
    pub min_score: Option<f64>,
    pub max_critical_issues: Option<u32>,
    pub block_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub gate_type: GateType,
    pub threshold: f64,
    pub dimension: Option<String>,
    pub action_on_fail: GateAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GateType {
    ScoreThreshold,
    IssueCount,
    ConsistencyCheck,
    WordCountRange,
    ForbiddenPattern,
    CompletenessCheck,
    DimensionScore,
    CustomRule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GateAction {
    Block,
    Revise,
    Warn,
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GateSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEngineConfig {
    pub max_system_prompt_tokens: u32,
    pub max_context_window_tokens: u32,
    pub protected_sections: Vec<String>,
    pub context_priorities: Vec<String>,
    pub compaction_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRule {
    pub id: String,
    pub trigger: FeedbackTrigger,
    pub constraint: String,
    pub target: FeedbackTarget,
    pub cooldown_chapters: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackTrigger {
    pub error_type: String,
    pub min_occurrences: u32,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackTarget {
    SystemPrompt,
    AuditDimension,
    WritingRules,
    AllAgents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcPolicy {
    pub stale_snapshot_days: u32,
    pub max_snapshots_per_novel: u32,
    pub compact_state_every_n_chapters: u32,
    pub archive_completed_novels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Markdown,
    Json,
    Structured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelHarness {
    pub version: String,
    pub novel_id: String,
    pub genre_config: GenreHarnessConfig,
    pub style_profile: StyleProfile,
    pub continuity_rules: ContinuityRules,
    pub audit_dimensions: AuditDimensionConfig,
    pub quality_gates: Vec<QualityGate>,
    pub writing_constraints: WritingConstraints,
    pub lesson_log: Vec<ConstraintLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreHarnessConfig {
    pub genre_id: String,
    pub genre_name: String,
    pub language: String,
    pub fatigue_words: Vec<String>,
    pub pacing_rules: Vec<String>,
    pub chapter_types: Vec<String>,
    pub genre_specific_rules: Vec<String>,
    pub numerical_system: bool,
    pub power_scaling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    pub narrative_person: String,
    pub tone: Vec<String>,
    pub prose_density: String,
    pub dialogue_ratio: String,
    pub sentence_length_target: u32,
    pub paragraph_length_target: u32,
    pub forbidden_phrases: Vec<String>,
    pub required_devices: Vec<String>,
    pub anti_ai_rules: Vec<String>,
    pub reference_authors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityRules {
    pub protagonist_lock: Option<ProtagonistLock>,
    pub timeline_strict: bool,
    pub character_appearance_max_absence: Option<u32>,
    pub hook_lifecycle_rules: Vec<HookLifecycleRule>,
    pub fact_validation_rules: Vec<String>,
    pub relationship_change_requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtagonistLock {
    pub name: String,
    pub personality_traits: Vec<String>,
    pub behavioral_constraints: Vec<String>,
    pub prohibited_actions: Vec<String>,
    pub speech_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookLifecycleRule {
    pub hook_type: String,
    pub max_open_chapters: Option<u32>,
    pub requires_payoff_before: Option<String>,
    pub max_concurrent_open: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDimensionConfig {
    pub enabled_dimensions: Vec<String>,
    pub dimension_weights: HashMap<String, f64>,
    pub severity_overrides: HashMap<String, String>,
    pub pass_threshold: f64,
    pub critical_dimensions: Vec<String>,
    pub custom_rules: Vec<CustomAuditRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAuditRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub check_type: String,
    pub pattern: Option<String>,
    pub severity: String,
    pub suggestion_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingConstraints {
    pub chapter_words: ChapterWordConfig,
    pub structure_rules: Vec<String>,
    pub pacing_rules: Vec<String>,
    pub prohibited_patterns: Vec<String>,
    pub required_elements: Vec<String>,
    pub golden_chapter_rules: HashMap<u32, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterWordConfig {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLesson {
    pub id: String,
    pub novel_id: String,
    pub chapter_number: u32,
    pub error_type: String,
    pub description: String,
    pub constraint_added: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    OocViolation,
    TimelineError,
    LoreConflict,
    HookAbandon,
    AiFlavor,
    WordCountDeviation,
    ForbiddenPhrase,
    PacingIssue,
    DialogueUnnatural,
    FactContradiction,
    StyleInconsistency,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LessonState {
    Active,
    Suppressed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub novel_id: String,
    pub chapter_number: u32,
    pub agent_role: String,
    pub error_type: String,
    pub dimension: Option<String>,
    pub severity: String,
    pub description: String,
    pub suggestion: Option<String>,
    pub lesson_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct MergedHarnessContext {
    pub project: ProjectHarness,
    pub novel: Option<NovelHarness>,
    pub active_constraints: Vec<ActiveConstraint>,
    pub audit_config: AuditDimensionConfig,
    pub quality_gates: Vec<QualityGate>,
}

#[derive(Debug, Clone)]
pub struct ActiveConstraint {
    pub source: ConstraintSource,
    pub target_agent: Option<String>,
    pub priority: u32,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintSource {
    Project,
    Novel,
    Lesson,
}

#[derive(Debug, Clone)]
pub struct GateEvaluation {
    pub passed: bool,
    pub gate_results: Vec<SingleGateResult>,
    pub action: GateAction,
}

#[derive(Debug, Clone)]
pub struct SingleGateResult {
    pub gate_id: String,
    pub gate_name: String,
    pub passed: bool,
    pub actual_value: f64,
    pub threshold: f64,
    pub message: String,
}
