//! ═══════════════════════════════════════════════════════════════════════════
//! Curator - 后台编排器
//! ═══════════════════════════════════════════════════════════════════════════

pub mod runner;
pub mod state;

pub use runner::{
    CuratorConfig, CuratorLlm, CuratorRunReport, CuratorRunner, ProviderBackedCuratorLlm,
};
pub use state::{
    apply_automatic_transitions, detect_similar_groups, SkillReviewItem, SkillState,
    TransitionConfig,
};
