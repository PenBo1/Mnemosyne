//! Chapter splitter utilities.

/// Split a long text into chapters
pub fn split_chapters(text: &str) -> Vec<ChapterSplit> {
    let mut chapters = Vec::new();
    let mut current_title = String::new();
    let mut current_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            if !current_content.trim().is_empty() || !current_title.is_empty() {
                chapters.push(ChapterSplit {
                    title: current_title.clone(),
                    content: current_content.trim().to_string(),
                });
                current_title = trimmed.trim_start_matches('#').trim().to_string();
                current_content.clear();
            } else {
                current_title = trimmed.trim_start_matches('#').trim().to_string();
            }
        } else {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    if !current_content.trim().is_empty() || !current_title.is_empty() {
        chapters.push(ChapterSplit {
            title: current_title,
            content: current_content.trim().to_string(),
        });
    }

    chapters
}

pub struct ChapterSplit {
    pub title: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_chapters() {
        let text = "# Chapter 1\nContent 1\n\n# Chapter 2\nContent 2";
        let chapters = split_chapters(text);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[1].title, "Chapter 2");
    }
}
