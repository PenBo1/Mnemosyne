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

/// S5.5: Filter character_matrix information boundaries for the POV character.
///
/// 移植自 inkos `filterMatrixByPOV`：
/// - 找到 `### 信息边界` / `### Information Boundaries` 段
/// - 在该段的表格里只保留 POV 角色那一行
/// - 其他角色的信息边界行被隐藏（避免 POV 角色读到不该知道的信息）
/// - 非信息边界段原样保留
pub fn filter_matrix_by_pov(character_matrix: &str, pov_character: &str) -> String {
    if character_matrix.is_empty() || character_matrix == "(文件尚未创建)" {
        return character_matrix.to_string();
    }
    if pov_character.is_empty() {
        return character_matrix.to_string();
    }

    // 按 `### ` 切分段落，保留分隔符
    let mut sections: Vec<&str> = Vec::new();
    let mut current_start = 0usize;
    let bytes = character_matrix.as_bytes();
    let mut i = 0usize;
    while i + 3 < bytes.len() {
        if bytes[i] == b'\n' && bytes[i + 1] == b'#' && bytes[i + 2] == b'#' && bytes[i + 3] == b'#' {
            // 找到一个 `### ` 分隔点（前面可能有 \n）
            if current_start < i {
                sections.push(&character_matrix[current_start..i]);
            }
            current_start = i + 1; // 跳过 \n
        }
        i += 1;
    }
    if current_start < character_matrix.len() {
        sections.push(&character_matrix[current_start..]);
    }
    if sections.is_empty() {
        return character_matrix.to_string();
    }

    let filtered: Vec<String> = sections
        .iter()
        .map(|section| {
            // 判断是否为信息边界段
            let is_info_boundary = section.contains("信息边界")
                || section.contains("Information Boundar");
            if !is_info_boundary {
                return section.to_string();
            }

            let lines: Vec<&str> = section.lines().collect();
            let section_header = lines
                .iter()
                .find(|l| l.starts_with("###"))
                .cloned()
                .unwrap_or("### 信息边界");

            // 表头行（含分隔行、列名行）
            let header_lines: Vec<&str> = lines
                .iter()
                .filter(|l| {
                    l.starts_with('|')
                        && (l.contains("---")
                            || l.contains("角色")
                            || l.contains("Character")
                            || l.contains("已知")
                            || l.contains("Known"))
                })
                .copied()
                .collect();

            // 数据行（非表头）
            let data_lines: Vec<&str> = lines
                .iter()
                .filter(|l| {
                    l.starts_with('|')
                        && !l.contains("---")
                        && !l.contains("角色")
                        && !l.contains("Character")
                        && !l.contains("已知")
                        && !l.contains("Known")
                })
                .copied()
                .collect();

            // 只保留 POV 角色那一行
            let pov_rows: Vec<&str> = data_lines
                .iter()
                .filter(|l| l.contains(pov_character))
                .copied()
                .collect();

            let other_count = data_lines.len() - pov_rows.len();

            let mut result = Vec::new();
            result.push(section_header.to_string());
            result.push(format!(
                "（当前视角：{}，其他 {} 个角色的信息边界已隐藏）",
                pov_character, other_count
            ));
            for h in &header_lines {
                result.push(h.to_string());
            }
            for r in &pov_rows {
                result.push(r.to_string());
            }
            result.join("\n")
        })
        .collect();

    filtered.join("\n")
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

    #[test]
    fn test_filter_matrix_by_pov_filters_info_boundary_section() {
        let matrix = "### 角色卡\n| 角色 | 标签 |\n| --- | --- |\n| 张三 | 主角 |\n\n### 信息边界\n| 角色 | 已知 |\n| --- | --- |\n| 张三 | 知道A |\n| 李四 | 知道B |\n| 王五 | 知道C |";
        let filtered = filter_matrix_by_pov(matrix, "张三");
        // POV 行应该保留
        assert!(filtered.contains("张三 | 知道A"));
        // 非 POV 行应该被隐藏
        assert!(!filtered.contains("知道B"));
        assert!(!filtered.contains("知道C"));
        // 应该有隐藏提示
        assert!(filtered.contains("当前视角：张三"));
        assert!(filtered.contains("其他 2 个角色的信息边界已隐藏"));
    }

    #[test]
    fn test_filter_matrix_by_pov_preserves_non_info_boundary_sections() {
        let matrix = "### 角色卡\n| 角色 | 标签 |\n| --- | --- |\n| 张三 | 主角 |\n\n### 信息边界\n| 角色 | 已知 |\n| --- | --- |\n| 张三 | 知道A |\n| 李四 | 知道B |";
        let filtered = filter_matrix_by_pov(matrix, "张三");
        // 角色卡段应该原样保留
        assert!(filtered.contains("### 角色卡"));
        assert!(filtered.contains("张三 | 主角"));
    }

    #[test]
    fn test_filter_matrix_by_pov_english_section() {
        let matrix = "### Information Boundaries\n| Character | Known |\n| --- | --- |\n| Alice | secret1 |\n| Bob | secret2 |";
        let filtered = filter_matrix_by_pov(matrix, "Alice");
        assert!(filtered.contains("Alice | secret1"));
        assert!(!filtered.contains("secret2"));
    }

    #[test]
    fn test_filter_matrix_by_pov_no_info_boundary_section() {
        // 没有信息边界段时应该原样返回
        let matrix = "### 角色卡\n| 角色 | 标签 |\n| --- | --- |\n| 张三 | 主角 |";
        let filtered = filter_matrix_by_pov(matrix, "张三");
        assert_eq!(filtered, matrix);
    }

    #[test]
    fn test_filter_matrix_by_pov_pov_not_in_info_boundary() {
        // POV 角色在信息边界段没有行时，应该保留表头但隐藏所有数据行
        let matrix = "### 信息边界\n| 角色 | 已知 |\n| --- | --- |\n| 李四 | 知道B |\n| 王五 | 知道C |";
        let filtered = filter_matrix_by_pov(matrix, "张三");
        assert!(!filtered.contains("知道B"));
        assert!(!filtered.contains("知道C"));
        assert!(filtered.contains("其他 2 个角色的信息边界已隐藏"));
    }
}
