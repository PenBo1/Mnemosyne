use crate::shared::errors::AppError;
use crate::infrastructure::state_store::gc::utils;

/// Save a chapter file to disk
pub fn save_chapter_file(
    book_dir: &std::path::Path,
    chapter_number: u32,
    title: &str,
    content: &str,
) -> Result<(), AppError> {
    let chapters_dir = book_dir.join("chapters");
    std::fs::create_dir_all(&chapters_dir)?;

    // Remove old chapter file with same number
    let prefix = format!("{:04}_", chapter_number);
    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    let filename = format!("{}{}.md", prefix, utils::sanitize_filename(title));
    let heading = if utils::is_english_book(book_dir) {
        format!("# Chapter {}: {}", chapter_number, title)
    } else {
        format!("# 第{}章 {}", chapter_number, title)
    };

    std::fs::write(
        chapters_dir.join(filename),
        format!("{}\n\n{}", heading, content),
    ).map_err(|e| AppError::internal(format!("Failed to write chapter: {}", e)))
}

/// Save truth files after writing
pub fn save_truth_files(
    book_dir: &std::path::Path,
    current_state: Option<&str>,
    pending_hooks: Option<&str>,
    chapter_summary: Option<&str>,
    subplot_board: Option<&str>,
    emotional_arcs: Option<&str>,
    character_matrix: Option<&str>,
) -> Result<(), AppError> {
    let story_dir = book_dir.join("story");
    std::fs::create_dir_all(&story_dir)?;

    if let Some(content) = current_state {
        if !content.is_empty() {
            std::fs::write(story_dir.join("current_state.md"), content)?;
        }
    }
    if let Some(content) = pending_hooks {
        if !content.is_empty() {
            std::fs::write(story_dir.join("pending_hooks.md"), content)?;
        }
    }
    if let Some(content) = chapter_summary {
        if !content.is_empty() {
            append_chapter_summary(&story_dir, content)?;
        }
    }
    if let Some(content) = subplot_board {
        if !content.is_empty() {
            std::fs::write(story_dir.join("subplot_board.md"), content)?;
        }
    }
    if let Some(content) = emotional_arcs {
        if !content.is_empty() {
            std::fs::write(story_dir.join("emotional_arcs.md"), content)?;
        }
    }
    if let Some(content) = character_matrix {
        if !content.is_empty() {
            std::fs::write(story_dir.join("character_matrix.md"), content)?;
        }
    }

    Ok(())
}

fn append_chapter_summary(story_dir: &std::path::Path, summary: &str) -> Result<(), AppError> {
    let path = story_dir.join("chapter_summaries.md");
    let header = "# 章节摘要\n\n| 章节 | 标题 | 出场人物 | 关键事件 | 状态变化 | 伏笔动态 | 情绪基调 | 章节类型 |\n|------|------|----------|----------|----------|----------|----------|----------|\n";

    let mut content = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        header.to_string()
    };

    for line in summary.lines() {
        if line.starts_with('|') && !line.starts_with("| 章节") && !line.starts_with("|--") {
            content.push_str(line);
            content.push('\n');
        }
    }

    std::fs::write(path, content)?;
    Ok(())
}
