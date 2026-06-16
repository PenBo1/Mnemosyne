use crate::domain::utils::story_markdown::*;

/// Memory selection for planner input.
pub struct MemorySelection {
    pub summaries: Vec<Vec<String>>,
    pub hooks: Vec<ParsedHook>,
    pub active_hooks: Vec<ParsedHook>,
    pub recyclable_hooks: Vec<ParsedHook>,
    pub facts: Vec<String>,
}

/// Retrieve memory selection from story files.
pub fn retrieve_memory_selection(
    book_dir: &std::path::Path,
    chapter_number: u32,
    _goal: &str,
) -> MemorySelection {
    let story_dir = book_dir.join("story");

    let hooks_raw = read_safe(&story_dir.join("pending_hooks.md"));
    let summaries_raw = read_safe(&story_dir.join("chapter_summaries.md"));
    let current_state_raw = read_safe(&story_dir.join("current_state.md"));

    let all_hooks = parse_pending_hooks_markdown(&hooks_raw);
    let active_hooks: Vec<ParsedHook> = all_hooks.iter()
        .filter(|h| !crate::domain::utils::hook_lifecycle::is_terminal_status(&h.status))
        .cloned()
        .collect();

    let recyclable_hooks = compute_recyclable_hooks(&active_hooks, chapter_number);
    let summaries = parse_chapter_summaries_markdown(&summaries_raw);
    let facts = extract_facts_from_state(&current_state_raw);

    MemorySelection {
        summaries,
        hooks: all_hooks,
        active_hooks,
        recyclable_hooks,
        facts,
    }
}

/// Compute hooks that MUST be addressed this chapter (stale enough to force action).
pub fn compute_recyclable_hooks(hooks: &[ParsedHook], chapter_number: u32) -> Vec<ParsedHook> {
    hooks.iter()
        .filter(|h| !crate::domain::utils::hook_lifecycle::is_terminal_status(&h.status))
        .filter(|h| !crate::domain::utils::hook_lifecycle::is_future_planned_hook(h.start_chapter, chapter_number))
        .filter(|h| {
            let silence = crate::domain::utils::hook_lifecycle::hook_silence(
                h.start_chapter, h.last_advanced_chapter, chapter_number,
            );
            let threshold = crate::domain::utils::hook_lifecycle::recycle_threshold(
                &h.status, h.core_hook,
            );
            silence >= threshold
        })
        .cloned()
        .collect()
}

/// Format hooks for display in prompts.
pub fn format_hook_snapshot(hooks: &[ParsedHook], language: &str) -> String {
    if hooks.is_empty() {
        return if language == "en" { "(no active hooks)" } else { "(无活跃伏笔)" }.to_string();
    }
    render_hook_snapshot(hooks, language)
}

/// Format summaries for display in prompts.
pub fn format_summary_snapshot(summaries: &[Vec<String>], language: &str) -> String {
    if summaries.is_empty() {
        return if language == "en" { "(no chapter summaries)" } else { "(无章节摘�?" }.to_string();
    }

    let header = if language == "en" {
        "| Chapter | Title | Characters | Events | State Changes | Hook Activity | Mood | Type |"
    } else {
        "| 章节 | 标题 | 出场人物 | 关键事件 | 状态变�?| 伏笔动�?| 情绪基调 | 章节类型 |"
    };
    let sep = "| --- | --- | --- | --- | --- | --- | --- | --- |";

    let rows: Vec<String> = summaries.iter().map(|row| {
        format!("| {} |", row.join(" | "))
    }).collect();

    format!("{}\n{}\n{}", header, sep, rows.join("\n"))
}

fn read_safe(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn extract_facts_from_state(state: &str) -> Vec<String> {
    state.lines()
        .filter(|l| l.starts_with("- ") || l.starts_with("* "))
        .map(|l| l.trim_start_matches(['-', '*', ' ']).to_string())
        .filter(|l| !l.is_empty())
        .collect()
}
