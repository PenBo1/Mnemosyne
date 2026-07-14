// Loop-Engineering 应用层 —— 前端 loop_* IPC 命令的后端实现。
//
// 模块组成:
// - types:与前端 src/features/loop/types/loop.ts 对齐的 DTO(camelCase serde)
// - builtin_patterns:4 个 builtin pattern 的声明式定义 + DB seed
// - commands:IPC 命令实现(loop_create_state / loop_get_states / ...)
//
// 与 core/agent/loop_engine/ 的关系:
// - core 层:运行时循环引擎(types/budget/prompts,用于 check_budget 和 sub-agent prompt)
// - application 层:面向前端的 CRUD 接口(状态管理 + 模式管理 + 运行日志查询)
// - 两层通过 pattern_id(LoopPatternId::as_str())关联

pub mod types;
pub mod builtin_patterns;
pub mod commands;

pub use types::{
    CostConfigDto, CreateLoopStateRequest, LoopConfigDto, LoopPatternDto, LoopRunLogDto,
    LoopRunResultDto, LoopStateDto, PhaseDefDto, PhaseResultDto, UpdateLoopStateRequest,
    UpsertLoopPatternRequest,
};
pub use builtin_patterns::ensure_builtin_patterns;
