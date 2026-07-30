//! ═══════════════════════════════════════════════════════════════════════════
//! Memory 验证 - 参数验证逻辑
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::AppError;
use crate::infrastructure::fs::fs_utils::validate_id_component;

// ── 日期验证 ────────────────────────────────────────────────────────────────

/// 验证 YYYY-MM-DD 格式日期
///
/// # 参数
/// - `s`: 日期字符串
/// - `field`: 字段名（用于错误消息）
///
/// # 返回值
/// 成功返回 Ok(())，失败返回错误
pub fn validate_date(s: &str, field: &str) -> Result<(), AppError> {
    if s.len() != 10
        || s.chars().nth(4) != Some('-')
        || s.chars().nth(7) != Some('-')
        || !s[0..4].chars().all(|c| c.is_ascii_digit())
        || !s[5..7].chars().all(|c| c.is_ascii_digit())
        || !s[8..10].chars().all(|c| c.is_ascii_digit())
    {
        return Err(AppError::invalid_input(format!(
            "Invalid {} format, expected YYYY-MM-DD, got: {}",
            field, s
        )));
    }
    Ok(())
}

/// 验证日期范围（start_date <= end_date）
pub fn validate_date_range(start_date: &str, end_date: &str) -> Result<(), AppError> {
    if start_date > end_date {
        return Err(AppError::invalid_input("start_date must be <= end_date"));
    }
    Ok(())
}

// ── ID 验证 ──────────────────────────────────────────────────────────────────

/// 验证 session_id
pub fn validate_session_id(session_id: &str) -> Result<(), AppError> {
    validate_id_component(session_id, "session_id")
}

/// 验证 book_id
pub fn validate_book_id(book_id: &str) -> Result<(), AppError> {
    validate_id_component(book_id, "book_id")
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_date_valid() {
        assert!(validate_date("2024-01-15", "test").is_ok());
        assert!(validate_date("2024-12-31", "test").is_ok());
    }

    #[test]
    fn test_validate_date_invalid() {
        assert!(validate_date("2024-1-15", "test").is_err());
        assert!(validate_date("2024/01/15", "test").is_err());
        assert!(validate_date("24-01-15", "test").is_err());
        assert!(validate_date("", "test").is_err());
    }

    #[test]
    fn test_validate_date_range_valid() {
        assert!(validate_date_range("2024-01-01", "2024-01-31").is_ok());
        assert!(validate_date_range("2024-01-15", "2024-01-15").is_ok());
    }

    #[test]
    fn test_validate_date_range_invalid() {
        assert!(validate_date_range("2024-01-31", "2024-01-01").is_err());
    }
}