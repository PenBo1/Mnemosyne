// Project Memory —— 工作区级别的项目记忆文件。
//
// 每个工作区(Workspace)对应一个独立的 project_memory.md 文件,内容为自由 markdown。
//
// 与 agent 的 MEMORY.md 区别:
// - agent MEMORY.md 是"跨 session 的 agent 教训" —— 全局,与 workspace 无关
// - project_memory.md 是"项目级背景知识/约定/进度" —— 隔离,与 workspace 1:1
//
// 存储位置:`<data_dir>/workspaces/<workspace_id>/project_memory.md`
// 不污染 workspace 的外部 path 目录,便于 delete_workspace 时确定性清理。

pub mod store;
pub mod commands;
pub mod state;

pub use store::ProjectMemoryStore;
pub use state::ProjectMemoryState;
