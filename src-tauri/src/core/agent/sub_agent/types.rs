//! 子 Agent 类型定义。
//!
//! 参考 codex 的 sub-agent 类型设计，适配 Thalia 的角色工具白名单需求。
//! 子 Agent 是由主 Agent 通过 `spawn_subagent` 工具自主 spawn 的轻量 agent，
//! 运行在同进程的 tokio task 中，通过 channel 异步回传结果。

use serde::{Deserialize, Serialize};

/// 子 Agent 角色。
///
/// 不同角色拥有不同的工具白名单，实现最小权限原则。
/// - Researcher / Critic：只读，不能写文件
/// - Outliner：只能写 outline 相关文件（路径由 service 层校验）
/// - Default：继承父 Agent 的标准工具集（减去 spawn_subagent）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubAgentRole {
    /// 资料收集：只能读不能写
    Researcher,
    /// 大纲生成：只能写 outline 文件
    Outliner,
    /// 审稿：只读
    Critic,
    /// 通用：继承父 agent 工具集（减去 spawn_subagent）
    Default,
}

impl SubAgentRole {
    /// 返回该角色允许的工具名列表（None = 继承全部父工具，但排除 spawn_subagent）。
    ///
    /// 工具名与 `crate::core::agent::tools` 中注册的名称一致。
    pub fn allowed_tools(&self) -> Option<Vec<String>> {
        match self {
            SubAgentRole::Researcher => Some(vec![
                "search_memory".to_string(),
                "read_file".to_string(),
                "list_files".to_string(),
            ]),
            SubAgentRole::Outliner => Some(vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "list_files".to_string(),
            ]),
            SubAgentRole::Critic => Some(vec![
                "read_file".to_string(),
                "search_memory".to_string(),
                "list_files".to_string(),
            ]),
            SubAgentRole::Default => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SubAgentRole::Researcher => "researcher",
            SubAgentRole::Outliner => "outliner",
            SubAgentRole::Critic => "critic",
            SubAgentRole::Default => "default",
        }
    }

    /// 从字符串解析角色（用于工具参数解析）。
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "researcher" => Ok(SubAgentRole::Researcher),
            "outliner" => Ok(SubAgentRole::Outliner),
            "critic" => Ok(SubAgentRole::Critic),
            "default" => Ok(SubAgentRole::Default),
            other => Err(format!(
                "Unknown sub-agent role: \"{}\". Must be one of: researcher, outliner, critic, default",
                other
            )),
        }
    }
}

impl std::fmt::Display for SubAgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 子 Agent spawn 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentSpawnRequest {
    pub role: SubAgentRole,
    pub task: String,
    /// 父线程 ID（主 Agent 的 session_id 或上级子 Agent 的 task_id）。
    pub parent_thread_id: String,
    /// 可选附加上下文（拼接到任务描述前）。
    pub context: Option<String>,
}

/// 子 Agent 执行结果。
///
/// 学习 codex 的纯文本返回模式，同时增加结构化字段（artifacts）适配 Thalia 需求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub task_id: String,
    pub role: SubAgentRole,
    pub status: SubAgentStatus,
    /// 最终输出文本（学习 codex 的纯文本返回）。
    pub output: String,
    /// 产出的文件路径列表（从 write_file 工具调用中提取）。
    pub artifacts: Vec<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 子 Agent 执行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubAgentStatus {
    Pending,
    Running,
    Completed,
    Errored,
    Cancelled,
}

impl SubAgentStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            SubAgentStatus::Completed | SubAgentStatus::Errored | SubAgentStatus::Cancelled
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SubAgentStatus::Pending | SubAgentStatus::Running)
    }
}

/// 子 Agent 元信息（注册表条目）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentInfo {
    pub task_id: String,
    pub role: SubAgentRole,
    pub task: String,
    pub status: SubAgentStatus,
    pub parent_thread_id: String,
    /// 在 spawn 树中的深度（主 Agent 直接 spawn 的子 Agent depth=1）。
    pub depth: u32,
    /// ISO 8601 启动时间。
    pub started_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_allowed_tools_researcher_excludes_write() {
        let tools = SubAgentRole::Researcher.allowed_tools().unwrap();
        assert!(tools.contains(&"read_file".to_string()));
        assert!(!tools.contains(&"write_file".to_string()));
    }

    #[test]
    fn role_allowed_tools_outliner_includes_write() {
        let tools = SubAgentRole::Outliner.allowed_tools().unwrap();
        assert!(tools.contains(&"write_file".to_string()));
    }

    #[test]
    fn role_allowed_tools_default_is_none() {
        assert!(SubAgentRole::Default.allowed_tools().is_none());
    }

    #[test]
    fn role_from_str_valid() {
        assert_eq!(SubAgentRole::from_str("researcher").unwrap(), SubAgentRole::Researcher);
        assert_eq!(SubAgentRole::from_str("Critic").unwrap(), SubAgentRole::Critic);
    }

    #[test]
    fn role_from_str_invalid() {
        assert!(SubAgentRole::from_str("nonexistent").is_err());
    }

    #[test]
    fn status_is_final() {
        assert!(SubAgentStatus::Completed.is_final());
        assert!(SubAgentStatus::Errored.is_final());
        assert!(SubAgentStatus::Cancelled.is_final());
        assert!(!SubAgentStatus::Running.is_final());
        assert!(!SubAgentStatus::Pending.is_final());
    }

    #[test]
    fn status_is_active() {
        assert!(SubAgentStatus::Running.is_active());
        assert!(SubAgentStatus::Pending.is_active());
        assert!(!SubAgentStatus::Completed.is_active());
    }
}
