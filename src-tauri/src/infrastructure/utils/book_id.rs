//! Book ID utilities.

use regex::Regex;

/// Derive a book ID from a title
pub fn derive_book_id_from_title(title: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9\u{4e00}-\u{9fff}]+").unwrap();
    let normalized = re.replace_all(title, "-").trim_matches('-').to_lowercase();
    if normalized.is_empty() {
        format!("book-{}", chrono::Utc::now().timestamp())
    } else {
        normalized
    }
}

/// Check if a book ID is safe (no traversal, no special chars)
pub fn is_safe_book_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 100 { return false; }
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('\0') { return false; }
    true
}

/// Assert a book ID is safe, or return error
pub fn assert_safe_book_id(id: &str, context: &str) -> Result<String, crate::shared::errors::AppError> {
    if is_safe_book_id(id) {
        Ok(id.to_string())
    } else {
        Err(crate::shared::errors::AppError::bad_request(format!("{}: invalid book id '{}'", context, id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_book_id() {
        assert_eq!(derive_book_id_from_title("Test Book"), "test-book");
        assert_eq!(derive_book_id_from_title("测试小说"), "测试小说");
        assert_eq!(derive_book_id_from_title("a/b/c"), "a-b-c");
    }

    #[test]
    fn test_is_safe_book_id() {
        assert!(is_safe_book_id("test-book"));
        assert!(is_safe_book_id("测试"));
        assert!(!is_safe_book_id("../etc"));
        assert!(!is_safe_book_id(""));
        assert!(!is_safe_book_id("a/b"));
    }
}
