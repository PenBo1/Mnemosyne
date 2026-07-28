//! ═══════════════════════════════════════════════════════════════════════════
//! Capability Types - 能力技能类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 用于内置能力声明、上下文需求模型、PromptPack 系统和技能解析。

use serde::{Deserialize, Serialize};

/// 上下文层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillContextTier {
    /// 受保护 —— 不可被压缩,必须完整注入
    Protected,
    /// 可压缩 —— 允许语义压缩或片段截取
    Compressible,
}

/// 上下文检索策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillContextRetrieval {
    /// 完整注入
    Full,
    /// 按段注入
    Sections,
    /// 语义检索
    Semantic,
}

/// 技能源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSource {
    Builtin,
    Project,
    User,
    External,
}

impl Default for SkillSource {
    fn default() -> Self {
        Self::Builtin
    }
}

/// 单个上下文需求声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContextNeed {
    pub id: String,
    pub purpose: String,
    pub sources: Vec<String>,
    pub tier: SkillContextTier,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default = "default_retrieval")]
    pub retrieval: SkillContextRetrieval,
}

fn default_retrieval() -> SkillContextRetrieval {
    SkillContextRetrieval::Semantic
}

/// Capability Skill 清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySkillManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub when_to_use: String,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub session_kinds: Vec<String>,
    #[serde(default)]
    pub prompt_packs: Vec<String>,
    #[serde(default)]
    pub tool_hints: Vec<String>,
    #[serde(default)]
    pub context_needs: Vec<SkillContextNeed>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub source: SkillSource,
}

/// PromptPack 清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPackManifest {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub prompts: Vec<String>,
    #[serde(default)]
    pub source: SkillSource,
}

/// Builtin Prompt 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinPrompt {
    pub id: String,
    pub pack_id: String,
    pub title: String,
    pub content: String,
}

/// 技能解析输入
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillResolutionInput {
    #[serde(default)]
    pub requested_skills: Vec<String>,
    #[serde(default)]
    pub disabled_skills: Vec<String>,
    #[serde(default)]
    pub session_kind: Option<String>,
    #[serde(default)]
    pub instruction: Option<String>,
    /// 候选技能 id(由 agent/model 提议,registry 校验过滤)
    #[serde(default)]
    pub candidate_skills: Vec<String>,
}

/// 技能解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResolutionResult {
    pub used_skills: Vec<CapabilitySkillManifest>,
    pub forced_skill_ids: Vec<String>,
    pub auto_skill_ids: Vec<String>,
    pub missing_skill_ids: Vec<String>,
    pub disabled_skill_ids: Vec<String>,
    pub available_skill_ids: Vec<String>,
}

/// 已加载的 PromptPack Prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedPromptPackPrompt {
    pub prompt_id: String,
    pub content: String,
    pub source: PromptSource,
    pub path: Option<String>,
    pub title: Option<String>,
    pub pack_id: Option<String>,
}

/// Prompt 来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptSource {
    Project,
    User,
    Builtin,
}
