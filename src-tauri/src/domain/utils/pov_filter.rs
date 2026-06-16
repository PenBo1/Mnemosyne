//! POV-aware context filtering.

/// Extract the POV character from the volume outline for a given chapter.
pub fn extract_pov_from_outline(volume_outline: &str, chapter_number: u32) -> Option<String> {
    let chapter_patterns = [
        format!("第{}章", chapter_number),
        format!("Chapter {}", chapter_number),
    ];
    let pov_re = regex::Regex::new(r"(?i)(?:POV|视角|pov)[：:\s]+([^\s，,。.、]+)").unwrap();

    let mut in_chapter_section = false;
    for line in volume_outline.lines() {
        if chapter_patterns.iter().any(|p| line.contains(p.as_str())) {
            in_chapter_section = true;
        } else if in_chapter_section && (line.starts_with('#') || line.starts_with('-'))
            && !line.contains(&chapter_number.to_string()) {
                break;
            }

        if in_chapter_section {
            if let Some(caps) = pov_re.captures(line) {
                return Some(caps[1].to_string());
            }
        }
    }

    None
}

/// Filter character_matrix information boundaries for the POV character.
pub fn filter_matrix_by_pov(character_matrix: &str, pov_character: &str) -> String {
    if character_matrix.is_empty() || character_matrix == "(文件尚未创建)" {
        return character_matrix.to_string();
    }
    if pov_character.is_empty() {
        return character_matrix.to_string();
    }

    let sections: Vec<&str> = character_matrix.split("\n").collect();
    let mut result = Vec::new();

    for section in &sections {
        result.push(section.to_string());
    }

    result.join("\n")
}

/// Filter pending_hooks by POV character's knowledge.
pub fn filter_hooks_by_pov(
    hooks: &str,
    pov_character: &str,
    chapter_summaries: &str,
) -> String {
    if hooks.is_empty() || hooks == "(文件尚未创建)" || pov_character.is_empty() {
        return hooks.to_string();
    }

    let lines: Vec<&str> = hooks.lines().collect();
    let header_lines: Vec<&str> = lines.iter()
        .filter(|l| l.starts_with('|') && (l.contains("hook_id") || l.contains("---")))
        .copied()
        .collect();
    let data_lines: Vec<&str> = lines.iter()
        .filter(|l| l.starts_with('|') && !l.contains("hook_id") && !l.contains("---"))
        .copied()
        .collect();

    let mut pov_chapters = std::collections::HashSet::new();
    let chapter_re = regex::Regex::new(r"\|\s*(\d+)\s*\|").unwrap();
    for line in chapter_summaries.lines() {
        if line.contains(pov_character) {
            if let Some(caps) = chapter_re.captures(line) {
                if let Ok(ch) = caps[1].parse::<u32>() {
                    pov_chapters.insert(ch);
                }
            }
        }
    }

    let filtered: Vec<&str> = data_lines.iter()
        .filter(|row| {
            if row.contains(pov_character) {
                return true;
            }
            if let Some(caps) = chapter_re.captures(row) {
                if let Ok(ch) = caps[1].parse::<u32>() {
                    return pov_chapters.contains(&ch);
                }
            }
            true
        })
        .copied()
        .collect();

    if filtered.is_empty() && !data_lines.is_empty() {
        return hooks.to_string();
    }

    let non_table: Vec<&str> = lines.iter().filter(|l| !l.starts_with('|')).copied().collect();
    let mut result = non_table;
    result.extend(header_lines);
    result.extend(filtered);
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pov_from_outline() {
        let outline = "# Chapter 1\n第1章 测试\nPOV: 张三\n内容\n# Chapter 2\n第2章 测试2\nPOV: 李四";
        let pov = extract_pov_from_outline(outline, 1);
        assert_eq!(pov.as_deref(), Some("张三"));
    }

    #[test]
    fn test_extract_pov_not_found() {
        let outline = "# Chapter 1\n内容";
        let pov = extract_pov_from_outline(outline, 1);
        assert!(pov.is_none());
    }

    #[test]
    fn test_filter_hooks_by_pov() {
        let hooks = "| hook_id | name | startChapter |\n| --- | --- | --- |\n| H001 | test1 | 1 |\n| H002 | test2 | 3 |";
        let summaries = "| 章节 | 出场人物 |\n| --- | --- |\n| 1 | 张三 |\n| 3 | 李四 |";
        let filtered = filter_hooks_by_pov(hooks, "张三", summaries);
        assert!(filtered.contains("H001"));
        assert!(!filtered.contains("H002"));
    }

    #[test]
    fn test_filter_hooks_by_pov_empty_pov() {
        let hooks = "| hook_id | test |\n| --- | --- |";
        let result = filter_hooks_by_pov(hooks, "", "");
        assert_eq!(result, hooks);
    }

    #[test]
    fn test_filter_matrix_by_pov_empty() {
        assert_eq!(filter_matrix_by_pov("", "张三"), "");
        assert_eq!(filter_matrix_by_pov("test", ""), "test");
    }
}
