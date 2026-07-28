//! ═══════════════════════════════════════════════════════════════════════════
//! Fragments - ContextualUserFragment 实现
//! ═══════════════════════════════════════════════════════════════════════════

pub mod soul;
pub mod context_files;
pub mod skills;
pub mod memory;
pub mod user_profile;
pub mod permissions_instructions;
pub mod token_budget_reminder;
pub mod world_state;

pub use context_files::ContextFilesFragment;
pub use memory::MemoryFragment;
pub use permissions_instructions::{AskForApproval, PermissionsInstructionsFragment, SandboxMode};
pub use skills::{SkillInstructions, SkillsFragment};
pub use soul::SoulFragment;
pub use token_budget_reminder::TokenBudgetReminderFragment;
pub use user_profile::UserProfileFragment;
pub use world_state::{WorldState, WorldStateFragment};
