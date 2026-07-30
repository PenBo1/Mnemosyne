//! ═══════════════════════════════════════════════════════════════════════════
//! 记忆类型 - 领域模型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use crate::shared::error::AppError;

// ── 记忆类型分类 ────────────────────────────────────────────────────────────

/// 记忆类型分类
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MemoryType {
    /// 事实（客观信息）
    #[default]
    Fact,
    /// 上下文（背景信息）
    Context,
    /// 偏好（用户偏好）
    Preference,
    /// 教训（经验教训）
    Lesson,
    /// 对话（对话记录）
    Conversation,
    /// 角色（人物信息）
    Character,
    /// 剧情（情节信息）
    Plot,
    /// 设定（世界观设定）
    Setting,
    /// 对话风格
    Dialogue,
    /// 写作风格
    Style,
    /// 研究资料
    Research,
    /// 通用记忆
    General,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Fact => write!(f, "fact"),
            MemoryType::Context => write!(f, "context"),
            MemoryType::Preference => write!(f, "preference"),
            MemoryType::Lesson => write!(f, "lesson"),
            MemoryType::Conversation => write!(f, "conversation"),
            MemoryType::Character => write!(f, "character"),
            MemoryType::Plot => write!(f, "plot"),
            MemoryType::Setting => write!(f, "setting"),
            MemoryType::Dialogue => write!(f, "dialogue"),
            MemoryType::Style => write!(f, "style"),
            MemoryType::Research => write!(f, "research"),
            MemoryType::General => write!(f, "general"),
        }
    }
}

// ── 记忆条目 ────────────────────────────────────────────────────────────────

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 条目 ID
    pub id: String,
    /// 键名（用于检索）
    pub key: String,
    /// 值（记忆内容）
    pub value: String,
    /// 记忆类型
    pub memory_type: MemoryType,
    /// 来源（Agent/用户/系统）
    pub source: String,
    /// 重要性评分（0-100）
    pub importance: u32,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 详细内容
    pub content: Option<String>,
    /// 条目类型
    pub entry_type: Option<String>,
    /// 关联章节
    pub chapter: Option<String>,
    /// 时间戳
    pub timestamp: Option<String>,
    /// 标签列表
    pub tags: Option<Vec<String>>,
}

// ── 记忆系统 ────────────────────────────────────────────────────────────────

/// 记忆系统
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySystem {
    /// 所有记忆条目
    pub entries: Vec<MemoryEntry>,
    /// Token 预算上限
    pub budget: u32,
}

impl MemorySystem {
    /// 创建记忆系统实例
    pub fn new(budget: u32) -> Self {
        Self {
            entries: Vec::new(),
            budget,
        }
    }

    /// 获取所有条目
    pub fn get_all_entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    /// 归档条目（降低重要性为 0）
    pub fn archive(&mut self, entry_id: &str) -> Result<(), AppError> {
        let idx = self.entries.iter().position(|e| e.id == entry_id);
        if let Some(i) = idx {
            self.entries[i].importance = 0;
            Ok(())
        } else {
            Err(AppError::not_found(format!("Entry {} not found", entry_id)))
        }
    }

    /// 删除条目
    pub fn delete_entry(&mut self, entry_id: &str) -> Result<(), AppError> {
        let idx = self.entries.iter().position(|e| e.id == entry_id);
        if let Some(i) = idx {
            self.entries.remove(i);
            Ok(())
        } else {
            Err(AppError::not_found(format!("Entry {} not found", entry_id)))
        }
    }

    /// 更新条目内容
    pub fn update_entry(&mut self, entry_id: &str, content: &str) -> Result<(), AppError> {
        let idx = self.entries.iter().position(|e| e.id == entry_id);
        if let Some(i) = idx {
            self.entries[i].value = content.to_string();
            Ok(())
        } else {
            Err(AppError::not_found(format!("Entry {} not found", entry_id)))
        }
    }

    /// 格式化主上下文（用于 Prompt 注入）
    pub fn format_main_context(&self) -> String {
        self.entries.iter()
            .filter(|e| e.importance > 0)
            .map(|e| format!("{}: {}", e.key, e.value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}