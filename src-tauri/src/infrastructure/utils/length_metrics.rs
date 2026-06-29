//! Length metrics utilities.

pub type LengthLanguage = &'static str;
pub const LANG_ZH: LengthLanguage = "zh";
pub const LANG_EN: LengthLanguage = "en";

pub fn count_chapter_length(text: &str, counting_mode: &str) -> u32 {
    match counting_mode { "en_words" => text.split_whitespace().count() as u32, _ => count_chinese(text) }
}

fn count_chinese(text: &str) -> u32 {
    let mut count = 0u32;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() {} else if !ch.is_whitespace() { count += 1; }
    }
    let ascii_words: u32 = text.split_whitespace().filter(|w| w.bytes().all(|b| b.is_ascii())).count() as u32;
    count + ascii_words
}

pub fn default_chapter_length(language: &str) -> u32 { match language { "en" => 2000, _ => 3000 } }
pub fn format_length_count(count: u32, language: &str) -> String { match language { "en" => format!("{} words", count), _ => format!("{}字", count) } }
pub fn resolve_length_counting_mode(language: &str) -> &'static str { match language { "en" => "en_words", _ => "zh_chars" } }
