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

pub mod graph_schema;
pub mod graph_store;
pub mod delta;
pub mod evaluator;
pub mod validation;
pub mod paths;
pub mod emotion;
