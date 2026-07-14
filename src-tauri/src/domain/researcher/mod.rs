// 研究员系统 —— 基于 LLM 生成结构化研究报告。
//
// 输出结构: claims/conflicts/unknowns/creativeImplications/markdown,
// 但实现上不调用外部 web search,而是通过 AgentEngine.prompt_once 让 LLM 综合分析。
// depth(quick/standard/deep)控制 prompt 注入的查询角度数量(1/2/3)。

pub mod commands;
pub mod report;
pub mod types;
