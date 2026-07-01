// ── Main Agent 小说创作工具集 ───────────────────────────────────
//
// 设计目标：让主 Agent 在 ReAct 循环中能自主触发小说创作流水线。
// 入口选择（用户决策）：合并 chat 与 main-agent 入口，统一通过 main agent
// 暴露 spawn_subagent + 创作工具。
//
// 工具实现复用 `PipelineRunner` 的现成方法（`create_book` / `write_next_chapter`），
// 不绕过架构边界 —— core/agent 内部互调合法。

mod novel_tools;

pub use novel_tools::{NovelCreateTool, WriteNextChapterTool, GetNovelStatusTool, NovelToolDeps};
