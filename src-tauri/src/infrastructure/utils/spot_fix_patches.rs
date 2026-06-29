//! Spot-fix patches for targeted text replacements.

use crate::shared::errors::AppError;

/// Apply a spot-fix patch to text
pub fn apply_spot_fix_patch(
    content: &str,
    old_text: &str,
    new_text: &str,
) -> Result<String, AppError> {
    if !content.contains(old_text) {
        return Err(AppError::bad_request(format!(
            "Spot-fix target not found: '{}'",
            &old_text[..50.min(old_text.len())]
        )));
    }

    Ok(content.replacen(old_text, new_text, 1))
}

/// Parse spot-fix patches from a structured format
pub fn parse_spot_fix_patches(text: &str) -> Vec<SpotFixPatch> {
    let mut patches = Vec::new();
    let mut current_old = String::new();
    let mut current_new = String::new();
    let mut in_old = false;
    let mut in_new = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "=== OLD ===" || trimmed == "### OLD" {
            in_old = true;
            in_new = false;
            continue;
        }
        if trimmed == "=== NEW ===" || trimmed == "### NEW" {
            in_new = true;
            in_old = false;
            continue;
        }
        if trimmed.starts_with("=== ") && trimmed.ends_with(" ===") {
            if !current_old.is_empty() && !current_new.is_empty() {
                patches.push(SpotFixPatch {
                    old_text: current_old.trim().to_string(),
                    new_text: current_new.trim().to_string(),
                });
                current_old.clear();
                current_new.clear();
            }
            in_old = false;
            in_new = false;
            continue;
        }

        if in_old {
            if !current_old.is_empty() { current_old.push('\n'); }
            current_old.push_str(line);
        } else if in_new {
            if !current_new.is_empty() { current_new.push('\n'); }
            current_new.push_str(line);
        }
    }

    if !current_old.is_empty() && !current_new.is_empty() {
        patches.push(SpotFixPatch {
            old_text: current_old.trim().to_string(),
            new_text: current_new.trim().to_string(),
        });
    }

    patches
}

pub struct SpotFixPatch {
    pub old_text: String,
    pub new_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_spot_fix() {
        let result = apply_spot_fix_patch("hello world", "world", "rust").unwrap();
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_apply_spot_fix_not_found() {
        let result = apply_spot_fix_patch("hello", "world", "rust");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_spot_fix_patches() {
        let text = "=== OLD ===\nold text\n=== NEW ===\nnew text";
        let patches = parse_spot_fix_patches(text);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].old_text, "old text");
        assert_eq!(patches[0].new_text, "new text");
    }
}
