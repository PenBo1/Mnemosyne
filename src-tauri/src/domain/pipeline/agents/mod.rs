// 13 核心 pipeline agents —— Rust 实现。
//
// 每个 agent 文件包含：
// - prompt 模板（const 字符串 + 参数插值）
// - 执行函数（接收 AgentEngine + 上下文，调用 prompt_once，解析输出）
//
// 依赖关系：agents → core::agent::engine::AgentEngine（复用 prompt_once）
// 这与 domain/radar/agent.rs 的依赖模式一致。

pub mod architect;
pub mod planner;
pub mod writer;
pub mod continuity;
pub mod reviser;
pub mod polisher;
pub mod length_normalizer;
pub mod foundation_reviewer;
pub mod state_validator;
pub mod consolidator;
pub mod chapter_analyzer;
pub mod composer;
pub mod short_fiction;
pub mod fanfic_canon_importer;
pub mod script_storyboard;
