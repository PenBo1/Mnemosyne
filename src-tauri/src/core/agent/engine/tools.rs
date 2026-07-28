//! ═══════════════════════════════════════════════════════════════════════════
//! Tools - Agent 工具注册
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 工具注册：为单次 `send_message` 调用构建 `Vec<Arc<dyn Tool>>` 工具集，
//! 按 `EffortLevel` 分档注入，并支持可选白名单筛选。
//!
//! 从 engine.rs 拆分，避免编排模块耦合具体工具构造器。

use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::Arc;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::llm::tool::Tool;

use crate::core::agent::approval::ApprovalManager;
use crate::core::agent::effort::EffortLevel;
use crate::core::agent::subagent::SubAgentTool;
use crate::core::agent::tools::todo_tools::TodoWriteTool;
use crate::core::agent::tools::{
    CreateDirectoryTool, EditTool, ListDirectoryTool, McpCallTool, McpListToolsTool,
    MultiEditTool, ReadFileTool, WriteFileTool,
};
use crate::core::agent::tools::{
    GenerateCoverTool, GrepTool, ImportChaptersTool, IngestMaterialTool, LsTool,
    PatchChapterTextTool, PipelineDelegateTool, ProposeActionTool, RenameEntityTool,
    ReplaceChapterTextTool, ResearchWebTool, RetrieveMaterialTool, WriteTruthFileTool,
};

use super::AgentEngine;

/// 构造工具集:按 EffortLevel 分档注入工具,并应用可选 tool_whitelist 交集筛选。
///
/// 分档策略:
/// - Low:3 个只读工具(read_file / list_directory / todo_write),适合简单问答
/// - Medium:15 个 base tools,不含 subagent-gated 工具
/// - High / Ultra:全部 23 个工具(base + 8 个 subagent-gated)
///
/// 注意:Medium 的 `effort_params.allow_subagent` 虽为 true,但 spec 要求 Medium
/// 不含 subagent-gated 工具,故此处按 EffortLevel 枚举直接判断,而非依赖 allow_subagent。
/// `effort_params` 仍由调用方独立使用(max_tool_steps / max_tokens_per_call)。
///
/// tool_whitelist 为 None 时返回 effort 分档后的完整工具集;
/// Some 时按工具名二次过滤,与 effort 分档结果取交集。
///
/// 返回 Vec<Arc<dyn Tool>>,供 MultiTurnRunner 使用。
pub(super) fn build_tools(
    workspace_root: &Path,
    approval: &Arc<ApprovalManager>,
    effort: EffortLevel,
    tool_whitelist: Option<&[String]>,
    data_dir: &DataDir,
    engine: &Arc<AgentEngine>,
) -> Vec<Arc<dyn Tool>> {
    tracing::debug!(
        effort = %effort.as_str(),
        has_whitelist = tool_whitelist.is_some(),
        "[agent] build_tools: registering tools"
    );

    // ── 按 EffortLevel 分档注入工具 ──
    let mut tools: Vec<Arc<dyn Tool>> = if matches!(effort, EffortLevel::Low) {
        // Low:仅 3 个只读工具
        vec![
            Arc::new(ReadFileTool {
                workspace_root: workspace_root.to_path_buf(),
            }),
            Arc::new(ListDirectoryTool {
                workspace_root: workspace_root.to_path_buf(),
            }),
            Arc::new(TodoWriteTool),
        ]
    } else {
        // Medium / High / Ultra 共享 15 个 base 工具
        let mut base: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadFileTool {
                workspace_root: workspace_root.to_path_buf(),
            }),
            Arc::new(ListDirectoryTool {
                workspace_root: workspace_root.to_path_buf(),
            }),
            Arc::new(WriteFileTool {
                workspace_root: workspace_root.to_path_buf(),
                approval: approval.clone(),
            }),
            Arc::new(CreateDirectoryTool {
                workspace_root: workspace_root.to_path_buf(),
                approval: approval.clone(),
            }),
            Arc::new(EditTool {
                workspace_root: workspace_root.to_path_buf(),
                approval: approval.clone(),
            }),
            Arc::new(MultiEditTool {
                workspace_root: workspace_root.to_path_buf(),
                approval: approval.clone(),
            }),
            Arc::new(TodoWriteTool),
            Arc::new(GrepTool { data_dir: data_dir.clone() }),
            Arc::new(LsTool { data_dir: data_dir.clone() }),
            Arc::new(ResearchWebTool {
                engine: engine.clone(),
                research_ops: engine.research_ops.clone(),
            }),
            Arc::new(IngestMaterialTool {
                research_ops: engine.research_ops.clone(),
            }),
            Arc::new(RetrieveMaterialTool {
                research_ops: engine.research_ops.clone(),
            }),
            Arc::new(ProposeActionTool),
            Arc::new(McpCallTool),
            Arc::new(McpListToolsTool),
        ];
        // High / Ultra 追加 8 个 subagent-gated 工具
        if matches!(effort, EffortLevel::High | EffortLevel::Ultra) {
            tracing::debug!(
                "[agent] build_tools: registering subagent-gated tools (effort=high/ultra)"
            );
            base.push(Arc::new(SubAgentTool {
                engine: engine.clone(),
            }));
            base.push(Arc::new(PipelineDelegateTool {
                engine: engine.clone(),
                data_dir: data_dir.clone(),
                approval: approval.clone(),
                pipeline_ops: engine.pipeline_delegate_ops.clone(),
            }));
            base.push(Arc::new(WriteTruthFileTool {
                approval: approval.clone(),
                edit_ops: engine.book_edit_ops.clone(),
            }));
            base.push(Arc::new(RenameEntityTool {
                approval: approval.clone(),
                edit_ops: engine.book_edit_ops.clone(),
            }));
            base.push(Arc::new(PatchChapterTextTool {
                approval: approval.clone(),
                edit_ops: engine.book_edit_ops.clone(),
            }));
            base.push(Arc::new(ReplaceChapterTextTool {
                approval: approval.clone(),
                edit_ops: engine.book_edit_ops.clone(),
            }));
            base.push(Arc::new(ImportChaptersTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            }));
            base.push(Arc::new(GenerateCoverTool {
                data_dir: data_dir.clone(),
                approval: approval.clone(),
            }));
        } else {
            tracing::debug!("[agent] build_tools: skipping subagent-gated tools (effort=medium)");
        }
        base
    };

    tracing::debug!(
        effort = %effort.as_str(),
        count_before_whitelist = tools.len(),
        "[agent] build_tools: tools registered by effort"
    );

    // ── 应用 tool_whitelist 交集筛选 ──
    // None 时返回 effort 分档后的完整工具集;
    // Some 时按工具名二次过滤,仅保留白名单内工具。
    if let Some(whitelist) = tool_whitelist {
        let before = tools.len();
        tools.retain(|t| whitelist.iter().any(|w| w == t.name()));
        tracing::debug!(
            before,
            after = tools.len(),
            "[agent] build_tools: applied tool_whitelist filter"
        );
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::core::agent::types::ChatEvent;

    // ── build_tools 单元测试 ──
    // 验证按 EffortLevel 分档注入工具数量,以及 tool_whitelist 交集筛选。

    /// 构造测试用的 AgentEngine fixture。
    /// 使用 ProviderRegistry::empty() + 内存数据库 + 临时 DataDir。
    fn build_tools_fixture_engine() -> Arc<AgentEngine> {
        use crate::infrastructure::db::connection::Database;
        use crate::infrastructure::llm::registry::ProviderRegistry;
        let registry = ProviderRegistry::empty();
        let db = Database::connect_in_memory().expect("in-memory db");
        let tmp = tempfile::tempdir().expect("tempdir for data_dir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        // tmp 路径泄漏是有意的:测试期间 DataDir 需保持有效,本测试不写盘
        std::mem::forget(tmp);
        Arc::new(AgentEngine::new(
            registry,
            db,
            data_dir,
            PathBuf::new(),
            None,
        ))
    }

    /// 构造测试用 ApprovalManager(用 mpsc channel 的 sender,不接收消息)。
    fn build_tools_fixture_approval() -> Arc<ApprovalManager> {
        let (tx, _rx) = mpsc::channel::<ChatEvent>(8);
        // ApprovalManager::new 接管 sender;_rx 未接收,测试中不会触发审批回调
        ApprovalManager::new(tx)
    }

    #[test]
    fn build_tools_low_returns_three_readonly_tools() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let tools = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::Low,
            None,
            &data_dir,
            &engine,
        );

        assert_eq!(tools.len(), 3, "Low effort 应注入恰好 3 个只读工具");
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"read_file"), "应含 read_file");
        assert!(names.contains(&"list_directory"), "应含 list_directory");
        assert!(names.contains(&"todo_write"), "应含 todo_write");
        // Low 不应含任何写工具或 subagent-gated 工具
        assert!(!names.contains(&"write_file"), "Low 不应含 write_file");
        assert!(!names.contains(&"subagent"), "Low 不应含 subagent");
        assert!(!names.contains(&"pipeline_delegate"), "Low 不应含 pipeline_delegate");
    }

    #[test]
    fn build_tools_medium_returns_fifteen_base_tools() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let tools = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::Medium,
            None,
            &data_dir,
            &engine,
        );

        assert_eq!(tools.len(), 15, "Medium effort 应注入 15 个 base 工具");
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        // 验证 base 工具齐全
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit"));
        assert!(names.contains(&"grep"));
        assert!(names.contains(&"research_web"));
        assert!(names.contains(&"mcp_call"));
        // Medium 不应含 subagent-gated 工具
        assert!(!names.contains(&"subagent"), "Medium 不应含 subagent");
        assert!(!names.contains(&"pipeline_delegate"), "Medium 不应含 pipeline_delegate");
        assert!(!names.contains(&"generate_cover"), "Medium 不应含 generate_cover");
        assert!(!names.contains(&"import_chapters"), "Medium 不应含 import_chapters");
    }

    #[test]
    fn build_tools_high_returns_twenty_three_tools() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let tools = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::High,
            None,
            &data_dir,
            &engine,
        );

        assert_eq!(tools.len(), 23, "High effort 应注入全部 23 个工具");
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        // 验证 subagent-gated 工具齐全
        assert!(names.contains(&"subagent"), "High 应含 subagent");
        assert!(names.contains(&"pipeline_delegate"), "High 应含 pipeline_delegate");
        assert!(names.contains(&"write_truth_file"), "High 应含 write_truth_file");
        assert!(names.contains(&"generate_cover"), "High 应含 generate_cover");
        assert!(names.contains(&"import_chapters"), "High 应含 import_chapters");
    }

    #[test]
    fn build_tools_ultra_returns_twenty_three_tools() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let tools = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::Ultra,
            None,
            &data_dir,
            &engine,
        );

        assert_eq!(tools.len(), 23, "Ultra effort 应与 High 一样注入 23 个工具");
    }

    #[test]
    fn build_tools_whitelist_intersects_with_effort() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        // whitelist 仅含 read_file + todo_write;High 本应 23 个,交集后只剩 2 个
        let whitelist = vec!["read_file".to_string(), "todo_write".to_string()];
        let tools = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::High,
            Some(&whitelist),
            &data_dir,
            &engine,
        );

        assert_eq!(tools.len(), 2, "whitelist 应与 High 工具集取交集,剩 2 个");
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"todo_write"));
    }

    #[test]
    fn build_tools_whitelist_with_names_not_in_effort_returns_empty() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        // whitelist 仅含 subagent,但 Low 不含 subagent → 交集为空
        let whitelist = vec!["subagent".to_string()];
        let tools = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::Low,
            Some(&whitelist),
            &data_dir,
            &engine,
        );

        assert_eq!(tools.len(), 0, "Low 不含 subagent,交集应为空");
    }

    #[test]
    fn build_tools_whitelist_none_returns_full_effort_set() {
        let workspace_root = PathBuf::from("/tmp/test-ws");
        let approval = build_tools_fixture_approval();
        let engine = build_tools_fixture_engine();
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let tools_none = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::Medium,
            None,
            &data_dir,
            &engine,
        );
        let tools_some_empty = build_tools(
            &workspace_root,
            &approval,
            EffortLevel::Medium,
            Some(&[]),
            &data_dir,
            &engine,
        );

        // None:不过滤,返回完整 Medium 工具集(15)
        assert_eq!(tools_none.len(), 15, "None 应返回 Medium 完整工具集");
        // 空 whitelist:交集为空
        assert_eq!(
            tools_some_empty.len(),
            0,
            "空 whitelist 应使交集为空"
        );
    }
}
