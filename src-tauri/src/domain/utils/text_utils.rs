/// Count words in content (approximation for mixed Chinese/English text).
/// Chinese characters are counted individually; English words by whitespace.
pub fn count_words(content: &str) -> u32 {
    let chinese_chars = content.chars().filter(|c| !c.is_ascii()).count() as u32;
    let english_words = content.split_whitespace().count() as u32;
    chinese_chars + english_words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!(count_words(""), 0);
    }

    #[test]
    fn test_english_only() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("one two three four five"), 5);
    }

    #[test]
    fn test_chinese_only() {
        assert_eq!(count_words("你好世界"), 5);
        assert_eq!(count_words("这是一个测试"), 7);
    }

    #[test]
    fn test_mixed() {
        assert_eq!(count_words("你好 world"), 4);
        assert_eq!(count_words("Hello 世界"), 4);
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(count_words("  hello   world  "), 2);
        assert_eq!(count_words("hello\nworld"), 2);
        assert_eq!(count_words("hello\tworld"), 2);
    }

    #[test]
    fn test_punctuation_counted_as_english() {
        assert_eq!(count_words("hello, world!"), 2);
    }
}
