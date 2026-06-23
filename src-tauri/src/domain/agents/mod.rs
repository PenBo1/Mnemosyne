pub mod types;
pub mod base;
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
pub mod length_normalizer;
pub mod observer;
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
pub use observer::ObserverAgent;

// ── 移植模块导出 ─────────────────────────────────────────────
pub use iteration_budget::IterationBudget;
pub use tool_guardrails::{ToolCallGuardrailController, ToolGuardrailConfig, ToolGuardrailDecision, ToolCallSignature, toolguard_synthetic_result, append_toolguard_guidance, classify_tool_failure};
pub use error_classifier::{FailoverReason, ClassifiedError, classify_api_error};
pub use context_compressor::{ContextCompressor, CompressorConfig, CompressedSummary, CompressibleMessage};
pub use continuous_learning::{SkillCurator, CuratorConfig, SkillUsage, SkillState, SkillTransition, MemoryProvider, MemoryManager, BuiltinMemoryEntry};
pub use retry_utils::{jittered_backoff, RetryConfig, RetryState};
pub use message_sanitization::{sanitize_message_sequence, validate_message_sequence, fill_missing_tool_results, SanitizeResult};
pub use lesson_tracker::{LessonTracker, ConstraintLesson, append_lessons_to_memory, load_lessons_from_memory};
