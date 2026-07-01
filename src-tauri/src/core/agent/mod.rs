pub mod types;
pub mod base;
pub mod chat_loop;
pub mod attachments;
pub mod config;
pub mod identity;
pub mod agent_identity;
pub mod verification;
pub mod recovery;
pub mod governance;
pub mod architect;
pub mod foundation_reviewer;
pub mod planner;
pub mod composer;
pub mod writer;
pub mod continuity;
pub mod reviser;
pub mod length_metrics;
pub mod length_normalizer;
pub mod observer;
pub mod reflector;
pub mod audit_dimensions;
pub mod genre_profile;
pub mod style_profile;
pub mod spot_fix_patches;
pub mod auto_routing;
pub mod prompts;
pub mod lesson_tracker;

// ── 高优先级移植模块（来自 Hermes Agent）──────────────────────
pub mod iteration_budget;
pub mod tool_guardrails;
pub mod error_classifier;
pub mod context_compressor;
pub mod continuous_learning;
pub mod retry_utils;
pub mod message_sanitization;
pub mod tools;
pub mod task_lifecycle;
pub mod goal_decomposer;
pub mod main_agent;
pub mod sub_agent;

pub use types::*;
pub use base::{AgentContext, BaseAgent, ToolCall, ToolResult, ToolRegistry, ToolExecutor, MemoryEntry, MemoryType, MemorySystem};
pub use config::AgentConfig;
pub use governance::*;
pub use architect::ArchitectAgent;
pub use foundation_reviewer::FoundationReviewerAgent;
pub use planner::PlannerAgent;
pub use composer::ComposerAgent;
pub use writer::WriterAgent;
pub use continuity::ContinuityAuditor;
pub use reviser::ReviserAgent;
pub use length_normalizer::LengthNormalizerAgent;
pub use length_metrics::{
    LengthSpec, LengthCheck, LengthCountingMode, LengthNormalizeMode,
    LengthTelemetry, LengthWarning,
    count_chapter_length, resolve_length_counting_mode, format_length_count,
    is_outside_soft_range, is_outside_hard_range, choose_normalize_mode,
    default_chapter_length, DEFAULT_CHAPTER_LENGTH_ZH, DEFAULT_CHAPTER_LENGTH_EN,
};
pub use observer::ObserverAgent;
pub use reflector::ReflectorAgent;
pub use audit_dimensions::{
    AuditDimensionContext, DimensionInfo, FanficMode,
    DIMENSION_LABELS, build_dimension_list, dimension_name,
    parse_repair_scope, render_dimension_list,
};
pub use genre_profile::{
    GenreProfile, ParsedGenreProfile, GenreEntry, GenreSource,
    parse_genre_profile, read_genre_profile, list_available_genres,
};
pub use style_profile::{
    StyleProfile, ParagraphLengthRange,
    analyze_style, build_deterministic_style_guide,
    save_style_profile, load_style_profile,
};
pub use spot_fix_patches::{SpotFixPatch, SpotFixPatchApplyResult, parse_spot_fix_patches, apply_spot_fix_patches};
pub use auto_routing::{AutoOutputMode, resolve_auto_output_mode};

// ── 移植模块导出 ─────────────────────────────────────────────
pub use iteration_budget::IterationBudget;
pub use tool_guardrails::{ToolCallGuardrailController, ToolGuardrailConfig, ToolGuardrailDecision, ToolCallSignature, toolguard_synthetic_result, append_toolguard_guidance, classify_tool_failure};
pub use error_classifier::{FailoverReason, ClassifiedError, classify_api_error};
pub use context_compressor::{ContextCompressor, CompressorConfig, CompressedSummary, CompressibleMessage};
pub use continuous_learning::{SkillCurator, CuratorConfig, SkillUsage, SkillState, SkillTransition, MemoryProvider, MemoryManager, BuiltinMemoryEntry};
pub use retry_utils::{jittered_backoff, RetryConfig, RetryState};
pub use message_sanitization::{sanitize_message_sequence, validate_message_sequence, fill_missing_tool_results, SanitizeResult};
pub use lesson_tracker::{LessonTracker, ConstraintLesson, append_lessons_to_memory, load_lessons_from_memory};
pub use goal_decomposer::GoalDecomposer;
pub use main_agent::{AgentLoop, AgentStatus, ProgressUpdate, ConfirmationRequest, ConfirmationResponse};

// ── 子 Agent 模块（主 Agent 自主 spawn 的轻量子 Agent）────────
pub use sub_agent::{
    SubAgentControl, SubAgentInfo, SubAgentResult, SubAgentRole,
    SubAgentSpawnRequest, SubAgentStatus, SpawnAgentTool, ParentAgentRefs,
};

// ── 子模块（从 domain/ 迁移）──────────────────────────────────
pub mod pipeline;
pub mod loop_engine;
