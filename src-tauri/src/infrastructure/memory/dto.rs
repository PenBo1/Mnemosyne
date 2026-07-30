//! ═══════════════════════════════════════════════════════════════════════════
//! Memory DTO - 传输对象定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 从 types.rs 和 commands 中提取的传输对象（DTO），
//! 用于 IPC 响应和前端数据交换。

use serde::{Deserialize, Serialize};

// ── 短期记忆统计 ────────────────────────────────────────────────────────────

/// 短期记忆统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryStats {
    /// 总记录数
    pub total: u64,
    /// 今日记录数
    pub today_count: u64,
    /// 最近 7 天记录数
    pub last_7_days_count: u64,
}

// ── 记忆检索请求（从 types.rs 移入）─────────────────────────────────────────

/// 记忆检索请求 —— 前端 camelCase 调用。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRetrievalRequest {
    /// 书籍 ID
    pub book_id: String,
    /// 当前章节号
    pub chapter_number: Option<i64>,
    /// 当前章节目标
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
    /// 每类最多返回数
    #[serde(default = "default_max_items")]
    pub max_items_per_category: i64,
}

fn default_true() -> bool { true }
fn default_max_items() -> i64 { 20 }

// ── 记忆检索结果 ────────────────────────────────────────────────────────────

/// 检索结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRetrievalResult {
    pub facts: Vec<RetrievedFact>,
    pub summaries: Vec<RetrievedSummary>,
    pub has_data: bool,
}

/// 事实 DTO
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

/// 章节摘要 DTO
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

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_term_memory_stats_default() {
        let stats = ShortTermMemoryStats {
            total: 0,
            today_count: 0,
            last_7_days_count: 0,
        };
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_memory_retrieval_request_defaults() {
        let req = MemoryRetrievalRequest {
            book_id: "test".to_string(),
            chapter_number: None,
            goal: None,
            include_facts: true,
            include_hooks: true,
            include_summaries: true,
            include_volume_summaries: true,
            max_items_per_category: 20,
        };
        assert!(req.include_facts);
        assert_eq!(req.max_items_per_category, 20);
    }
}