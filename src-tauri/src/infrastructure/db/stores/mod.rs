// 按业务域分类的 store 模块
// 每个 store 只依赖 shared 类型，不依赖业务域
//
// ============================================================================
// 未来迁移清单（按域分类）
// ============================================================================
//
// 当前 infra/db/ 下 11 个 store 文件平铺，未来按业务域聚合到本目录。
// 迁移时需同步更新 infra/db/mod.rs 的 pub mod 声明与 pub use 重导出，
// 以及所有引用方的 use 路径。迁移分批进行，每批一个域，保证可回滚。
//
// 未来迁移清单（按域分类）：
// workspace_store.rs → stores/workspace.rs
// session_store.rs   → stores/session.rs
// prompt_store.rs    → stores/prompt.rs
// novel_store.rs     → stores/novel.rs
// trend_store.rs     → stores/trend.rs
// version_store.rs   → stores/version.rs
// wiki_store.rs      → stores/wiki.rs
// kanban_store.rs    → stores/kanban.rs
// loop_store.rs      → stores/loop.rs
// ai_log_store.rs    → stores/ai_log.rs
// stats_store.rs     → stores/stats.rs
//
// ============================================================================
// 迁移约束
// ============================================================================
//
// - 每个 store 只依赖 shared 类型，不依赖任何业务域（保持 infra 不反向依赖）
// - 迁移完成后，infra/db/mod.rs 应声明 `pub mod stores;` 并保留
//   connection/migrate/error/models 等基础设施模块在 db/ 顶层
// - 迁移期间允许"双路径"（旧路径 + 新路径同时存在），但同一 store 不得
//   同时在两处有实现——要么旧文件转发到新路径，要么新文件未启用
// - 迁移不是重命名，是"移动 + 更新所有 use 路径"，必须配套修改引用方
//
// ============================================================================
// 当前状态
// ============================================================================
//
// 本目录为占位目录，尚未在 infra/db/mod.rs 中声明。
// 现有 store 仍在 infra/db/ 顶层平铺，待后续任务分批迁移。
