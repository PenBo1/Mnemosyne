//! 子 Agent 角色配置与工具白名单。
//!
//! 不同角色的子 Agent 拥有不同的工具集。由于 `ToolExecutor` 不是 `Clone` 的，
//! 这里通过「从角色白名单重建 ToolRegistry」实现工具过滤（而非克隆后过滤）。
//! 这与 codex 的 `apply_role_to_tools` 目标一致，但适配了 Rust 的所有权约束。

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::agent::base::{MemorySystem, ToolRegistry};
use crate::core::agent::tools::{ArchiveMemoryTool, BashTool, ListFilesTool, ReadFileTool, SearchMemoryTool, WriteFileTool};

use super::types::SubAgentRole;

/// 角色配置：角色 + 允许的工具 + 系统提示词片段。
pub struct RoleConfig {
    pub role: SubAgentRole,
    /// 该角色的系统提示词片段（拼到子 Agent 的 system prompt 前）。
    pub system_prompt_fragment: &'static str,
}

impl RoleConfig {
    pub fn for_role(role: SubAgentRole) -> Self {
        Self {
            role,
            system_prompt_fragment: system_prompt_fragment(role),
        }
    }
}

/// 返回角色的系统提示词片段。
///
/// 每个角色有简短的身份说明，告诉子 Agent 它的职责和限制。
/// 该片段会拼到 `REACT_DISCIPLINE_ZH` 前面，构成完整的系统提示词。
pub fn system_prompt_fragment(role: SubAgentRole) -> &'static str {
    match role {
        SubAgentRole::Researcher => r#"你是资料收集子 Agent（Researcher）。

## 你的职责
- 搜索和阅读已有资料，为父 Agent 提供信息支撑。
- 你只能读取文件和搜索记忆，**不能写文件或执行命令**。
- 聚焦于收集与任务直接相关的信息，不要发散。
- 输出时条理清晰地列出你找到的关键信息，标注来源（文件路径或记忆条目）。"#,

        SubAgentRole::Outliner => r#"你是大纲生成子 Agent（Outliner）。

## 你的职责
- 基于父 Agent 提供的上下文，生成或修订大纲文件。
- 你可以读写文件，但产出应聚焦在大纲结构上。
- 大纲要有清晰的层级（章 → 节 → 要点），每层信息密度合适。
- 完成后简述你写了哪些文件、大纲的整体结构。"#,

        SubAgentRole::Critic => r#"你是审稿子 Agent（Critic）。

## 你的职责
- 审阅已有内容，给出结构化的评审意见。
- 你只能读取文件和搜索记忆，**不能写文件或执行命令**。
- 评审要具体：指出问题所在（引用原文）、说明问题、给出改进建议。
- 区分「严重问题」（影响可读性/逻辑）和「改进建议」（锦上添花）。"#,

        SubAgentRole::Default => r#"你是通用子 Agent（Default）。

## 你的职责
- 完成父 Agent 委托的特定任务。
- 你继承了父 Agent 的标准工具集（文件读写、命令执行、记忆搜索）。
- 聚焦于完成委托任务，不要扩展到任务范围之外。
- 完成后清晰汇报你做了什么、产出了什么。"#,
    }
}

/// 根据角色白名单构建 ToolRegistry。
///
/// 由于 `ToolExecutor` 不是 `Clone`，这里从角色白名单重建全新的工具实例，
/// 而非从父 Agent 的 ToolRegistry 中过滤。效果等价于 codex 的 `apply_role_to_tools`。
///
/// - 指定白名单的角色：只注册白名单中的工具。
/// - Default 角色：注册全部标准工具，但**不包含 spawn_subagent**（防止递归 spawn）。
pub fn build_role_tool_registry(
    role: SubAgentRole,
    work_dir: PathBuf,
    memory: Arc<RwLock<MemorySystem>>,
) -> ToolRegistry {
    let mut tools = ToolRegistry::new();

    let allowed: Vec<String> = role.allowed_tools().unwrap_or_else(|| {
        vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "list_files".to_string(),
            "bash".to_string(),
            "search_memory".to_string(),
            "archive_memory".to_string(),
        ]
    });

    for name in allowed {
        match name.as_str() {
            "read_file" => {
                tools.register("read_file", Box::new(ReadFileTool::new(work_dir.clone())));
            }
            "write_file" => {
                tools.register("write_file", Box::new(WriteFileTool::new(work_dir.clone())));
            }
            "list_files" => {
                tools.register("list_files", Box::new(ListFilesTool::new(work_dir.clone())));
            }
            "bash" => {
                tools.register("bash", Box::new(BashTool::new(work_dir.clone(), None)));
            }
            "search_memory" => {
                tools.register("search_memory", Box::new(SearchMemoryTool::new(memory.clone())));
            }
            "archive_memory" => {
                tools.register("archive_memory", Box::new(ArchiveMemoryTool::new(memory.clone())));
            }
            unknown => {
                tracing::warn!(tool = %unknown, role = %role, "Unknown tool in role whitelist, skipping");
            }
        }
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_memory() -> Arc<RwLock<MemorySystem>> {
        Arc::new(RwLock::new(MemorySystem::new(20)))
    }

    #[test]
    fn researcher_excludes_write_and_bash() {
        let tools = build_role_tool_registry(SubAgentRole::Researcher, PathBuf::from("/tmp"), make_memory());
        let defs = tools.definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();

        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"search_memory"));
        assert!(names.contains(&"list_files"));
        assert!(!names.contains(&"write_file"));
        assert!(!names.contains(&"bash"));
        assert!(!names.contains(&"spawn_subagent"));
    }

    #[test]
    fn outliner_includes_write() {
        let tools = build_role_tool_registry(SubAgentRole::Outliner, PathBuf::from("/tmp"), make_memory());
        let defs = tools.definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();

        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"read_file"));
        assert!(!names.contains(&"bash"));
    }

    #[test]
    fn default_includes_all_except_spawn() {
        let tools = build_role_tool_registry(SubAgentRole::Default, PathBuf::from("/tmp"), make_memory());
        let defs = tools.definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();

        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"search_memory"));
        assert!(!names.contains(&"spawn_subagent"));
    }

    #[test]
    fn system_prompt_fragment_non_empty() {
        for role in [
            SubAgentRole::Researcher,
            SubAgentRole::Outliner,
            SubAgentRole::Critic,
            SubAgentRole::Default,
        ] {
            let fragment = system_prompt_fragment(role);
            assert!(!fragment.is_empty(), "prompt fragment empty for {:?}", role);
        }
    }
}
