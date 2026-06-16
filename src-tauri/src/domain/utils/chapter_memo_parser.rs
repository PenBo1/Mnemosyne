//! Chapter memo parser utilities.

use crate::domain::agents::governance::ChapterMemo;

/// Parse a chapter memo from markdown text
pub fn parse_chapter_memo(text: &str, chapter_number: u32) -> ChapterMemo {
    let goal = extract_section(text, &["本章目标", "Chapter goal"]).unwrap_or_default();
    let body = text.to_string();
    let thread_refs = extract_list_items(text, &["关联线索", "Thread refs"]);
    let is_golden_opening = chapter_number <= 3;

    ChapterMemo {
        chapter: chapter_number,
        goal,
        is_golden_opening,
        body,
        thread_refs,
    }
}

fn extract_section(content: &str, headings: &[&str]) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim().to_lowercase();
        for heading in headings {
            if trimmed.contains(&heading.to_lowercase()) {
                let mut result = Vec::new();
                for next in lines.iter().skip(i + 1) {
                    let next = next.trim();
                    if next.starts_with('#') { break; }
                    if !next.is_empty() { result.push(next.to_string()); }
                }
                if !result.is_empty() { return Some(result.join("\n")); }
            }
        }
    }
    None
}

fn extract_list_items(content: &str, headings: &[&str]) -> Vec<String> {
    let section = extract_section(content, headings).unwrap_or_default();
    section.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .map(|s| s.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chapter_memo() {
        let text = "# 第5章 memo\n\n## 本章目标\n揭露秘密\n\n## 关联线索\n- H001\n- H003";
        let memo = parse_chapter_memo(text, 5);
        assert_eq!(memo.goal, "揭露秘密");
        assert_eq!(memo.thread_refs.len(), 2);
        assert!(!memo.is_golden_opening);
    }
}
