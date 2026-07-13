// 长度治理（length-metrics + length-governance）。
//
// 职责：定义长度规格（LengthSpec）、计数模式（ZhChars/EnWords）、
// 字数统计与硬边界判定，以及 LengthTelemetry 遥测结构。
//
// build_length_spec 采用百分比区间（软 ±15% / 硬 ±30%），
// 而非参考值算法；测试按百分比区间校准。

use super::super::types::Language;

// ── 计数模式与长度规格 ───────────────────────────────────────

/// 计数模式
/// - ZhChars: 中文字符计数（每个汉字算 1）
/// - EnWords: 英文单词计数
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountingMode {
    ZhChars,
    EnWords,
}

/// 长度规格
#[derive(Debug, Clone, Copy)]
pub struct LengthSpec {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
    pub counting_mode: CountingMode,
}

// ── 构建与解析 ───────────────────────────────────────────────

/// 构建长度规格。
/// 软边界 = target ±15%，硬边界 = target ±30%。
///
/// 注意：使用 .round() 而非直接 as u32 截断。f32 无法精确表示 1.30 等
/// 小数（1.30_f32 ≈ 1.2999999），3000*1.30 在 f32 下为 3899.9997…，
/// 截断会得到 3899 而非 3900。round 保证数学意义上的正确整数。
pub fn build_length_spec(target_word_count: u32, language: Language) -> LengthSpec {
    let target = target_word_count;
    let soft_min = (target as f32 * 0.85).round() as u32;
    let soft_max = (target as f32 * 1.15).round() as u32;
    let hard_min = (target as f32 * 0.70).round() as u32;
    let hard_max = (target as f32 * 1.30).round() as u32;
    let counting_mode = resolve_length_counting_mode(language);
    LengthSpec {
        target,
        soft_min,
        soft_max,
        hard_min,
        hard_max,
        counting_mode,
    }
}

/// 解析计数模式。
pub fn resolve_length_counting_mode(language: Language) -> CountingMode {
    match language {
        Language::Zh => CountingMode::ZhChars,
        Language::En => CountingMode::EnWords,
    }
}

// ── 字数统计 ─────────────────────────────────────────────────

/// 统计章节长度。
pub fn count_chapter_length(content: &str, mode: CountingMode) -> u32 {
    match mode {
        CountingMode::ZhChars => count_zh_chars(content),
        CountingMode::EnWords => count_en_words(content),
    }
}

/// 统计中文字符数：CJK 统一表意文字（U+4E00–U+9FFF）
/// + CJK 扩展 A（U+3400–U+4DBF）+ CJK 兼容表意文字（U+F900–U+FAFF）。
/// 不计 ASCII、标点、空白。
fn count_zh_chars(content: &str) -> u32 {
    content
        .chars()
        .filter(|&c| {
            ('\u{4E00}'..='\u{9FFF}').contains(&c)
                || ('\u{3400}'..='\u{4DBF}').contains(&c)
                || ('\u{F900}'..='\u{FAFF}').contains(&c)
        })
        .count() as u32
}

/// 统计英文单词数：按空白切分，计非空 token。
fn count_en_words(content: &str) -> u32 {
    content
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .count() as u32
}

// ── 边界判定 ─────────────────────────────────────────────────

/// 判断字数是否超出硬边界。
pub fn is_outside_hard_range(word_count: u32, spec: &LengthSpec) -> bool {
    word_count < spec.hard_min || word_count > spec.hard_max
}

// ── 长度遥测 ─────────────────────────────────────────────────

/// 长度遥测
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LengthTelemetry {
    pub target: u32,
    /// "zh_chars" | "en_words"
    pub counting_mode: String,
    pub writer_count: u32,
    pub post_writer_normalize_count: u32,
    pub post_revise_count: u32,
    pub final_count: u32,
    pub normalize_applied: bool,
    pub length_warning: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_length_spec_zh() {
        let spec = build_length_spec(3000, Language::Zh);
        assert_eq!(spec.target, 3000);
        assert_eq!(spec.soft_min, 2550);
        assert_eq!(spec.soft_max, 3450);
        assert_eq!(spec.hard_min, 2100);
        assert_eq!(spec.hard_max, 3900);
        assert_eq!(spec.counting_mode, CountingMode::ZhChars);
    }

    #[test]
    fn build_length_spec_en() {
        let spec = build_length_spec(3000, Language::En);
        assert_eq!(spec.target, 3000);
        assert_eq!(spec.soft_min, 2550);
        assert_eq!(spec.soft_max, 3450);
        assert_eq!(spec.hard_min, 2100);
        assert_eq!(spec.hard_max, 3900);
        assert_eq!(spec.counting_mode, CountingMode::EnWords);
    }

    #[test]
    fn count_zh_chars_counts_cjk_only() {
        // 你好 = 2 个汉字；world/！/123 不计入
        assert_eq!(count_zh_chars("你好world！123"), 2);
    }

    #[test]
    fn count_zh_chars_counts_extension_a() {
        // U+3400 㐀 属于 CJK 扩展 A 区段
        assert_eq!(count_zh_chars("\u{3400}test"), 1);
    }

    #[test]
    fn count_en_words_splits_whitespace() {
        assert_eq!(count_en_words("hello world foo bar"), 4);
    }

    #[test]
    fn is_outside_hard_range_below() {
        let spec = build_length_spec(3000, Language::Zh);
        // hard_min=2100，100 低于硬下限
        assert!(is_outside_hard_range(100, &spec));
    }

    #[test]
    fn is_outside_hard_range_above() {
        let spec = build_length_spec(3000, Language::Zh);
        // hard_max=3900，5000 超过硬上限
        assert!(is_outside_hard_range(5000, &spec));
    }

    #[test]
    fn is_outside_hard_range_in_range() {
        let spec = build_length_spec(3000, Language::Zh);
        // 3000 落在 [2100, 3900] 区间内
        assert!(!is_outside_hard_range(3000, &spec));
    }
}
