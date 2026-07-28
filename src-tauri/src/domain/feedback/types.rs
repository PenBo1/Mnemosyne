//! ═══════════════════════════════════════════════════════════════════════════
//! 反馈类型 - 反馈模块类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ── 类型定义 ────────────────────────────────────────────────────────────────

/// 错误事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// 事件 ID
    pub id: String,
    /// Agent 名称
    pub agent: String,
    /// 错误类型
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 关联章节
    pub chapter: Option<u32>,
    /// 关联书籍 ID
    pub book_id: Option<String>,
    /// 时间戳
    pub timestamp: String,
    /// 严重程度
    pub severity: Severity,
}

/// 严重程度分级
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// 警告
    Warning,
    /// 严重
    Critical,
}

/// 约束教训记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLesson {
    /// 教训 ID
    pub id: String,
    /// 约束规则
    pub rule: String,
    /// 规则原因
    pub reason: String,
    /// 来源错误 ID 列表
    pub source_errors: Vec<String>,
    /// 是否激活
    pub active: bool,
    /// 创建时间
    pub created_at: String,
}

/// 反馈规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRules {
    /// 警告阈值
    pub warning_threshold: usize,
    /// 严重阈值
    pub critical_threshold: usize,
    /// 最大活跃 Lessons 数量
    pub max_active_lessons: usize,
}

impl Default for FeedbackRules {
    fn default() -> Self {
        Self {
            warning_threshold: 5,
            critical_threshold: 2,
            max_active_lessons: 10,
        }
    }
}