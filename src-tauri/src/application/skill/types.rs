
//! ═══════════════════════════════════════════════════════════════════════════
//! Types - 技能类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 技能作用域 —— 对照 codex `SkillScope` 四层模型。
///
/// 决定技能的来源权威与可见性，由扫描目录决定（不从 frontmatter 解析）：
/// - System：随应用分发的内置技能
/// - Admin：系统级管理员安装的技能
/// - User：用户个人技能目录（`~/.mnemosyne/skills/`）
/// - Repo：项目/工作区级别技能
///
/// 扫描顺序：System → Admin → User → Repo（前层覆盖后层同名技能）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillScope {
    System,
    Admin,
    User,
    Repo,
}

impl Default for SkillScope {
    fn default() -> Self {
        Self::User
    }
}

impl SkillScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Admin => "admin",
            Self::User => "user",
            Self::Repo => "repo",
        }
    }
}

/// 技能扩展元数据 —— 对照 codex `SkillMetadata`。
///
/// 用于技能目录展示与简短描述，避免在 available skills 列表中
/// 拼接完整 `description`（可能很长）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// 简短描述（一句话），用于技能目录展示
    #[serde(default)]
    pub short_description: Option<String>,
}

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
    /// 需要的工具列表（对照 codex `dependencies.tools`）
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
    /// 扩展元数据（对照 codex `metadata.short_description`）
    #[serde(default)]
    pub metadata: Option<SkillMetadata>,
    /// 安全策略声明（对照 codex `policy`，如 "read-only" / "sandboxed"）
    #[serde(default)]
    pub policy: Option<String>,
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
    /// 作用域（由扫描目录决定，不从 frontmatter 解析；默认 User）
    #[serde(default)]
    pub scope: SkillScope,
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