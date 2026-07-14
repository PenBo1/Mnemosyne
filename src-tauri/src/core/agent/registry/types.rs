// 统一 Agent 类型定义 —— 跨 Main / Pipeline / SubAgent / LoopSkill 四类。
//
// 设计动机(AGENTS.md):
// - 前端 Persona(coder/architect/...)、后端 SubAgentRole、pipeline agents、loop skill
//   四者此前无统一抽象，IPC 调用方需要分别知道每类的入口
// - 本注册表提供统一 AgentDescriptor，便于仪表盘/调度器/权限系统按统一模型查询
//
// 轻量原则:
// - 不强制重构现有 agent 实现（pipeline agents 仍在 domain/pipeline/agents/ 各自实现）
// - 不接管 agent 执行逻辑（仍由 AgentEngine / PipelineRunner / SubAgentExecutor 负责）
// - 仅作为元数据注册表，描述 "有哪些 agent、各自什么角色、用什么工具"

use serde::{Deserialize, Serialize};

/// Agent 大类。
///
/// - Main:      主对话 agent（与用户直接交互，身份文件 agents/main/）
/// - Pipeline:  小说创作 pipeline 的 15 个阶段 agent
/// - SubAgent:  任务委派 sub-agent（Researcher/Outliner/Critic，由 main agent 调用）
/// - LoopSkill: loop-engineering 的 skill agent（loop_triage/loop_verifier/minimal_fix）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentCategory {
    Main,
    Pipeline,
    SubAgent,
    LoopSkill,
}

impl AgentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentCategory::Main => "main",
            AgentCategory::Pipeline => "pipeline",
            AgentCategory::SubAgent => "subagent",
            AgentCategory::LoopSkill => "loopskill",
        }
    }

    /// 从字符串解析类别。未知值返回 Err。
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "main" => Ok(AgentCategory::Main),
            "pipeline" => Ok(AgentCategory::Pipeline),
            "subagent" => Ok(AgentCategory::SubAgent),
            "loopskill" => Ok(AgentCategory::LoopSkill),
            _ => Err(format!(
                "Unknown agent category '{}', expected one of: main/pipeline/subagent/loopskill",
                s
            )),
        }
    }
}

/// Agent 描述符 —— 注册表中每条记录的统一形态。
///
/// `role` 字段对应身份文件目录名（agents/<role>/），与 prompts::ALL_ROLES 对齐。
/// `system_prompt` 为 None 时，表示运行时从身份文件 + 内嵌 prompt 组装。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDescriptor {
    /// 唯一 ID（如 "main", "planner", "researcher", "loop_triage"）
    pub id: String,
    /// 显示名（如 "Planner", "Researcher"）
    pub name: String,
    pub category: AgentCategory,
    /// 角色标识（用于身份文件加载，对应 agents/<role>/ 目录）
    pub role: String,
    /// 简短描述
    pub description: String,
    /// 内置 system prompt（如果有）；None 表示运行时组装
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// 工具白名单；None = 全工具集，Some = 仅这些工具
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_whitelist: Option<Vec<String>>,
    /// 模型覆盖；None = 使用全局 active model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    /// 是否能 spawn sub-agent（仅 main agent 为 true）
    pub can_spawn_subagent: bool,
}
