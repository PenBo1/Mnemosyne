
use serde::{Deserialize, Serialize};

/// 技能元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 分类标签
    #[serde(default = "default_category")]
    pub category: String,
    /// 需要的工具列表
    #[serde(default)]
    pub requires_tools: Vec<String>,
    /// 支持的平台
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
    /// 版本号
    #[serde(default = "default_version")]
    pub version: u32,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 依赖的其他技能
    #[serde(default)]
    pub depends_on: Vec<String>,
}

fn default_category() -> String {
    "general".to_string()
}

fn default_version() -> u32 {
    1
}

/// 技能定义
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    /// 技能元数据
    pub meta: SkillMeta,
    /// 技能内容（Prompt）
    pub content: String,
    /// 技能文件路径
    pub path: String,
    /// 版本历史
    #[serde(default)]
    pub history: Vec<SkillVersion>,
}

/// 技能版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    /// 版本号
    pub version: u32,
    /// 版本内容
    pub content: String,
    /// 更新时间
    pub updated_at: String,
    /// 变更摘要
    pub change_summary: String,
}

impl Skill {
    /// 更新技能内容
    ///
    /// 自动记录版本历史，保留最近 10 个版本
    pub fn update_content(&mut self, new_content: String, change_summary: String) {
        self.history.push(SkillVersion {
            version: self.meta.version,
            content: self.content.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            change_summary,
        });
        // 保留最近 10 个版本
        if self.history.len() > 10 {
            self.history.drain(0..self.history.len() - 10);
        }
        self.meta.version += 1;
        self.content = new_content;
    }
}

/// 技能错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillError {
    /// 错误信息
    pub message: String,
}