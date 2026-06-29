//! Language detection utilities.

/// Detect the language of text
pub fn detect_language(text: &str) -> Language {
    let chinese_chars = text.chars().filter(|c| *c >= '\u{4e00}' && *c <= '\u{9fff}').count();
    let total_chars = text.chars().filter(|c| !c.is_whitespace()).count();

    if total_chars == 0 {
        return Language::Zh;
    }

    if chinese_chars as f64 / total_chars as f64 > 0.3 {
        Language::Zh
    } else {
        Language::En
    }
}

/// Detect language from a value (used for normalization)
pub fn infer_language(value: &str) -> String {
    match detect_language(value) {
        Language::Zh => "zh".to_string(),
        Language::En => "en".to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Zh,
    En,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("这是一个中文文本"), Language::Zh);
        assert_eq!(detect_language("This is English text"), Language::En);
        assert_eq!(detect_language(""), Language::Zh);
    }

    #[test]
    fn test_infer_language() {
        assert_eq!(infer_language("中文"), "zh");
        assert_eq!(infer_language("English"), "en");
    }
}
