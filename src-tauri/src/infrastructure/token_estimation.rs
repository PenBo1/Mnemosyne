//! ═══════════════════════════════════════════════════════════════════════════
//! Token 估算 - 字符数估算工具
//! ═══════════════════════════════════════════════════════════════════════════

pub const BYTES_PER_TOKEN: u64 = 4;
pub const IMAGE_TOKEN_ESTIMATE: u64 = 765;

pub fn estimate_tokens(s: &str) -> u64 {
    (s.len() as u64) / BYTES_PER_TOKEN
}

pub fn estimate_message_tokens(content: &str, has_images: bool) -> u64 {
    let text_tokens = estimate_tokens(content);
    if has_images {
        text_tokens + IMAGE_TOKEN_ESTIMATE
    } else {
        text_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_exact_multiple() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn test_estimate_tokens_partial() {
        assert_eq!(estimate_tokens("abc"), 0);
        assert_eq!(estimate_tokens("abcde"), 1);
    }

    #[test]
    fn test_estimate_message_tokens_no_images() {
        assert_eq!(estimate_message_tokens("abcdefgh", false), 2);
    }

    #[test]
    fn test_estimate_message_tokens_with_images() {
        let expected = 2 + IMAGE_TOKEN_ESTIMATE;
        assert_eq!(estimate_message_tokens("abcdefgh", true), expected);
    }

    #[test]
    fn test_estimate_message_tokens_empty_with_images() {
        assert_eq!(estimate_message_tokens("", true), IMAGE_TOKEN_ESTIMATE);
    }

    #[test]
    fn test_constants() {
        assert_eq!(BYTES_PER_TOKEN, 4);
        assert_eq!(IMAGE_TOKEN_ESTIMATE, 765);
    }
}