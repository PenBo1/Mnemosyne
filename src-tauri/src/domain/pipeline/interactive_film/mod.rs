// 互动电影模块。
//
// 子模块：
// - graph_schema: StoryGraph 数据结构（对应 zod schema）
// - graph_store: JSON 持久化（load/save）
// - delta: StoryGraphDelta + applyStoryGraphDelta
// - evaluator: 运行时求值（条件/效果/可见选项/变量初始化）
// - validation: 校验（4 error 级 + 9 issue 级）
// - paths: 路径枚举（DFS + 状态去重）
// - emotion: 情感弧线分析
// - generate: 从前提一次性生成完整 StoryGraph
// - authoring: 创作辅助（LLM→delta 转换 + delta builder + 创作状态）
// - export_html: 导出为单文件可玩 HTML
// - export_ink: 导出为 Ink 脚本
// - commands: IPC 命令（film_*）

pub mod graph_schema;
pub mod graph_store;
pub mod delta;
pub mod evaluator;
pub mod validation;
pub mod paths;
pub mod emotion;
pub mod generate;
pub mod authoring;
pub mod export_html;
pub mod export_ink;
pub mod commands;
