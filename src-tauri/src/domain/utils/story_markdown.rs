//! Parse and render story markdown files.

#[derive(Debug, Clone)]
pub struct ParsedHook {
    pub hook_id: String, pub name: String, pub hook_type: String, pub status: String,
    pub start_chapter: u32, pub last_advanced_chapter: u32,
    pub expected_payoff: String, pub core_hook: bool, pub promoted: bool,
}

pub fn parse_pending_hooks_markdown(markdown: &str) -> Vec<ParsedHook> {
    let mut hooks = Vec::new();
    for line in markdown.lines() {
        if line.starts_with('|') && !line.contains("---") && !is_hook_header(line) {
            if let Some(hook) = parse_hook_row(line) { hooks.push(hook); }
        }
    }
    hooks
}

fn is_hook_header(line: &str) -> bool { let l = line.to_lowercase(); l.contains("hook_id") || l.contains("伏笔") }

fn parse_hook_row(line: &str) -> Option<ParsedHook> {
    let cells: Vec<&str> = line.split('|').skip(1).collect();
    if cells.len() < 6 { return None; }
    let hook_id = cells.first()?.trim().to_string();
    if hook_id.is_empty() || hook_id == "---" { return None; }
    Some(ParsedHook {
        hook_id,
        name: cells.get(1).map(|c| c.trim().to_string()).unwrap_or_default(),
        hook_type: cells.get(2).map(|c| c.trim().to_string()).unwrap_or_default(),
        status: cells.get(3).map(|c| c.trim().to_string()).unwrap_or_default(),
        start_chapter: cells.get(4).and_then(|c| c.trim().parse().ok()).unwrap_or(0),
        last_advanced_chapter: cells.get(5).and_then(|c| c.trim().parse().ok()).unwrap_or(0),
        expected_payoff: cells.get(6).map(|c| c.trim().to_string()).unwrap_or_default(),
        core_hook: cells.get(7).map(|c| c.trim().to_lowercase() == "true").unwrap_or(false),
        promoted: cells.get(8).map(|c| c.trim().to_lowercase() == "true").unwrap_or(false),
    })
}

pub fn render_hook_snapshot(hooks: &[ParsedHook], language: &str) -> String {
    let header = if language == "en" {
        "| hook_id | name | type | status | startChapter | lastAdvanced | payoff | coreHook | promoted |"
    } else {
        "| hook_id | 名称 | 类型 | 状态 | 起始章 | 上次推进 | 预期兑现 | 核心 | 升级 |"
    };
    let sep = "| --- | --- | --- | --- | --- | --- | --- | --- | --- |";
    let rows: Vec<String> = hooks.iter().map(|h| {
        format!("| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            h.hook_id, h.name, h.hook_type, h.status, h.start_chapter, h.last_advanced_chapter, h.expected_payoff, h.core_hook, h.promoted)
    }).collect();
    format!("{}\n{}\n{}", header, sep, rows.join("\n"))
}

pub fn render_summary_snapshot(_summaries: &[Vec<String>], _language: &str) -> String { String::new() }

pub fn parse_chapter_summaries_markdown(markdown: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for line in markdown.lines() {
        if line.starts_with('|') && !line.contains("---") && !is_summary_header(line) {
            let cells: Vec<String> = line.split('|').skip(1).map(|c| c.trim().to_string()).collect();
            if cells.len() >= 2 && !cells[0].is_empty() { rows.push(cells); }
        }
    }
    rows
}

pub fn parse_pending_hooks(_markdown: &str) -> Vec<ParsedHook> { Vec::new() }
pub fn parse_current_state_facts(_markdown: &str, _chapter: u32) -> Vec<String> { Vec::new() }

fn is_summary_header(line: &str) -> bool { let l = line.to_lowercase(); l.contains("章节") || l.contains("chapter") || l.contains("标题") }
