//! ═══════════════════════════════════════════════════════════════════════════
//! Skill - 技能系统模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! Agent 的核心能力层，实现：
//! - 闭环学习：任务完成自动创建技能
//! - 技能进化：使用中自动改进
//! - 技能发现：扫描目录自动发现 SKILL.md 文件
//! - 技能版本：保留历史版本，支持回滚

pub mod commands;
pub mod discovery;
pub mod types;
pub mod state;
pub mod cache;
pub mod evolution;
pub mod evolution_commands;
pub mod capability_types;
pub mod capability_builtin;
pub mod capability_registry;
pub mod prompt_pack;
pub mod capability_commands;
pub mod bundles;

pub use types::{Skill, SkillMeta, SkillMetadata, SkillScope, SkillVersion};
pub use discovery::SkillManager;
pub use evolution::{SkillEvolutionManager, SkillUsage, SkillState, SkillCreationRequest, CuratorConfig, CuratorReport};
pub use bundles::{BundleRegistry, SkillBundle, invoke_bundle};