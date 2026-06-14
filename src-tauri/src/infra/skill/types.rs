use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub requires_tools: Vec<String>,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
}

fn default_category() -> String {
    "general".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub meta: SkillMeta,
    pub content: String,
    pub path: String,
}
