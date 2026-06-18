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
    /// Version number, incremented on each update
    #[serde(default = "default_version")]
    pub version: u32,
    /// Tags for search/filtering
    #[serde(default)]
    pub tags: Vec<String>,
    /// Dependencies on other skills
    #[serde(default)]
    pub depends_on: Vec<String>,
}

fn default_category() -> String {
    "general".to_string()
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub meta: SkillMeta,
    pub content: String,
    pub path: String,
    /// History of previous versions (content snapshots)
    #[serde(default)]
    pub history: Vec<SkillVersion>,
}

/// A snapshot of a previous skill version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: u32,
    pub content: String,
    pub updated_at: String,
    pub change_summary: String,
}

impl Skill {
    /// Update skill content, bumping version and saving history
    pub fn update_content(&mut self, new_content: String, change_summary: String) {
        // Save current version to history
        self.history.push(SkillVersion {
            version: self.meta.version,
            content: self.content.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            change_summary,
        });
        // Keep only last 10 versions
        if self.history.len() > 10 {
            self.history.drain(0..self.history.len() - 10);
        }
        // Update current
        self.meta.version += 1;
        self.content = new_content;
    }
}
