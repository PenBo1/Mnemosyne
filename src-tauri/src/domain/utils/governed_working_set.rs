//! Governed working set — hook and character working sets from ContextPackage.

use crate::domain::agents::governance::ContextPackage;
use crate::domain::utils::story_markdown::{ParsedHook, parse_pending_hooks_markdown, render_hook_snapshot};

pub fn build_governed_hook_working_set(
    hooks_markdown: &str, context_package: &ContextPackage, chapter_intent: Option<&str>,
    chapter_number: u32, language: &str,
) -> String {
    if hooks_markdown.is_empty() || hooks_markdown == "(文件不存在)" || hooks_markdown == "(文件尚未创建)" {
        return hooks_markdown.to_string();
    }
    let hooks = parse_pending_hooks_markdown(hooks_markdown);
    if hooks.is_empty() { return hooks_markdown.to_string(); }
    let selected_ids: std::collections::HashSet<String> = context_package.selected_context.iter()
        .filter(|e| e.source.starts_with("story/pending_hooks.md"))
        .filter_map(|e| e.source.strip_prefix("story/pending_hooks.md#")).map(|s| s.to_string()).collect();
    let agenda_ids = collect_hook_agenda_ids(chapter_intent);
    let working_set: Vec<&ParsedHook> = hooks.iter().filter(|h| {
        selected_ids.contains(&h.hook_id) || agenda_ids.contains(&h.hook_id)
            || crate::domain::utils::hook_lifecycle::is_hook_within_chapter_window(h.last_advanced_chapter, h.start_chapter, chapter_number, 5)
    }).collect();
    if working_set.is_empty() || working_set.len() >= hooks.len() { return hooks_markdown.to_string(); }
    render_hook_snapshot(&working_set.into_iter().cloned().collect::<Vec<_>>(), language)
}

fn collect_hook_agenda_ids(chapter_intent: Option<&str>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    if let Some(intent) = chapter_intent {
        let mut in_hook_agenda = false;
        let mut capture = false;
        for line in intent.lines() {
            let trimmed = line.trim();
            if trimmed == "## Hook Agenda" || trimmed == "## 本章 hook 账" { in_hook_agenda = true; capture = false; continue; }
            if in_hook_agenda && trimmed.starts_with("## ") && trimmed != "## Hook Agenda" && trimmed != "## 本章 hook 账" { break; }
            if trimmed == "### Must Advance" || trimmed == "### advance" { capture = true; continue; }
            if trimmed.starts_with("### ") { capture = false; continue; }
            if capture && trimmed.starts_with("- ") {
                let value = trimmed[2..].trim();
                if !value.is_empty() && value.to_lowercase() != "none" { ids.insert(value.to_string()); }
            }
        }
    }
    ids
}

pub fn merge_table_markdown_by_key(original: &str, updated: &str, key_columns: &[usize]) -> String {
    let original_rows = parse_table_rows(original);
    let updated_rows = parse_table_rows(updated);
    if original_rows.is_empty() || updated_rows.is_empty() { return updated.to_string(); }
    let mut merged = original_rows.clone();
    let mut original_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, row) in merged.iter().enumerate() { original_index.insert(build_row_key(row, key_columns), i); }
    for row in &updated_rows {
        let key = build_row_key(row, key_columns);
        if let Some(idx) = original_index.get(&key) { merged[*idx] = row.clone(); }
        else { original_index.insert(key, merged.len()); merged.push(row.clone()); }
    }
    let header = extract_table_header(original).unwrap_or_default();
    let separator = extract_table_separator(original).unwrap_or_default();
    let rows: Vec<String> = merged.iter().map(|r| format!("| {} |", r.join(" | "))).collect();
    format!("{}\n{}\n{}", header, separator, rows.join("\n"))
}

pub fn merge_character_matrix_markdown(original: &str, updated: &str) -> String {
    if original.is_empty() || updated.is_empty() { return updated.to_string(); }
    updated.to_string()
}

pub fn build_governed_character_matrix_working_set(matrix_markdown: &str, _chapter_intent: &str, _context_package: &ContextPackage, _protagonist_name: Option<&str>) -> String {
    matrix_markdown.to_string()
}

fn parse_table_rows(content: &str) -> Vec<Vec<String>> {
    content.lines().filter(|l| l.starts_with('|') && !l.contains("---"))
        .map(|l| l.split('|').skip(1).map(|c| c.trim().to_string()).collect())
        .filter(|r: &Vec<String>| !r.is_empty() && !r[0].is_empty()).collect()
}

fn extract_table_header(content: &str) -> Option<String> {
    content.lines().find(|l| l.starts_with('|') && (l.contains("hook_id") || l.contains("章节") || l.contains("Chapter"))).map(|s| s.to_string())
}

fn extract_table_separator(content: &str) -> Option<String> {
    content.lines().find(|l| l.starts_with('|') && l.contains("---")).map(|s| s.to_string())
}

fn build_row_key(row: &[String], key_columns: &[usize]) -> String {
    key_columns.iter().filter_map(|&i| row.get(i)).cloned().collect::<Vec<_>>().join("::")
}
