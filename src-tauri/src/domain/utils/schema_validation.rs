//! Runtime schema validation for LLM outputs and data structures.

use serde::{Deserialize, Serialize};
use crate::errors::AppError;

/// Validate a JSON string against expected structure
pub fn validate_json<T: for<'de> Deserialize<'de>>(json_str: &str, context: &str) -> Result<T, AppError> {
    serde_json::from_str(json_str)
        .map_err(|e| AppError::bad_request(format!("{}: JSON parse error: {}", context, e)))
}

/// Validate that required fields exist in a JSON value
pub fn validate_required_fields(value: &serde_json::Value, fields: &[&str], context: &str) -> Result<(), AppError> {
    let missing: Vec<&str> = fields.iter()
        .filter(|f| value.get(*f).is_none())
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::bad_request(format!("{}: missing required fields: {}", context, missing.join(", "))))
    }
}

/// Validate string length constraints
pub fn validate_string_length(value: &str, min: usize, max: usize, field_name: &str) -> Result<(), AppError> {
    if value.len() < min {
        Err(AppError::bad_request(format!("{}: too short ({} < {})", field_name, value.len(), min)))
    } else if value.len() > max {
        Err(AppError::bad_request(format!("{}: too long ({} > {})", field_name, value.len(), max)))
    } else {
        Ok(())
    }
}

/// Validate numeric range
pub fn validate_range(value: f64, min: f64, max: f64, field_name: &str) -> Result<(), AppError> {
    if value < min || value > max {
        Err(AppError::bad_request(format!("{}: out of range ({:.2} not in [{:.2}, {:.2}])", field_name, value, min, max)))
    } else {
        Ok(())
    }
}

/// Validate enum value
pub fn validate_enum(value: &str, allowed: &[&str], field_name: &str) -> Result<(), AppError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(AppError::bad_request(format!("{}: invalid value '{}', allowed: {}", field_name, value, allowed.join(", "))))
    }
}

/// Schema validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub code: String,
}

/// Validate a chapter memo structure
pub fn validate_chapter_memo(memo: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();

    if memo.get("goal").is_none() {
        errors.push(ValidationError { path: "goal".into(), message: "missing required field".into(), code: "required".into() });
    }
    if memo.get("body").is_none() {
        errors.push(ValidationError { path: "body".into(), message: "missing required field".into(), code: "required".into() });
    }

    if let Some(goal) = memo.get("goal").and_then(|v| v.as_str()) {
        if goal.len() > 50 {
            errors.push(ValidationError { path: "goal".into(), message: format!("too long ({} > 50)", goal.len()), code: "max_length".into() });
        }
    }

    ValidationResult { valid: errors.is_empty(), errors }
}

/// Validate audit result structure
pub fn validate_audit_result(result: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();

    if result.get("passed").is_none() {
        errors.push(ValidationError { path: "passed".into(), message: "missing required field".into(), code: "required".into() });
    }
    if result.get("overall_score").is_none() && result.get("score").is_none() {
        errors.push(ValidationError { path: "score".into(), message: "missing score field".into(), code: "required".into() });
    }
    if let Some(issues) = result.get("issues").and_then(|v| v.as_array()) {
        for (i, issue) in issues.iter().enumerate() {
            if issue.get("severity").is_none() {
                errors.push(ValidationError { path: format!("issues[{}].severity", i), message: "missing severity".into(), code: "required".into() });
            }
            if issue.get("category").is_none() {
                errors.push(ValidationError { path: format!("issues[{}].category", i), message: "missing category".into(), code: "required".into() });
            }
        }
    }

    ValidationResult { valid: errors.is_empty(), errors }
}

/// Validate book config structure
pub fn validate_book_config(config: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();

    let required = ["id", "title", "genre", "platform", "language"];
    for field in &required {
        if config.get(*field).is_none() {
            errors.push(ValidationError { path: field.to_string(), message: "missing required field".into(), code: "required".into() });
        }
    }

    if let Some(chapter_words) = config.get("chapter_words").and_then(|v| v.as_u64()) {
        if chapter_words == 0 || chapter_words > 50000 {
            errors.push(ValidationError { path: "chapter_words".into(), message: format!("invalid value: {}", chapter_words), code: "range".into() });
        }
    }

    ValidationResult { valid: errors.is_empty(), errors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_json_valid() {
        let json = r#"{"name": "test", "value": 42}"#;
        let result: serde_json::Value = validate_json(json, "test").unwrap();
        assert_eq!(result["name"], "test");
    }

    #[test]
    fn test_validate_json_invalid() {
        let result: Result<serde_json::Value, _> = validate_json("not json", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_required_fields() {
        let json = serde_json::json!({"name": "test"});
        assert!(validate_required_fields(&json, &["name"], "test").is_ok());
        assert!(validate_required_fields(&json, &["name", "value"], "test").is_err());
    }

    #[test]
    fn test_validate_string_length() {
        assert!(validate_string_length("hello", 1, 10, "test").is_ok());
        assert!(validate_string_length("hi", 3, 10, "test").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(5.0, 0.0, 10.0, "test").is_ok());
        assert!(validate_range(-1.0, 0.0, 10.0, "test").is_err());
    }

    #[test]
    fn test_validate_chapter_memo() {
        let valid = serde_json::json!({"goal": "test", "body": "body"});
        assert!(validate_chapter_memo(&valid).valid);
        let invalid = serde_json::json!({});
        assert!(!validate_chapter_memo(&invalid).valid);
    }

    #[test]
    fn test_validate_book_config() {
        let valid = serde_json::json!({"id": "1", "title": "test", "genre": "fantasy", "platform": "local", "language": "zh"});
        assert!(validate_book_config(&valid).valid);
        let invalid = serde_json::json!({});
        assert!(!validate_book_config(&invalid).valid);
    }
}
