//! ═══════════════════════════════════════════════════════════════════════════
//! 故事类型 - 故事相关类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 故事错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryError {
    /// 错误信息
    pub message: String,
}

/// 默认字数统计函数
///
/// 统计文本中的单词数量，过滤纯标点符号
pub fn count_words_default(content: &str) -> u32 {
    content
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
        .count() as u32
}