//! Smart context filtering for Writer and Auditor prompts.

/// Cap a large context block, keeping head + tail when over budget.
pub fn cap_context_block(content: &str, label: &str, max_chars: usize) -> String {
    if content.is_empty() || content == "(文件尚未创建)" {
        return content.to_string();
    }
    if max_chars == 0 { return String::new(); }
    if content.len() <= max_chars { return content.to_string(); }

    let omitted = content.len() - max_chars;
    let note = format!("\n\n[上下文预算：从 {} 省略了约 {} 字符，保留开头和最新尾部。]\n\n", label, omitted);
    let keep_chars = max_chars.saturating_sub(note.len());
    if keep_chars < 2 { return content[..max_chars].to_string(); }
    let head_chars = (keep_chars as f64 * 0.45) as usize;
    let tail_chars = keep_chars - head_chars;
    format!("{}{}{}", &content[..head_chars], note, &content[content.len() - tail_chars..])
}

/// Filter pending_hooks: remove resolved/closed hooks.
pub fn filter_hooks(hooks: &str) -> String {
    if hooks.is_empty() || hooks == "(文件尚未创建)" { return hooks.to_string(); }
    filter_table_rows(hooks, |row| {
        let lower = row.to_lowercase();
        !lower.contains("已回收") && !lower.contains("resolved") && !lower.contains("closed")
    })
}

/// Filter chapter_summaries: keep only the most recent N chapters.
pub fn filter_summaries(summaries: &str, current_chapter: u32, keep_recent: u32) -> String {
    if summaries.is_empty() || summaries == "(文件尚未创建)" { return summaries.to_string(); }
    let cutoff = current_chapter.saturating_sub(keep_recent);
    filter_table_rows(summaries, |row| {
        extract_chapter_num_from_row(row).map(|n| n > cutoff).unwrap_or(true)
    })
}

/// Filter subplot_board: remove closed/resolved subplots.
pub fn filter_subplots(board: &str) -> String {
    if board.is_empty() || board == "(文件尚未创建)" { return board.to_string(); }
    filter_table_rows(board, |row| {
        let lower = row.to_lowercase();
        !lower.contains("已回收") && !lower.contains("closed")
            && !lower.contains("resolved") && !lower.contains("已完结")
    })
}

/// Filter emotional_arcs: keep only the most recent N chapters.
pub fn filter_emotional_arcs(arcs: &str, current_chapter: u32, keep_recent: u32) -> String {
    if arcs.is_empty() || arcs == "(文件尚未创建)" { return arcs.to_string(); }
    let cutoff = current_chapter.saturating_sub(keep_recent);
    filter_table_rows(arcs, |row| {
        extract_chapter_num_from_row(row).map(|n| n > cutoff).unwrap_or(true)
    })
}

/// Filter character_matrix for relevant characters.
pub fn filter_character_matrix(matrix: &str, _volume_outline: &str, _protagonist_name: Option<&str>) -> String {
    if matrix.is_empty() || matrix == "(文件尚未创建)" { return matrix.to_string(); }
    matrix.to_string()
}

fn extract_chapter_num_from_row(row: &str) -> Option<u32> {
    let parts: Vec<&str> = row.split('|').collect();
    if parts.len() < 2 { return None; }
    parts.get(1)?.trim().parse::<u32>().ok()
}

fn is_header_row(line: &str) -> bool {
    let trimmed = line.trim_start_matches('|').trim();
    let lower = trimmed.to_lowercase();
    lower.starts_with("章节") || lower.starts_with("角色") || lower.starts_with("hook_id")
        || lower.starts_with("chapter") || lower.starts_with("character")
}

fn filter_table_rows(content: &str, predicate: impl Fn(&str) -> bool) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut non_table = Vec::new();
    let mut header_lines = Vec::new();
    let mut data_lines = Vec::new();
    for line in &lines {
        if !line.starts_with('|') {
            non_table.push(line.to_string());
        } else if line.contains("---") || is_header_row(line) {
            header_lines.push(line.to_string());
        } else {
            data_lines.push(line.to_string());
        }
    }
    let filtered: Vec<String> = data_lines.iter().filter(|row| predicate(row)).cloned().collect();
    if filtered.is_empty() && !data_lines.is_empty() { return content.to_string(); }
    let mut result = non_table;
    result.extend(header_lines);
    result.extend(filtered);
    result.join("\n")
}
