use serde_json::Value;
use crate::errors::AppError;

/// Validate that LLM output is valid JSON matching expected structure.
/// Returns Ok(parsed_value) or Err with description of what's wrong.
pub fn validate_json_output(
    content: &str,
    required_fields: &[&str],
) -> Result<Value, AppError> {
    // Strip markdown code fences if present
    let stripped = strip_code_fences(content);

    let value: Value = serde_json::from_str(&stripped)
        .map_err(|e| AppError::internal(format!("Invalid JSON output: {}", e)))?;

    // Check required fields
    if let Some(obj) = value.as_object() {
        for field in required_fields {
            if !obj.contains_key(*field) {
                return Err(AppError::internal(format!(
                    "Missing required field '{}' in LLM output", field
                )));
            }
        }
    }

    Ok(value)
}

/// Validate that output contains expected section markers.
pub fn validate_sections(
    content: &str,
    required_sections: &[&str],
) -> Result<(), AppError> {
    for section in required_sections {
        if !content.contains(section) {
            return Err(AppError::internal(format!(
                "Missing required section '{}' in output", section
            )));
        }
    }
    Ok(())
}

/// Validate word count is within bounds.
pub fn validate_word_count(
    content: &str,
    min: u32,
    max: u32,
) -> Result<u32, AppError> {
    let words = crate::infra::gc::utils::count_words_en(content);
    if words < min {
        return Err(AppError::internal(format!(
            "Output too short: {} words (minimum: {})", words, min
        )));
    }
    if words > max {
        return Err(AppError::internal(format!(
            "Output too long: {} words (maximum: {})", words, max
        )));
    }
    Ok(words)
}

/// Retry-able wrapper: validate output, return error with context for retry.
pub fn validate_with_retry_context(
    content: &str,
    required_fields: &[&str],
    agent_name: &str,
) -> Result<Value, String> {
    match validate_json_output(content, required_fields) {
        Ok(value) => Ok(value),
        Err(e) => Err(format!("[{}] {}", agent_name, e)),
    }
}

/// Strip markdown code fences (```json ... ```) from LLM output.
fn strip_code_fences(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with("```") {
        let after_open = trimmed.lines().skip(1).collect::<Vec<_>>().join("\n");
        if let Some(end) = after_open.rfind("```") {
            after_open[..end].trim().to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_code_fences() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        let result = strip_code_fences(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_validate_json_output() {
        let content = r#"{"title": "Test", "content": "Hello"}"#;
        let result = validate_json_output(content, &["title", "content"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_output_missing_field() {
        let content = r#"{"title": "Test"}"#;
        let result = validate_json_output(content, &["title", "content"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sections() {
        let content = "# Chapter 1\n\n## Plot\n\nContent here";
        assert!(validate_sections(content, &["# Chapter", "## Plot"]).is_ok());
        assert!(validate_sections(content, &["## Characters"]).is_err());
    }
}
