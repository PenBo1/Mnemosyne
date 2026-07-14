// 交互运行时（Interaction Runtime）—— 面向用户操作的统一入口。
//
// 模块划分：
// - types: 核心类型（SessionKind / AutomationMode / InteractionIntent / ExecutionState / ...）
// - truth_authority: 真相文件白名单 + TruthAuthority 分类（防路径穿越）
// - edit_controller: 编辑事务规划与执行（6 种 EditRequest）
// - runtime: run_interaction_request 核心调度（intent → pipeline / agent / edit 分发）
// - session_store: SQLite 持久化（interaction_sessions 表）
// - commands: IPC 命令入口（参数提取 + 校验 + 委派，无业务逻辑）
//
// 关键差异：
// - 后端实现，IPC 通过 Tauri State 注入 AgentState / DbState / DataDir
// - 真相文件存放在 <book_dir>/story/ 下（与 pipeline state/manager.rs 一致）
// - 编辑事务使用 DataDir.books_dir() 获取路径，不手动构造

pub mod types;
pub mod truth_authority;
pub mod edit_controller;
pub mod runtime;
pub mod session_store;
pub mod commands;
