//! 应用核心数据库模块。
//!
//! ## 架构
//!
//! - `connection` — `Database` 句柄与连接池管理
//! - `migrate` — `sqlx::migrate!` 入口，启动时自动应用 `migrations/` 下 SQL
//! - `error` — JSON 列编解码 helper（`json_decode`/`json_encode`）
//! - `models` — 跨 store 共享的领域结构体
//! - `<domain>_store.rs` — 各业务领域的 CRUD 实现（`impl Database`）
//!
//! ## 约定
//!
//! - 所有 store 共享同一个 `SqlitePool`，无跨库问题
//! - `db_err` / `json_decode` / `json_encode` 统一从 `connection` / `error` 模块导入
//! - 多步写操作必须用事务（`pool.begin()` + `tx.commit()`）

pub mod ai_log_store;
pub mod connection;
pub mod error;
pub mod kanban_store;
pub mod loop_store;
pub mod migrate;
pub mod models;
pub mod novel_store;
pub mod prompt_store;
pub mod session_store;
pub mod stats_store;
pub mod trend_store;
pub mod version_store;
pub mod wiki_store;
pub mod workspace_store;

pub use connection::Database;
pub use models::*;
pub use session_store::{CreateSessionRequest, Message, MessageMeta, Session};
pub use ai_log_store::{
    AgentThinking, LlmCall, MemoryOperation, PipelineStageLog, SandboxViolation, ToolExecution,
};
