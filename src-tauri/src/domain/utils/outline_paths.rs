//! Outline path utilities.

use std::path::Path;

pub fn read_story_frame(book_dir: &Path, fallback: &str) -> String {
    let primary = read_safe(&book_dir.join("story/outline/story_frame.md"));
    if primary.is_empty() { let legacy = read_safe(&book_dir.join("story/story_bible.md")); if legacy.is_empty() { fallback.to_string() } else { legacy } } else { primary }
}

pub fn read_volume_map(book_dir: &Path, fallback: &str) -> String {
    let primary = read_safe(&book_dir.join("story/outline/volume_map.md"));
    if primary.is_empty() { let legacy = read_safe(&book_dir.join("story/volume_outline.md")); if legacy.is_empty() { fallback.to_string() } else { legacy } } else { primary }
}

pub fn read_character_context(book_dir: &Path, fallback: &str) -> String {
    let roles_dir = book_dir.join("story/roles");
    if !roles_dir.exists() { let legacy = read_safe(&book_dir.join("story/character_matrix.md")); return if legacy.is_empty() { fallback.to_string() } else { legacy }; }
    let mut content = String::new();
    if let Ok(read_dir) = std::fs::read_dir(&roles_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(inner) = std::fs::read_dir(&path) {
                    for file in inner.flatten() {
                        let fp = file.path();
                        if fp.extension().and_then(|e| e.to_str()) == Some("md") {
                            if let Ok(c) = std::fs::read_to_string(&fp) { content.push_str(&c); content.push_str("\n\n"); }
                        }
                    }
                }
            }
        }
    }
    if content.is_empty() { fallback.to_string() } else { content }
}

pub fn is_new_layout_book(book_dir: &Path) -> bool {
    book_dir.join("story/outline/story_frame.md").exists() || book_dir.join("story/outline/volume_map.md").exists()
}

pub fn read_current_state_with_fallback(book_dir: &Path, fallback: &str) -> String {
    let primary = read_safe(&book_dir.join("story/current_state.md"));
    if primary.is_empty() || primary == "(文件尚未创建)" { fallback.to_string() } else { primary }
}

fn read_safe(path: &std::path::Path) -> String { std::fs::read_to_string(path).unwrap_or_default() }
