//! ═══════════════════════════════════════════════════════════════════════════
//! Script/Storyboard Agents - 剧本分镜代理模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：3 个独立 agent（剧本创作 / 分镜创作 / 互动影游创作）+ 规格渲染 + 提示词抽取。
//!
//! 约束：AgentEngine.prompt_once 只支持单轮对话，TS 中的多轮对话合并为单轮。
//! temp/maxTokens 参数无法透传（prompt_once 不支持），仅在注释中标注原始值。

// ── 模块声明 ────────────────────────────────────────────────────────────────

mod agents;
mod parsing;
mod prompts;
mod specs;
mod types;

pub use agents::{write_interactive_film, write_script, write_storyboard};
pub use parsing::{
    estimate_max_tokens, extract_image_prompts, extract_markdown_section,
    normalize_episode_end_labels,
};
pub use specs::{render_interactive_film_spec, render_script_spec, render_storyboard_spec};
pub use types::{
    InteractiveFilmCreationInput, ScriptCreationInput, ScriptTargetFormat,
    StoryboardCreationInput,
};
