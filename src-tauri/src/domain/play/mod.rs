// Play 模式：互动小说引擎。
//
// 子模块：
// - types: 数据模型（实体/边/状态槽/事件/Mutation/世界/回合结果）
// - db: SQLite 图谱持久化（entities/edges/state_slots/events）
// - reducer: Mutation 应用器（canonicalize/validate/transaction）
// - agents: 4 个 LLM agent（interpreter/mutator/renderer/reconciler）
// - runner: 4-agent 流水线编排（step/seed_opening/regenerate）
// - store: 文件系统存储（world/run/events/transcript/projection/checkpoint）
// - commands: IPC 命令

pub mod types;
pub mod db;
pub mod reducer;
pub mod agents;
pub mod runner;
pub mod store;
pub mod commands;
