mod types;
mod writer;
mod writer_zh;
mod writer_en;
mod planner;
mod settler;
mod observer;
mod short_fiction;
mod fanfic;

pub use types::*;
pub use writer::WriterPromptBuilder;
pub use planner::build_planner_system_prompt;
pub use settler::build_settler_system_prompt;
pub use observer::build_observer_system_prompt;
pub use short_fiction::{build_short_fiction_outline_system_prompt, build_short_fiction_writer_system_prompt};
pub use fanfic::build_fanfic_canon_section;

pub fn architect_system_prompt() -> String {
    "你是一位小说架构师。你的职责是根据创作简报创建小说的基础设定，包括世界观、主要角色、故事主线和核心冲突。请输出结构化的JSON格式。".to_string()
}

pub fn planner_system_prompt() -> String {
    let language = Language::default();
    build_planner_system_prompt(&language)
}

pub fn composer_system_prompt() -> String {
    "你是一位小说编排师。你的职责是根据章节意图和上下文，编排章节的详细大纲，包括场景安排、角色出场、情感节奏等。请输出结构化的JSON格式。".to_string()
}

pub fn writer_system_prompt() -> String {
    let genre = GenreConfig::default();
    let builder = WriterPromptBuilder::new(Language::Zh, genre, 3000);
    builder.build()
}

pub fn auditor_system_prompt() -> String {
    let language = Language::default();
    build_observer_system_prompt(&language)
}

pub fn reviser_system_prompt() -> String {
    "你是一位小说修订师。你的职责是根据审计结果修订章节内容，修复所有critical和warning级别的问题，同时保持故事的连贯性和文风一致性。".to_string()
}

pub fn observer_system_prompt() -> String {
    let language = Language::default();
    build_observer_system_prompt(&language)
}

pub fn reflector_system_prompt() -> String {
    "你是一位小说反思师。你的职责是根据观察结果更新故事状态，包括新增伏笔、推进已有伏笔、提取新事实、更新章节摘要。请输出结构化的JSON格式。".to_string()
}
