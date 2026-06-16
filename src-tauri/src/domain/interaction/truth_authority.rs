//! Truth authority — determines which truth file is authoritative for a given content.

/// Truth authority level
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruthAuthority {
    /// Source of truth (outline/story_frame.md, outline/volume_map.md, roles/)
    Source,
    /// Derived truth (story_bible.md, character_matrix.md as compat shims)
    Derived,
    /// Runtime truth (current_state.md, pending_hooks.md, chapter_summaries.md)
    Runtime,
    /// User-controlled (author_intent.md, current_focus.md)
    UserControlled,
}

/// Classify the authority of a truth file
pub fn classify_truth_authority(file_path: &str) -> TruthAuthority {
    if file_path.starts_with("outline/") || file_path.starts_with("roles/") {
        TruthAuthority::Source
    } else if file_path.starts_with("story/current_state.md")
        || file_path.starts_with("story/pending_hooks.md")
        || file_path.starts_with("story/chapter_summaries.md")
        || file_path.starts_with("story/subplot_board.md")
        || file_path.starts_with("story/emotional_arcs.md")
    {
        TruthAuthority::Runtime
    } else if file_path.starts_with("story/author_intent.md")
        || file_path.starts_with("story/current_focus.md")
    {
        TruthAuthority::UserControlled
    } else {
        TruthAuthority::Derived
    }
}

/// Normalize a truth file name to a canonical form
pub fn normalize_truth_file_name(name: &str) -> String {
    let lowered = name.to_lowercase();
    match lowered.as_str() {
        "story_frame.md" | "storyframe.md" => "outline/story_frame.md".to_string(),
        "volume_map.md" | "volumemap.md" => "outline/volume_map.md".to_string(),
        "story_bible.md" => "story_bible.md".to_string(),
        "book_rules.md" => "book_rules.md".to_string(),
        "character_matrix.md" => "character_matrix.md".to_string(),
        "current_state.md" => "story/current_state.md".to_string(),
        "pending_hooks.md" => "story/pending_hooks.md".to_string(),
        "chapter_summaries.md" => "story/chapter_summaries.md".to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_authority() {
        assert_eq!(classify_truth_authority("outline/story_frame.md"), TruthAuthority::Source);
        assert_eq!(classify_truth_authority("roles/major/主角.md"), TruthAuthority::Source);
        assert_eq!(classify_truth_authority("story/current_state.md"), TruthAuthority::Runtime);
        assert_eq!(classify_truth_authority("story/pending_hooks.md"), TruthAuthority::Runtime);
        assert_eq!(classify_truth_authority("story/author_intent.md"), TruthAuthority::UserControlled);
        assert_eq!(classify_truth_authority("story/story_bible.md"), TruthAuthority::Derived);
    }

    #[test]
    fn test_normalize_truth_file_name() {
        assert_eq!(normalize_truth_file_name("story_frame.md"), "outline/story_frame.md");
        assert_eq!(normalize_truth_file_name("STORY_FRAME.md"), "outline/story_frame.md");
        assert_eq!(normalize_truth_file_name("book_rules.md"), "book_rules.md");
        assert_eq!(normalize_truth_file_name("unknown.md"), "unknown.md");
    }
}
