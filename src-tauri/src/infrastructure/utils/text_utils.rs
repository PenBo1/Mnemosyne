// text_utils 已整合到 crate::shared::text。
//
// 原本的 count_words(content) 单参实现存在混合计数重复计算的问题
// （"你好 world" 会被 split_whitespace 拆成 ["你好", "world"]，导致中文
// 被同时计入 chinese_chars 和 english_words）。统一使用 shared::text 的
// 语言感知实现后，此文件仅保留单参兼容包装。
//
// 新代码应直接使用 crate::shared::text::count_words(text, language)。

pub use crate::shared::text::count_words as count_words_with_lang;

/// 单参兼容包装：按中文感知路径统计字数。
///
/// 等价于 `crate::shared::text::count_words(content, "zh")`。
pub fn count_words(content: &str) -> u32 {
    crate::shared::text::count_words_default(content)
}
