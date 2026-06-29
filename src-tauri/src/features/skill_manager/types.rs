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
    /// 版本号，每次更新时递增
    #[serde(default = "default_version")]
    pub version: u32,
    /// 用于搜索/过滤的标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 依赖的其他 skill
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
    /// 历史版本记录（内容快照）
    #[serde(default)]
    pub history: Vec<SkillVersion>,
}

/// 旧版本 skill 的快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: u32,
    pub content: String,
    pub updated_at: String,
    pub change_summary: String,
}

impl Skill {
    /// 更新 skill 内容，递增版本号并保存历史
    pub fn update_content(&mut self, new_content: String, change_summary: String) {
        // 将当前版本保存到历史
        self.history.push(SkillVersion {
            version: self.meta.version,
            content: self.content.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            change_summary,
        });
        // 仅保留最近 10 个版本
        if self.history.len() > 10 {
            self.history.drain(0..self.history.len() - 10);
        }
        // 更新当前版本
        self.meta.version += 1;
        self.content = new_content;
    }
}
