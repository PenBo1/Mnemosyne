//! ═══════════════════════════════════════════════════════════════════════════
//! Tools - 工具模块
//! ═══════════════════════════════════════════════════════════════════════════

pub mod registry;
pub mod requirements;
pub mod fs_tools;
pub mod edit_tools;
pub mod todo_tools;
pub mod book_tools;
pub mod book_ops;
pub mod research_tools;
pub mod search_tools;
pub mod mcp_tools;
pub mod truncation; // Stage E3：tool 输出截断策略

pub use registry::{
    register, get, get_handler, get_schema, list_tools, get_tool_schemas,
    check_availability, execute, toolset_for_tool, list_toolsets,
    ToolMeta, ToolSchema, ToolError, ToolHandler, HERMES_CORE_TOOLS, HERMES_OPTIONAL_TOOLSETS,
};

pub use requirements::{
    ToolKind, ToolId, Expr, ParamCondition, ToolParamsRequirement,
    ToolRequirement, RequirementChecker, DefaultRequirementChecker,
};

pub use fs_tools::{ReadFileTool, ListDirectoryTool, WriteFileTool, CreateDirectoryTool};
pub use edit_tools::{EditTool, MultiEditTool};
pub use todo_tools::TodoWriteTool;
pub use book_tools::{
    GenerateCoverTool, ImportChaptersTool, PatchChapterTextTool, PipelineDelegateTool,
    ProposeActionTool, RenameEntityTool, ReplaceChapterTextTool, WriteTruthFileTool,
};
pub use research_tools::{IngestMaterialTool, ResearchWebTool, RetrieveMaterialTool};
pub use search_tools::{GrepTool, LsTool};
pub use mcp_tools::{McpCallTool, McpListToolsTool};
