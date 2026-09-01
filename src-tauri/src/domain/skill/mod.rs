//! ═══════════════════════════════════════════════════════════════════════════
//! Skill Domain Module - 技能领域模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供技能管理的核心业务逻辑：
//! - 技能发现、解析和管理
//! - 技能索引构建和相关性计算
//! - 技能创建、更新、删除操作

pub mod types;
pub mod manager;
pub mod parser;

pub use types::{Skill, SkillMeta, SkillScope, SkillMetadata};
pub use manager::SkillManager;
pub use parser::parse_frontmatter;