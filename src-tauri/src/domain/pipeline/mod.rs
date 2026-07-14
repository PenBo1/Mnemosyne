// 创作 pipeline —— Rust 后端实现（rig-core 驱动）。
//
// 模块划分：
// - types: 基础数据类型（BookConfig/ChapterMeta/状态枚举）
// - state: 双轨制状态管理（markdown 真相文件 + JSON 加速索引 + delta reducer）
// - agents: 13 核心 pipeline agents 的 prompts + 执行逻辑
// - runner: PipelineRunner 编排（8-agent cycle + Audit↔Revise 循环）
// - commands: IPC 命令入口

pub mod types;
pub mod state;
pub mod agents;
pub mod governance;
pub mod runner;
pub mod commands;
pub mod scheduler;
pub mod interactive_film;
pub mod utils;
