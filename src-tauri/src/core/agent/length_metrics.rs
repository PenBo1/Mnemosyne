//! 字数治理纯函数与类型。
//!
//! 移植自 inkos `length-metrics.ts` + `models/length-governance.ts`。
//! 与 `length_normalizer.rs` 的归一化逻辑配合使用：
//! - `LengthSpec` 定义三层区间（target / soft / hard）+ 计长模式 + 归一化模式
//! - `count_chapter_length` 按 counting_mode 中英文分治计长（先剥 markdown metadata）
//! - `build_length_spec` 按 target 比例推导 soft/hard delta
//! - `choose_normalize_mode` 根据当前字数自动选择 expand / compress / none

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 常量
// ════════════════════════════════════════════════════════════════════

/// 参考目标字数（与 inkos REFERENCE_TARGET 一致）。
///
/// soft/hard delta 按 `target * DELTA / REFERENCE_TARGET` 比例缩放，
/// 让 1000 字短章和 5000 字长章都有合理的相对区间。
const REFERENCE_TARGET: u32 = 2200;
const SOFT_RANGE_DELTA: u32 = 300;
const HARD_RANGE_DELTA: u32 = 600;

pub const DEFAULT_CHAPTER_LENGTH_ZH: u32 = 3000;
pub const DEFAULT_CHAPTER_LENGTH_EN: u32 = 2000;

/// 按语言返回默认章节目标字数。
pub fn default_chapter_length(language: &str) -> u32 {
    if language == "en" { DEFAULT_CHAPTER_LENGTH_EN } else { DEFAULT_CHAPTER_LENGTH_ZH }
}

// ════════════════════════════════════════════════════════════════════
// 枚举
// ════════════════════════════════════════════════════════════════════

/// 字数计长模式。
///
/// - `ZhChars`：中文按字符计长（剔除所有空白）
/// - `EnWords`：英文按单词计长（含数字、缩写如 don't）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthCountingMode {
    ZhChars,
    EnWords,
}

/// 字数归一化模式。
///
/// - `Expand`：扩写（当前字数 < softMin）
/// - `Compress`：压缩（当前字数 > softMax）
/// - `None`：无需归一化（在 soft 区间内）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthNormalizeMode {
    Expand,
    Compress,
    None,
}

// ════════════════════════════════════════════════════════════════════
// LengthSpec
// ════════════════════════════════════════════════════════════════════

/// 字数治理三层区间规范。
///
/// - `target`：目标字数
/// - `soft_min`/`soft_max`：允许区间（给 LLM 看，超出触发归一化）
/// - `hard_min`/`hard_max`：极限区间（pipeline 内部判断，超出触发硬门失败）
/// - `counting_mode`：中英文计长模式
/// - `normalize_mode`：归一化模式（None 表示由 `choose_normalize_mode` 自动决定）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthSpec {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
    pub counting_mode: LengthCountingMode,
    pub normalize_mode: LengthNormalizeMode,
}

impl LengthSpec {
    /// 按 target 字数 + 语言构造 LengthSpec。
    ///
    /// soft/hard delta 按 `target * DELTA / REFERENCE_TARGET` 比例缩放，
    /// 保证短章和长章都有合理的相对区间。
    pub fn build(target: u32, language: &str) -> Self {
        let soft_delta = scale_range_delta(target, SOFT_RANGE_DELTA);
        let hard_delta = std::cmp::max(soft_delta, scale_range_delta(target, HARD_RANGE_DELTA));
        Self {
            target,
            soft_min: target.saturating_sub(soft_delta).max(1),
            soft_max: target + soft_delta,
            hard_min: target.saturating_sub(hard_delta).max(1),
            hard_max: target + hard_delta,
            counting_mode: resolve_length_counting_mode(language),
            normalize_mode: LengthNormalizeMode::None,
        }
    }

    /// 区间检查：判断字数处于哪个区间。
    pub fn check(&self, count: u32) -> LengthCheck {
        if count < self.hard_min {
            LengthCheck::TooShort
        } else if count > self.hard_max {
            LengthCheck::TooLong
        } else if count < self.soft_min || count > self.soft_max {
            LengthCheck::OutsideSoft
        } else {
            LengthCheck::Ok
        }
    }
}

/// 字数区间检查结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthCheck {
    /// 在 soft 区间内（理想）
    Ok,
    /// 在 hard 区间内但超出 soft（可接受）
    OutsideSoft,
    /// 低于 hard_min（触发归一化）
    TooShort,
    /// 超过 hard_max（触发归一化）
    TooLong,
}

// ════════════════════════════════════════════════════════════════════
// LengthTelemetry / LengthWarning
// ════════════════════════════════════════════════════════════════════

/// 字数治理全链路遥测。
///
/// 记录 writerCount → postWriterNormalizeCount → postReviseCount → finalCount，
/// 便于排查字数漂移问题。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthTelemetry {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
    pub counting_mode: LengthCountingMode,
    pub writer_count: u32,
    pub post_writer_normalize_count: u32,
    pub post_revise_count: u32,
    pub final_count: u32,
    pub normalize_applied: bool,
    pub length_warning: bool,
}

/// 章节级字数警告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthWarning {
    pub chapter: u32,
    pub target: u32,
    pub actual: u32,
    pub counting_mode: LengthCountingMode,
    pub reason: String,
}

// ════════════════════════════════════════════════════════════════════
// 计长函数
// ════════════════════════════════════════════════════════════════════

/// 按计长模式统计章节字数。
///
/// 计长前先 `strip_markdown_metadata` 剥离 frontmatter / 代码块 / 标题 / 分隔线，
/// 避免非正文内容干扰字数统计。
pub fn count_chapter_length(content: &str, counting_mode: LengthCountingMode) -> u32 {
    let normalized = strip_markdown_metadata(content);
    match counting_mode {
        LengthCountingMode::EnWords => {
            // 英文按单词计长（含数字、缩写如 don't）
            // 正则与 inkos 一致：[A-Za-z0-9]+(?:'[A-Za-z0-9]+)?
            let regex = regex::Regex::new(r"[A-Za-z0-9]+(?:'[A-Za-z0-9]+)?").unwrap();
            regex.find_iter(&normalized).count() as u32
        }
        LengthCountingMode::ZhChars => {
            // 中文按字符计长（剔除所有空白）
            normalized.chars().filter(|c| !c.is_whitespace()).count() as u32
        }
    }
}

/// 按语言解析计长模式。
pub fn resolve_length_counting_mode(language: &str) -> LengthCountingMode {
    if language == "en" {
        LengthCountingMode::EnWords
    } else {
        LengthCountingMode::ZhChars
    }
}

/// 格式化字数显示（带单位）。
pub fn format_length_count(count: u32, counting_mode: LengthCountingMode) -> String {
    match counting_mode {
        LengthCountingMode::EnWords => format!("{} words", count),
        LengthCountingMode::ZhChars => format!("{}字", count),
    }
}

// ════════════════════════════════════════════════════════════════════
// 区间判断与模式选择
// ════════════════════════════════════════════════════════════════════

/// 判断字数是否超出 soft 区间。
pub fn is_outside_soft_range(count: u32, spec: &LengthSpec) -> bool {
    count < spec.soft_min || count > spec.soft_max
}

/// 判断字数是否超出 hard 区间。
pub fn is_outside_hard_range(count: u32, spec: &LengthSpec) -> bool {
    count < spec.hard_min || count > spec.hard_max
}

/// 根据当前字数自动选择归一化模式。
///
/// - `< soft_min` → `Expand`
/// - `> soft_max` → `Compress`
/// - 否则 → `None`
pub fn choose_normalize_mode(count: u32, spec: &LengthSpec) -> LengthNormalizeMode {
    if count < spec.soft_min {
        LengthNormalizeMode::Expand
    } else if count > spec.soft_max {
        LengthNormalizeMode::Compress
    } else {
        LengthNormalizeMode::None
    }
}

// ════════════════════════════════════════════════════════════════════
// 内部辅助函数
// ════════════════════════════════════════════════════════════════════

/// 按 target 比例缩放 delta。
///
/// `max(1, floor(target * reference_delta / REFERENCE_TARGET))`
fn scale_range_delta(target: u32, reference_delta: u32) -> u32 {
    std::cmp::max(1, (target as u64 * reference_delta as u64 / REFERENCE_TARGET as u64) as u32)
}

/// 剥离 markdown metadata，只保留正文行。
///
/// 剥离内容：
/// - 文件首部 `---` frontmatter（到下一个 `---`）
/// - ` ``` `/`~~~` 围起的代码块
/// - `#/##/...` 标题行
/// - 单独的 `---` / `...` 分隔行
fn strip_markdown_metadata(content: &str) -> String {
    // 统一换行符，剥 BOM
    let normalized = content.replace("\r\n", "\n");
    let normalized = normalized.trim_start_matches('\u{FEFF}');
    let lines: Vec<&str> = normalized.split('\n').collect();

    let mut prose_lines: Vec<&str> = Vec::new();
    let mut index = 0;

    // 跳过 frontmatter
    if lines.first().map(|l| l.trim() == "---").unwrap_or(false) {
        index += 1;
        while index < lines.len() && lines[index].trim() != "---" {
            index += 1;
        }
        if index < lines.len() {
            index += 1; // 跳过闭合的 ---
        }
    }

    let mut in_fence = false;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();

        // 代码块围栏切换
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            index += 1;
            continue;
        }
        if in_fence {
            index += 1;
            continue;
        }
        // 标题行（#/##/...）
        if is_markdown_heading(trimmed) {
            index += 1;
            continue;
        }
        // 分隔线
        if trimmed == "---" || trimmed == "..." {
            index += 1;
            continue;
        }

        prose_lines.push(line);
        index += 1;
    }

    prose_lines.join("\n")
}

/// 判断是否为 markdown 标题行（#/##/.../###### 后跟空白）。
fn is_markdown_heading(trimmed: &str) -> bool {
    let mut hash_count = 0;
    for c in trimmed.chars() {
        if c == '#' {
            hash_count += 1;
            if hash_count > 6 {
                return false;
            }
        } else if c.is_whitespace() {
            return hash_count >= 1;
        } else {
            return false;
        }
    }
    false
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── LengthSpec::build ───────────────────────────────────────

    #[test]
    fn build_length_spec_zh_default() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(spec.target, 3000);
        assert_eq!(spec.counting_mode, LengthCountingMode::ZhChars);
        assert_eq!(spec.normalize_mode, LengthNormalizeMode::None);
        // soft_delta = max(1, floor(3000 * 300 / 2200)) = max(1, 409) = 409
        assert_eq!(spec.soft_min, 3000 - 409);
        assert_eq!(spec.soft_max, 3000 + 409);
        // hard_delta = max(409, floor(3000 * 600 / 2200)) = max(409, 818) = 818
        assert_eq!(spec.hard_min, 3000 - 818);
        assert_eq!(spec.hard_max, 3000 + 818);
    }

    #[test]
    fn build_length_spec_en_default() {
        let spec = LengthSpec::build(2000, "en");
        assert_eq!(spec.target, 2000);
        assert_eq!(spec.counting_mode, LengthCountingMode::EnWords);
        // soft_delta = max(1, floor(2000 * 300 / 2200)) = max(1, 272) = 272
        assert_eq!(spec.soft_min, 2000 - 272);
        assert_eq!(spec.soft_max, 2000 + 272);
    }

    #[test]
    fn build_length_spec_small_target_ensures_min_delta_1() {
        let spec = LengthSpec::build(100, "zh");
        // soft_delta = max(1, floor(100 * 300 / 2200)) = max(1, 13) = 13
        assert_eq!(spec.soft_min, 100 - 13);
        assert_eq!(spec.soft_max, 100 + 13);
    }

    // ── LengthSpec::check ───────────────────────────────────────

    #[test]
    fn check_ok_when_within_soft_range() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(spec.check(3000), LengthCheck::Ok);
        assert_eq!(spec.check(spec.soft_min), LengthCheck::Ok);
        assert_eq!(spec.check(spec.soft_max), LengthCheck::Ok);
    }

    #[test]
    fn check_outside_soft_when_within_hard_range() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(spec.check(spec.soft_min - 1), LengthCheck::OutsideSoft);
        assert_eq!(spec.check(spec.soft_max + 1), LengthCheck::OutsideSoft);
        assert_eq!(spec.check(spec.hard_min), LengthCheck::OutsideSoft);
        assert_eq!(spec.check(spec.hard_max), LengthCheck::OutsideSoft);
    }

    #[test]
    fn check_too_short_when_below_hard_min() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(spec.check(spec.hard_min - 1), LengthCheck::TooShort);
        assert_eq!(spec.check(0), LengthCheck::TooShort);
    }

    #[test]
    fn check_too_long_when_above_hard_max() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(spec.check(spec.hard_max + 1), LengthCheck::TooLong);
    }

    // ── count_chapter_length ────────────────────────────────────

    #[test]
    fn count_zh_chars_strips_markdown_metadata() {
        let content = "---\ntitle: 测试\n---\n\n# 标题\n\n正文内容。\n\n```\ncode\n```\n\n更多正文。";
        let count = count_chapter_length(content, LengthCountingMode::ZhChars);
        // 正文 = "正文内容。" + "更多正文。" = 10 字符（去掉空白和标点外的内容）
        // "正文内容。" = 5 字符，"更多正文。" = 5 字符，共 10 字符
        assert_eq!(count, 10);
    }

    #[test]
    fn count_zh_chars_plain_text() {
        let count = count_chapter_length("你好世界", LengthCountingMode::ZhChars);
        assert_eq!(count, 4);
    }

    #[test]
    fn count_zh_chars_ignores_whitespace() {
        let count = count_chapter_length("你 好 \n世 界", LengthCountingMode::ZhChars);
        assert_eq!(count, 4);
    }

    #[test]
    fn count_en_words_with_apostrophe() {
        let count = count_chapter_length("don't stop believing", LengthCountingMode::EnWords);
        // don't, stop, believing = 3 words
        assert_eq!(count, 3);
    }

    #[test]
    fn count_en_words_with_numbers() {
        let count = count_chapter_length("hello 123 world 456", LengthCountingMode::EnWords);
        // hello, 123, world, 456 = 4 words
        assert_eq!(count, 4);
    }

    #[test]
    fn count_en_words_strips_markdown_metadata() {
        let content = "---\ntitle: Test\n---\n\n# Heading\n\nHello world.\n\n```\ncode block\n```\n\nMore text.";
        let count = count_chapter_length(content, LengthCountingMode::EnWords);
        // 正文 = "Hello world." + "More text." = Hello, world, More, text = 4 words
        assert_eq!(count, 4);
    }

    // ── strip_markdown_metadata ─────────────────────────────────

    #[test]
    fn strip_frontmatter() {
        let content = "---\ntitle: Test\n---\nBody text.";
        let stripped = strip_markdown_metadata(content);
        assert_eq!(stripped, "Body text.");
    }

    #[test]
    fn strip_code_blocks() {
        let content = "Before\n```\ncode\n```\nAfter";
        let stripped = strip_markdown_metadata(content);
        assert_eq!(stripped, "Before\nAfter");
    }

    #[test]
    fn strip_headings() {
        let content = "# Title\n\nSome text.\n\n## Subtitle\n\nMore text.";
        let stripped = strip_markdown_metadata(content);
        assert_eq!(stripped, "\nSome text.\n\n\nMore text.");
    }

    #[test]
    fn strip_separators() {
        let content = "Before\n---\nAfter";
        let stripped = strip_markdown_metadata(content);
        assert_eq!(stripped, "Before\nAfter");
    }

    #[test]
    fn strip_no_metadata_returns_unchanged() {
        let content = "Just plain text.";
        let stripped = strip_markdown_metadata(content);
        assert_eq!(stripped, "Just plain text.");
    }

    #[test]
    fn strip_handles_crlf_and_bom() {
        let content = "\u{FEFF}---\r\ntitle: Test\r\n---\r\nBody.";
        let stripped = strip_markdown_metadata(content);
        assert_eq!(stripped, "Body.");
    }

    // ── is_outside_soft/hard_range ──────────────────────────────

    #[test]
    fn is_outside_soft_range_boundary() {
        let spec = LengthSpec::build(3000, "zh");
        assert!(!is_outside_soft_range(spec.soft_min, &spec));
        assert!(!is_outside_soft_range(spec.soft_max, &spec));
        assert!(is_outside_soft_range(spec.soft_min - 1, &spec));
        assert!(is_outside_soft_range(spec.soft_max + 1, &spec));
    }

    #[test]
    fn is_outside_hard_range_boundary() {
        let spec = LengthSpec::build(3000, "zh");
        assert!(!is_outside_hard_range(spec.hard_min, &spec));
        assert!(!is_outside_hard_range(spec.hard_max, &spec));
        assert!(is_outside_hard_range(spec.hard_min - 1, &spec));
        assert!(is_outside_hard_range(spec.hard_max + 1, &spec));
    }

    // ── choose_normalize_mode ───────────────────────────────────

    #[test]
    fn choose_normalize_expand_when_below_soft_min() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(choose_normalize_mode(spec.soft_min - 1, &spec), LengthNormalizeMode::Expand);
        assert_eq!(choose_normalize_mode(0, &spec), LengthNormalizeMode::Expand);
    }

    #[test]
    fn choose_normalize_compress_when_above_soft_max() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(choose_normalize_mode(spec.soft_max + 1, &spec), LengthNormalizeMode::Compress);
    }

    #[test]
    fn choose_normalize_none_when_within_soft_range() {
        let spec = LengthSpec::build(3000, "zh");
        assert_eq!(choose_normalize_mode(spec.target, &spec), LengthNormalizeMode::None);
        assert_eq!(choose_normalize_mode(spec.soft_min, &spec), LengthNormalizeMode::None);
        assert_eq!(choose_normalize_mode(spec.soft_max, &spec), LengthNormalizeMode::None);
    }

    // ── 辅助函数 ────────────────────────────────────────────────

    #[test]
    fn resolve_length_counting_mode_by_language() {
        assert_eq!(resolve_length_counting_mode("zh"), LengthCountingMode::ZhChars);
        assert_eq!(resolve_length_counting_mode("en"), LengthCountingMode::EnWords);
        assert_eq!(resolve_length_counting_mode("fr"), LengthCountingMode::ZhChars); // 默认中文
    }

    #[test]
    fn format_length_count_with_unit() {
        assert_eq!(format_length_count(3000, LengthCountingMode::ZhChars), "3000字");
        assert_eq!(format_length_count(2000, LengthCountingMode::EnWords), "2000 words");
    }

    #[test]
    fn default_chapter_length_by_language() {
        assert_eq!(default_chapter_length("zh"), 3000);
        assert_eq!(default_chapter_length("en"), 2000);
        assert_eq!(default_chapter_length("fr"), 3000); // 默认中文
    }

    #[test]
    fn is_markdown_heading_detection() {
        assert!(is_markdown_heading("# Title"));
        assert!(is_markdown_heading("## Section"));
        assert!(is_markdown_heading("###### Deep heading"));
        assert!(!is_markdown_heading("####### Too many hashes"));
        assert!(!is_markdown_heading("#NoSpace"));
        assert!(!is_markdown_heading("Plain text"));
        assert!(!is_markdown_heading(""));
    }
}
