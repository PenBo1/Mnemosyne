//! ═══════════════════════════════════════════════════════════════════════════
//! Compaction - 上下文压缩策略
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 压缩策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionStrategy {
    /// 完全替换
    FullReplace,
    /// 摘要压缩
    Summarize,
    /// 选择性保留
    Selective,
}

/// 压缩策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionPolicy {
    /// 自动压缩阈值百分比
    pub auto_compact_threshold_percent: u32,
    /// 每次会话最大压缩次数
    pub max_compactions_per_session: u32,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            auto_compact_threshold_percent: 85,
            max_compactions_per_session: 3,
        }
    }
}

impl CompactionPolicy {
    /// 创建压缩策略配置
    /// 
    /// # 参数
    /// - `threshold_percent`: 自动压缩阈值百分比
    /// - `max_compactions`: 每次会话最大压缩次数
    pub fn new(threshold_percent: u32, max_compactions: u32) -> Self {
        Self {
            auto_compact_threshold_percent: threshold_percent,
            max_compactions_per_session: max_compactions,
        }
    }
}

/// 判断是否需要压缩上下文
/// 
/// # 参数
/// - `total_tokens`: 当前 token 总数
/// - `context_window`: 上下文窗口大小
/// - `policy`: 压缩策略配置
/// 
/// # 返回值
/// 返回是否需要执行压缩
pub fn should_compact(total_tokens: u64, context_window: u64, policy: &CompactionPolicy) -> bool {
    if context_window == 0 {
        return false;
    }
    let usage_percent = (total_tokens * 100) / context_window;
    usage_percent >= policy.auto_compact_threshold_percent as u64
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = CompactionPolicy::default();
        assert_eq!(policy.auto_compact_threshold_percent, 85);
        assert_eq!(policy.max_compactions_per_session, 3);
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let policy = CompactionPolicy::default();
        let result = should_compact(80_000, 100_000, &policy);
        assert!(!result);
    }

    #[test]
    fn test_should_compact_at_threshold() {
        let policy = CompactionPolicy::default();
        let result = should_compact(85_000, 100_000, &policy);
        assert!(result);
    }

    #[test]
    fn test_should_compact_above_threshold() {
        let policy = CompactionPolicy::default();
        let result = should_compact(90_000, 100_000, &policy);
        assert!(result);
    }

    #[test]
    fn test_should_compact_zero_context_window() {
        let policy = CompactionPolicy::default();
        let result = should_compact(1000, 0, &policy);
        assert!(!result);
    }
}