//! ═══════════════════════════════════════════════════════════════════════════
//! Agent - 智能代理模块
//! ═══════════════════════════════════════════════════════════════════════════

// ── 子模块导出 ───────────────────────────────────────────────────────────────

pub mod types;
pub mod engine;
pub mod multi_turn;
pub mod approval;
pub mod commands;
pub mod tools;
pub mod subagent;
pub mod prompts;
pub mod identity;
pub mod loop_engine;
pub mod effort;
pub mod collaboration_style;
pub mod daily_summary;
pub mod daily_summary_commands;
pub mod registry;
pub mod prompt_cache;
pub mod thinking_scrubber;
pub mod compaction;
pub mod context;
pub mod memory;
pub mod curator;
pub mod user_profile;

// ── 公开类型导出 ─────────────────────────────────────────────────────────────

pub use engine::AgentEngine;
pub use types::{ChatEvent, ChatRequest};
pub use subagent::{SubAgentRole, SubAgentResult, SubAgent, SubAgentTool};
pub use effort::{EffortLevel, EffortParams};
pub use collaboration_style::CollaborationStyle;
pub use daily_summary::{DailySummaryTask, DailySummaryConfig, DailySummaryReport, DailySummaryState};
pub use registry::{AgentCategory, AgentDescriptor, AgentRegistry, AgentRegistryState};
pub use prompt_cache::{SystemPromptBuilder, ConversationPromptCache, PromptCacheKey};
pub use compaction::{CompactionPolicy, CompactionStrategy};
pub use context::{ContextEngine, ContextEngineStatus};
pub use multi_turn::{MultiTurnEvent, MultiTurnOutcome, MultiTurnRunner, MultiTurnStream};