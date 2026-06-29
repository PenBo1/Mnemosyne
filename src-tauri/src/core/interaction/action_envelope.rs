use serde::{Deserialize, Serialize};

/// Where the action originated from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSource {
    FreeText,
    Button,
    Slash,
    QuickAction,
}

/// Requested intent from UI confirmation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedIntent {
    CreateBook,
    WriteNext,
    ShortRun,
    PlayStart,
    PlayStep,
    GenerateCover,
    EditArtifact,
    FanficInit,
    ContinuationImport,
    SpinoffCreate,
    StyleImitation,
}

/// Create book action payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateBookPayload {
    pub title: Option<String>,
    pub genre: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub target_chapters: Option<u32>,
    pub chapter_word_count: Option<u32>,
}

/// Short fiction run payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortRunPayload {
    pub direction: Option<String>,
    pub reference: Option<String>,
    pub story_id: Option<String>,
    pub chapters: Option<u32>,
    pub chars_per_chapter: Option<u32>,
    pub cover: Option<bool>,
}

/// Play start payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayStartPayload {
    pub title: Option<String>,
    pub premise: Option<String>,
    pub world_contract: Option<String>,
    pub visual_contract: Option<String>,
    pub mode: Option<String>,
    pub initial_scene: Option<String>,
    pub suggested_actions: Option<Vec<String>>,
}

/// Generate cover payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerateCoverPayload {
    pub title: Option<String>,
    pub intro: Option<String>,
    pub selling_points: Option<String>,
    pub cover_prompt: Option<String>,
    pub output_dir: Option<String>,
}

/// Combined action payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionPayload {
    pub create_book: Option<CreateBookPayload>,
    pub short_run: Option<ShortRunPayload>,
    pub play_start: Option<PlayStartPayload>,
    pub generate_cover: Option<GenerateCoverPayload>,
}

/// Check if a play initial scene is usable (not truncated/incomplete)
pub fn is_usable_play_initial_scene(value: Option<&str>) -> bool {
    let text = match value {
        Some(t) => t.trim(),
        None => return false,
    };
    if text.len() < 12 {
        return false;
    }
    // Check for incomplete ending patterns
    let incomplete_patterns = ["叫", "是", "为", "在", "向", "把", "将", "和", "与", "或", "但", "却", "因为", "如果", "当", "等"];
    for pattern in &incomplete_patterns {
        if text.ends_with(pattern) {
            return false;
        }
    }
    true
}

/// Check if instruction is a "write next" command
pub fn is_write_next_instruction(instruction: &str) -> bool {
    let lower = instruction.to_lowercase();
    lower.contains("write next") || lower.contains("next chapter")
        || lower.contains("续写") || lower.contains("下一章")
        || lower.contains("继续写") || lower.contains("write the next")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_source_serialization() {
        let source = ActionSource::Button;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"button\"");
    }

    #[test]
    fn test_is_usable_play_scene() {
        assert!(is_usable_play_initial_scene(Some("你走进了一个神秘的洞穴，四周弥漫着奇异的光芒。")));
        assert!(!is_usable_play_initial_scene(None));
        assert!(!is_usable_play_initial_scene(Some("short")));
        // "叫" is not at the end, so this should be usable
        assert!(is_usable_play_initial_scene(Some("这是一段以叫结尾的文本")));
    }

    #[test]
    fn test_is_write_next_instruction() {
        assert!(is_write_next_instruction("write next chapter"));
        assert!(is_write_next_instruction("续写下一章"));
        assert!(is_write_next_instruction("继续写"));
        assert!(!is_write_next_instruction("create a new book"));
    }

    #[test]
    fn test_action_payload() {
        let payload = ActionPayload {
            create_book: Some(CreateBookPayload {
                title: Some("Test".into()),
                genre: Some("fantasy".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(payload.create_book.is_some());
        assert!(payload.short_run.is_none());
    }
}
