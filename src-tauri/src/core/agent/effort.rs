//! ═══════════════════════════════════════════════════════════════════════════
//! Effort - Agent 投入程度控制
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 核心洞察:
//! - Model 决定 "能力/知识" (会不会), Effort 决定 "态度/投入" (愿不愿意努力做)
//! - 小模型 + 高 Effort 可能比大模型 + 低 Effort 效果更好
//! - Effort 不是 "多想几秒", 而是控制 Agent 的行为投入程度:
//!   * 低 Effort: 快速回复, 多问用户, 少动手, 少读文件
//!   * 高 Effort: 多读文件、跑测试、反复验证、把长任务链跑完

use serde::{Deserialize, Serialize};

/// Effort 级别枚举
/// 
/// 用户可通过 settings 选择, 也可在每次 ChatRequest 中覆盖。
/// 默认为 Medium。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffortLevel {
    /// 低投入: 快速回复, 最小工具调用, 适合简单问答
    Low,
    /// 中等投入: 默认, 平衡速度与质量
    Medium,
    /// 高投入: 深度分析, 多轮验证, 适合复杂任务
    High,
    /// 超高投入: 多 agent 并行, 适合超大型任务
    Ultra,
}

impl EffortLevel {
    /// 获取级别的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }
}

impl std::str::FromStr for EffortLevel {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" | "med" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "ultra" | "ultracode" => Ok(Self::Ultra),
            _ => Err(format!("Unknown EffortLevel '{}'", s)),
        }
    }
}

impl EffortLevel {
    /// 获取该 Effort 级别对应的行为参数
    /// 
    /// 这些参数由 AgentEngine 在构建 agent 时读取, 影响:
    /// - MultiTurnRunner 的 max_turns 上限
    /// - 工具调用循环的退出条件
    /// - sub-agent spawn 权限
    pub fn params(&self) -> EffortParams {
        match self {
            // Low: 5 轮工具, 10 文件, 不验证, 2k tokens, 无 subagent
            Self::Low => EffortParams {
                max_tool_steps: 5,
                max_files_to_read: 10,
                max_test_runs: 0,
                verify_rounds: 0,
                max_tokens_per_call: 2_048,
                allow_subagent: false,
                allow_parallel: false,
            },
            // Medium: 20 轮工具, 50 文件, 1 次测试, 8k tokens, 允许 subagent
            Self::Medium => EffortParams {
                max_tool_steps: 20,
                max_files_to_read: 50,
                max_test_runs: 1,
                verify_rounds: 1,
                max_tokens_per_call: 8_192,
                allow_subagent: true,
                allow_parallel: false,
            },
            // High: 50 轮工具, 200 文件, 3 次测试, 16k tokens, 允许 subagent
            Self::High => EffortParams {
                max_tool_steps: 50,
                max_files_to_read: 200,
                max_test_runs: 3,
                verify_rounds: 2,
                max_tokens_per_call: 16_384,
                allow_subagent: true,
                allow_parallel: false,
            },
            // Ultra: 100 轮工具, 无上限, 5 次测试, 32k tokens, 允许并行
            Self::Ultra => EffortParams {
                max_tool_steps: 100,
                max_files_to_read: 1_000,
                max_test_runs: 5,
                verify_rounds: 3,
                max_tokens_per_call: 32_768,
                allow_subagent: true,
                allow_parallel: true,
            },
        }
    }
}

impl Default for EffortLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for EffortLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Effort 级别对应的行为参数
/// 
/// 控制 Agent 在一次对话中的 "投入上限"。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EffortParams {
    /// 工具调用循环最大轮次
    pub max_tool_steps: usize,
    /// 单次任务读取文件上限
    pub max_files_to_read: u32,
    /// 测试执行次数上限 (0 = 不跑测试)
    pub max_test_runs: u32,
    /// 验证轮次 (0 = 不验证, 1 = 单次, 2+ = 多次交叉验证)
    pub verify_rounds: u32,
    /// 单次 LLM 调用 token 上限
    pub max_tokens_per_call: u64,
    /// 是否允许 spawn sub-agent
    pub allow_subagent: bool,
    /// 是否允许并行 agent (仅 Ultra)
    pub allow_parallel: bool,
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effort_str_roundtrip() {
        for &e in &[EffortLevel::Low, EffortLevel::Medium, EffortLevel::High, EffortLevel::Ultra] {
            assert_eq!(e.as_str().parse::<EffortLevel>(), Ok(e));
        }
        assert_eq!("ultracode".parse::<EffortLevel>(), Ok(EffortLevel::Ultra));
        assert!("unknown".parse::<EffortLevel>().is_err());
    }

    #[test]
    fn default_is_medium() {
        assert_eq!(EffortLevel::default(), EffortLevel::Medium);
    }

    #[test]
    fn low_params_minimal() {
        let p = EffortLevel::Low.params();
        assert_eq!(p.max_tool_steps, 5);
        assert_eq!(p.max_test_runs, 0);
        assert_eq!(p.verify_rounds, 0);
        assert!(!p.allow_subagent);
        assert!(!p.allow_parallel);
    }

    #[test]
    fn ultra_params_maximal() {
        let p = EffortLevel::Ultra.params();
        assert_eq!(p.max_tool_steps, 100);
        assert_eq!(p.max_test_runs, 5);
        assert_eq!(p.verify_rounds, 3);
        assert!(p.allow_subagent);
        assert!(p.allow_parallel);
    }

    #[test]
    fn params_monotonic_increasing() {
        let low = EffortLevel::Low.params();
        let med = EffortLevel::Medium.params();
        let high = EffortLevel::High.params();
        let ultra = EffortLevel::Ultra.params();
        assert!(med.max_tool_steps >= low.max_tool_steps);
        assert!(high.max_tool_steps >= med.max_tool_steps);
        assert!(ultra.max_tool_steps >= high.max_tool_steps);
        assert!(ultra.max_tokens_per_call >= high.max_tokens_per_call);
    }
}