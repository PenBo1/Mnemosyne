// ============================================================================
// text —— 跨层共享的文本处理纯函数
// ============================================================================
//
// 下沉理由：count_words 被 infrastructure（utils/text_utils.rs、state_store/gc.rs、
// db/wiki_store.rs）、features（version/service.rs）、core/agent（length_normalizer、
// writer、verification、reviser、pipeline）同时需要，原本存在 4 个不一致的实现
// （其中 features/version/service.rs 的实现甚至是错的——把 ASCII 字符当作中文统计）。
// 提升到 shared/text.rs 作为唯一实现，消除重复与 bug。
//
// 仅含无副作用纯函数，无业务逻辑、无 I/O。

/// 按语言感知方式统计文本字数。
///
/// - `language == "en"`：按空白分隔的英文单词数。
/// - 其他（含 `"zh"` 与默认）：非 ASCII 非空白字符数（中文字符逐个计数）+ 纯 ASCII 单词数。
///
/// 这是对原 `infrastructure/state_store/gc.rs::utils::count_words` 的提升，
/// 替代了 `infrastructure/utils/text_utils.rs`（混合计数会重复计算）与
/// `features/version/service.rs`（实现错误：把 ASCII 字符当作中文）的私有副本。
pub fn count_words(text: &str, language: &str) -> u32 {
    if language == "en" {
        text.split_whitespace().count() as u32
    } else {
        let mut non_ascii = 0u32;
        for ch in text.chars() {
            if !ch.is_ascii() && !ch.is_whitespace() {
                non_ascii += 1;
            }
        }
        let ascii_words: u32 = text.split_whitespace()
            .filter(|w| w.bytes().all(|b| b.is_ascii()))
            .count() as u32;
        non_ascii + ascii_words
    }
}

/// 统计字数（默认中文感知路径）。
///
/// 适用于不知道语言、希望按混合中英文场景估算字数的调用点。
/// 等价于 `count_words(text, "zh")`。
pub fn count_words_default(text: &str) -> u32 {
    count_words(text, "zh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!(count_words("", "en"), 0);
        assert_eq!(count_words("", "zh"), 0);
    }

    #[test]
    fn test_english_only() {
        assert_eq!(count_words("hello world", "en"), 2);
        assert_eq!(count_words("one two three four five", "en"), 5);
    }

    #[test]
    fn test_chinese_only() {
        assert_eq!(count_words("你好世界", "zh"), 4);
        // "这是一个测试" 共 6 个中文字符，无 ASCII 单词
        assert_eq!(count_words("这是一个测试", "zh"), 6);
    }

    #[test]
    fn test_mixed() {
        // "你好" 2 个非 ASCII 字符 + "world" 1 个 ASCII 单词 = 3
        assert_eq!(count_words("你好 world", "zh"), 3);
        // "Hello" 1 个 ASCII 单词 + "世界" 2 个非 ASCII 字符 = 3
        assert_eq!(count_words("Hello 世界", "zh"), 3);
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(count_words("  hello   world  ", "en"), 2);
        assert_eq!(count_words("hello\nworld", "en"), 2);
        assert_eq!(count_words("hello\tworld", "en"), 2);
    }

    #[test]
    fn test_punctuation_counted_as_english() {
        assert_eq!(count_words("hello, world!", "en"), 2);
    }
}
