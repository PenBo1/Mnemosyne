
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

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


// ── P2.2 memory-retrieval 加速层 DTO ──────────────────────────
//
// 前端 retrieveMemorySelection 原本并行读 7 个 markdown/JSON 文件（无 SQLite 加速）。
// P2.2 新增 memory_retrieve_selection IPC 命令，通过 story_facts / chapter_summaries
// 表（已有 valid_from_chapter / valid_until_chapter 时序索引）批量查询。
//
// 以下 DTO 使用 camelCase 序列化，与前端 Fact / StoredSummary 类型字段名对齐。
// 不直接复用 shared::story::models::StoryFact（其序列化为 snake_case，且
// ChapterSummary.characters 为 Vec<String>，前端 StoredSummary.characters 为 String）。

/// 记忆检索请求 —— 前端 camelCase 调用。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRetrievalRequest {
    /// 书籍 ID（对应 story_facts.novel_id / memory_entries.book_id）
    pub book_id: String,
    /// 当前章节号（用于 facts 时序过滤和 summaries 近期窗口）
    pub chapter_number: Option<i64>,
    /// 当前章节目标（保留扩展位，当前未用于 SQL 过滤）
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default = "default_true")]
    pub include_facts: bool,
    #[serde(default = "default_true")]
    pub include_hooks: bool,
    #[serde(default = "default_true")]
    pub include_summaries: bool,
    #[serde(default = "default_true")]
    pub include_volume_summaries: bool,
    /// 每类最多返回数，默认 20
    #[serde(default = "default_max_items")]
    pub max_items_per_category: i64,
}

fn default_true() -> bool { true }
fn default_max_items() -> i64 { 20 }

/// 检索结果 —— 直接对前端 MemorySelection 部分字段赋值。
///
/// hooks / volumeSummaries 目前无 SQLite 表（hooks 存于 state.json，
/// volume_summaries 仅 markdown），故这两个字段在 IPC 路径下返回空数组，
/// 前端对这两个类别自动降级到 markdown 读取。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRetrievalResult {
    pub facts: Vec<RetrievedFact>,
    pub summaries: Vec<RetrievedSummary>,
    pub has_data: bool,
}

/// 事实 DTO —— 字段名与前端 Fact 接口对齐（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedFact {
    pub id: Option<i64>,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from_chapter: i64,
    pub valid_until_chapter: Option<i64>,
    pub source_chapter: i64,
}

/// 章节摘要 DTO —— characters/events 等前端期望 String（join 后），
/// 非 Vec<String>。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedSummary {
    pub chapter: i64,
    pub title: String,
    pub characters: String,
    pub events: String,
    pub state_changes: String,
    pub hook_activity: String,
    pub mood: String,
    pub chapter_type: String,
}

impl RetrievedFact {
    pub fn from_story_fact(f: &crate::shared::story::models::StoryFact) -> Self {
        Self {
            id: None,
            subject: f.subject.clone(),
            predicate: f.predicate.clone(),
            object: f.object.clone(),
            valid_from_chapter: f.valid_from_chapter as i64,
            valid_until_chapter: f.valid_until_chapter.map(|v| v as i64),
            source_chapter: f.source_chapter as i64,
        }
    }
}

impl RetrievedSummary {
    pub fn from_chapter_summary(s: &crate::shared::story::models::ChapterSummary) -> Self {
        Self {
            chapter: s.chapter as i64,
            title: s.title.clone(),
            characters: s.characters.join(", "),
            events: s.events.join("; "),
            state_changes: s.state_changes.join("; "),
            hook_activity: s.hook_activity.join("; "),
            mood: s.mood.clone(),
            chapter_type: s.chapter_type.clone(),
        }
    }
}