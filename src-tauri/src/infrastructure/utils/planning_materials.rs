//! Planning materials gathering for the planner agent.

use std::path::Path;

/// Gather materials needed for chapter planning.
pub fn gather_planning_materials(book_dir: &Path, chapter_number: u32) -> PlanningMaterials {
    let story_dir = book_dir.join("story");
    let read_safe = |path: &Path| -> String { std::fs::read_to_string(path).unwrap_or_default() };
    let outline_dir = story_dir.join("outline");
    let volume_map = { let p = read_safe(&outline_dir.join("volume_map.md")); if p.is_empty() { read_safe(&story_dir.join("volume_outline.md")) } else { p } };
    let story_bible = { let p = read_safe(&outline_dir.join("story_frame.md")); if p.is_empty() { read_safe(&story_dir.join("story_bible.md")) } else { p } };

    let previous_ending = if chapter_number > 1 {
        let chapters_dir = book_dir.join("chapters");
        let prefix = format!("{:04}_", chapter_number - 1);
        if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(&prefix) {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        let chars: Vec<char> = content.chars().collect();
                        let len = chars.len();
                        return PlanningMaterials {
                            volume_map, story_bible,
                            current_state: read_safe(&story_dir.join("current_state.md")),
                            pending_hooks: read_safe(&story_dir.join("pending_hooks.md")),
                            chapter_summaries: read_safe(&story_dir.join("chapter_summaries.md")),
                            author_intent: read_safe(&story_dir.join("author_intent.md")),
                            current_focus: read_safe(&story_dir.join("current_focus.md")),
                            character_matrix: read_safe(&story_dir.join("character_matrix.md")),
                            previous_ending: if len > 500 { format!("...{}", chars[len-500..].iter().collect::<String>()) } else { content },
                        };
                    }
                }
            }
        }
        String::new()
    } else { String::new() };

    PlanningMaterials {
        volume_map, story_bible, previous_ending,
        current_state: read_safe(&story_dir.join("current_state.md")),
        pending_hooks: read_safe(&story_dir.join("pending_hooks.md")),
        chapter_summaries: read_safe(&story_dir.join("chapter_summaries.md")),
        author_intent: read_safe(&story_dir.join("author_intent.md")),
        current_focus: read_safe(&story_dir.join("current_focus.md")),
        character_matrix: read_safe(&story_dir.join("character_matrix.md")),
    }
}

pub struct PlanningMaterials {
    pub volume_map: String, pub story_bible: String, pub previous_ending: String,
    pub current_state: String, pub pending_hooks: String, pub chapter_summaries: String,
    pub author_intent: String, pub current_focus: String, pub character_matrix: String,
}
