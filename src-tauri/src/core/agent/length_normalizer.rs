//! 章节字数归一化 Agent。
//!
//! 移植自 inkos `agents/length-normalizer.ts`。核心机制：
//! - **单次修正**：只能执行一次 LLM 归一化，不得递归重写
//! - **三重安全网**：
//!   1. `sanitize_normalized_content`：剥离 LLM 包装文本（```fence / "下面是正文" / "I'll rewrite" 等）
//!   2. `looks_truncated`：检测 LLM 输出是否在句中截断
//!   3. `crosses_opposite_hard_bound`：检测过度修正穿越到相反硬边界
//! - **安全网触发回退**：任一安全网触发时保留原文，避免章节被破坏
//! - **归一化模式**：根据 `spec.normalize_mode` 或 `choose_normalize_mode` 自动决定 expand/compress
//!
//! LengthSpec/LengthCheck 等类型定义在 `length_metrics.rs`，本模块只负责归一化逻辑。

use async_trait::async_trait;
use regex::Regex;
use crate::shared::errors::AppError;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::length_metrics::{
    LengthSpec, LengthNormalizeMode,
    count_chapter_length, choose_normalize_mode,
    is_outside_soft_range, is_outside_hard_range,
};

pub struct LengthNormalizerAgent;

impl Default for LengthNormalizerAgent {
    fn default() -> Self { Self }
}

impl LengthNormalizerAgent {
    pub fn new() -> Self { Self }

    /// 对章节正文做一次单次字数归一化。
    ///
    /// 流程：
    /// 1. 用 `count_chapter_length` 统计原始字数（剥 markdown metadata）
    /// 2. 决定归一化模式：`spec.normalize_mode` 非 None 时用之，否则 `choose_normalize_mode`
    /// 3. 模式为 None → 直接返回原文
    /// 4. LLM 单次修正（system + user prompt）
    /// 5. 三重安全网校验：sanitize → truncation → hard bound crossing
    /// 6. 任一安全网触发 → 保留原文，标记 warning
    /// 7. 返回 `NormalizeOutput`（含 mode 和 warning）
    pub async fn normalize(
        &self,
        ctx: &AgentContext,
        content: &str,
        spec: &LengthSpec,
        language: &str,
    ) -> Result<NormalizeOutput, AppError> {
        let original_count = count_chapter_length(content, spec.counting_mode);

        // 决定归一化模式：显式指定优先，否则按当前字数自动选择
        let mode = if spec.normalize_mode != LengthNormalizeMode::None {
            spec.normalize_mode
        } else {
            choose_normalize_mode(original_count, spec)
        };

        if mode == LengthNormalizeMode::None {
            return Ok(NormalizeOutput {
                content: content.to_string(),
                word_count: original_count,
                applied: false,
                mode,
                warning: None,
            });
        }

        let system = build_system_prompt(mode, language);
        let user = build_user_prompt(content, spec, original_count, mode, language);
        let response = self.chat(ctx, &system, &user).await?;

        // ── 三重安全网 ──────────────────────────────────────────
        let sanitized = sanitize_normalized_content(&response.content, content);
        let sanitized_count = count_chapter_length(&sanitized, spec.counting_mode);

        let was_truncated = sanitized != content
            && sanitized_count < spec.hard_min
            && looks_truncated(&sanitized);

        let crossed_hard_range = sanitized != content
            && crosses_opposite_hard_bound(original_count, sanitized_count, spec);

        let (final_content, warning) = if was_truncated || crossed_hard_range {
            // 安全网触发：保留原文
            let warning_msg = if was_truncated {
                "Length normalizer output appeared truncated; kept original chapter.".to_string()
            } else {
                "Length normalizer output crossed the hard range; kept original chapter.".to_string()
            };
            (content.to_string(), Some(warning_msg))
        } else {
            let final_count = count_chapter_length(&sanitized, spec.counting_mode);
            let warning = build_warning(final_count, spec);
            (sanitized, warning)
        };

        let final_count = count_chapter_length(&final_content, spec.counting_mode);
        let applied = final_content != content;

        Ok(NormalizeOutput {
            content: final_content,
            word_count: final_count,
            applied,
            mode,
            warning,
        })
    }
}

#[async_trait]
impl BaseAgent for LengthNormalizerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::LengthNormalizer
    }

    fn name(&self) -> &str {
        "length-normalizer"
    }
}

/// 归一化输出。
pub struct NormalizeOutput {
    /// 归一化后的正文（可能等于原文，若安全网触发或无需归一化）。
    pub content: String,
    /// 归一化后的字数（用 `count_chapter_length` 重新统计）。
    pub word_count: u32,
    /// 是否实际应用了归一化（final_content != 原文）。
    pub applied: bool,
    /// 归一化模式（expand / compress / none）。
    pub mode: LengthNormalizeMode,
    /// 警告信息（安全网触发或字数仍超区间时存在）。
    pub warning: Option<String>,
}

// ════════════════════════════════════════════════════════════════════
// Prompt 构造
// ════════════════════════════════════════════════════════════════════

fn build_system_prompt(mode: LengthNormalizeMode, language: &str) -> String {
    let action_zh = if mode == LengthNormalizeMode::Compress { "压缩" } else { "扩写" };
    let action_en = if mode == LengthNormalizeMode::Compress { "compress" } else { "expand" };

    if language == "en" {
        format!(
            "You are a chapter length normalizer. Your task is to perform a single correction on the chapter prose. You may only execute once; do not recursively rewrite.\n\nCorrection goal:\n- {action} the chapter length to the given target range\n- Preserve the chapter's original facts, key hooks, character names, and any required markers\n- Do not introduce new subplots, future reveals, or additional summaries\n- Do not output any explanation outside the prose",
            action = action_en,
        )
    } else {
        format!(
            "你是一位章节长度修正器。你的任务是对章节正文做一次单次修正，只能执行一次，不得递归重写。\n\n修正目标：\n- {action} 章节长度到给定目标区间\n- 保留章节原有事实、关键钩子、角色名和必须保留的标记\n- 不要引入新的支线、未来揭示或额外总结\n- 不要在正文外输出任何解释",
            action = action_zh,
        )
    }
}

fn build_user_prompt(
    content: &str,
    spec: &LengthSpec,
    original_count: u32,
    mode: LengthNormalizeMode,
    language: &str,
) -> String {
    let action_zh = if mode == LengthNormalizeMode::Compress { "压缩" } else { "扩写" };
    let action_en = if mode == LengthNormalizeMode::Compress { "compress" } else { "expand" };

    if language == "en" {
        format!(
            "Please {action} the following text once.\n\n## Length Spec\n- Target: {target}\n- Soft Range: {soft_min}-{soft_max}\n- Hard Range: {hard_min}-{hard_max}\n- Counting Mode: {counting_mode}\n\n## Current Count\n{original_count}\n\n## Correction Rules\n- Only correct once, do not recurse\n- Preserve key markers, character names, place names, and existing facts in the prose\n- Do not invent new subplots\n- Do not insert explanatory summaries or analysis\n- Output the complete corrected prose, without any tags\n\n## Chapter Content\n{content}",
            action = action_en,
            target = spec.target,
            soft_min = spec.soft_min,
            soft_max = spec.soft_max,
            hard_min = spec.hard_min,
            hard_max = spec.hard_max,
            counting_mode = format_counting_mode(spec.counting_mode),
            original_count = original_count,
            content = content,
        )
    } else {
        format!(
            "请对下面正文做一次{action}修正。\n\n## Length Spec\n- Target: {target}\n- Soft Range: {soft_min}-{soft_max}\n- Hard Range: {hard_min}-{hard_max}\n- Counting Mode: {counting_mode}\n\n## Current Count\n{original_count}\n\n## Correction Rules\n- 只修正一次，不要递归\n- 保留正文中的关键标记、人物名、地点名和已有事实\n- 不要凭空新增子情节\n- 不要插入解释性总结或分析\n- 输出修正后的完整正文，不要加标签\n\n## Chapter Content\n{content}",
            action = action_zh,
            target = spec.target,
            soft_min = spec.soft_min,
            soft_max = spec.soft_max,
            hard_min = spec.hard_min,
            hard_max = spec.hard_max,
            counting_mode = format_counting_mode(spec.counting_mode),
            original_count = original_count,
            content = content,
        )
    }
}

fn format_counting_mode(mode: super::length_metrics::LengthCountingMode) -> &'static str {
    use super::length_metrics::LengthCountingMode::*;
    match mode {
        ZhChars => "zh_chars",
        EnWords => "en_words",
    }
}

// ════════════════════════════════════════════════════════════════════
// 三重安全网
// ════════════════════════════════════════════════════════════════════

/// 生成归一化结果的警告信息（未触发安全网但字数仍超区间时）。
fn build_warning(final_count: u32, spec: &LengthSpec) -> Option<String> {
    if !is_outside_soft_range(final_count, spec) {
        return None;
    }
    if is_outside_hard_range(final_count, spec) {
        Some(format!(
            "Final count {} is outside the hard range {}-{} after one normalization pass.",
            final_count, spec.hard_min, spec.hard_max
        ))
    } else {
        Some(format!(
            "Final count {} is outside the soft range {}-{} after one normalization pass.",
            final_count, spec.soft_min, spec.soft_max
        ))
    }
}

/// 检测 LLM 输出是否过度修正，穿越到相反的硬边界。
///
/// - 原文超 hard_max，修正后却低于 hard_min → 穿越（过度压缩）
/// - 原文低于 hard_min，修正后却超 hard_max → 穿越（过度扩写）
fn crosses_opposite_hard_bound(original: u32, candidate: u32, spec: &LengthSpec) -> bool {
    (original > spec.hard_max && candidate < spec.hard_min)
        || (original < spec.hard_min && candidate > spec.hard_max)
}

/// 检测内容是否在句中截断。
///
/// 判定规则（与 inkos 一致）：
/// - 空内容 → 不算截断
/// - 以 ``` 结尾 → 不算截断（代码块正常闭合）
/// - 以句末标点结尾（。！？!?」』"'）)]】》…）→ 不算截断
/// - 以逗号/分号/冒号 + 换行结尾 → 截断
/// - 以逗号/分号/冒号/、 或中文/字母/数字结尾 → 截断
fn looks_truncated(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with("```") {
        return false;
    }

    // 句末标点 → 不截断
    let sentence_end = ['。', '！', '？', '!', '?', '」', '』', '"', '’', '）', ')', ']', '】', '》', '…'];
    if trimmed.ends_with(sentence_end) {
        return false;
    }

    // 逗号/分号/冒号 + 换行 → 截断
    let mid_punct = ['，', ',', '；', ';', '：', ':'];
    if content.ends_with('\n') && trimmed.ends_with(mid_punct) {
        return true;
    }

    // 以逗号/分号/冒号/、 结尾 → 截断
    if trimmed.ends_with(mid_punct) || trimmed.ends_with('、') {
        return true;
    }

    // 以中文/字母/数字结尾 → 截断
    trimmed.chars().last().map(|c| {
        c.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&c)
    }).unwrap_or(false)
}

/// 清洗 LLM 归一化输出，剥离包装文本。
///
/// 流程：
/// 1. trim
/// 2. 若空 → 回退原文
/// 3. 提取首个 ```fenced``` 代码块（若有）
/// 4. 剥离常见包装行（"下面是正文" / "I'll rewrite" 等）
/// 5. 若剥离后为空 → 回退原文
/// 6. 若剥离超过 50% → 包装行识别过激，回退到 trim 版本
fn sanitize_normalized_content(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }

    // 尝试提取首个 fenced 代码块
    if let Some(fenced) = extract_first_fenced_block(trimmed) {
        return fenced;
    }

    // 剥离常见包装行
    if let Some(stripped) = strip_common_wrappers(trimmed) {
        if stripped.is_empty() {
            return fallback.to_string();
        }
        // 防止正则过激：若剥离超过 50%，回退到 trim 版本
        if stripped.len() < trimmed.len() / 2 {
            return trimmed.to_string();
        }
        return stripped;
    }

    trimmed.to_string()
}

/// 提取首个 ```fenced``` 代码块内容。
fn extract_first_fenced_block(content: &str) -> Option<String> {
    let re = Regex::new(r"```(?:[a-zA-Z0-9-]+)?\s*\n([\s\S]*?)\n```").unwrap();
    re.captures(content).and_then(|cap| {
        cap.get(1).map(|m| {
            let body = m.as_str().trim();
            if body.is_empty() { String::new() } else { body.to_string() }
        })
    }).filter(|s| !s.is_empty())
}

/// 剥离常见包装行，返回剥离后的内容。
///
/// 返回 `None` 表示未识别到任何包装行（保持原文）。
fn strip_common_wrappers(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut removed_any = false;
    let mut kept: Vec<&str> = Vec::new();

    for raw_line in &lines {
        let trimmed = raw_line.trim();
        if is_wrapper_line(trimmed) {
            removed_any = true;
        } else {
            kept.push(raw_line);
        }
    }

    if !removed_any {
        return None;
    }

    let result = kept.join("\n").trim().to_string();
    Some(result)
}

/// 判断是否为包装行（应被剥离）。
fn is_wrapper_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    // 代码块围栏
    if line.starts_with("```") {
        return true;
    }
    // 中文：说明/解释/注释 标题
    if Regex::new(r"^#+\s*(说明|解释|注释|analysis|analysis note)").unwrap().is_match(line) {
        return true;
    }
    // 中文：下面是/以下是 ... 正文/章节/压缩/扩写/修正/...
    let re = Regex::new(r"^(下面是|以下是).*(正文|章节|压缩|扩写|修正|修改|调整|改写|润色|结果|内容|输出|版本)").unwrap();
    if re.is_match(line) {
        return true;
    }
    // 中文：我先 ... 压缩/扩写/修正 ... 正文/章节
    let re = Regex::new(r"^我先.*(压缩|扩写|修正|修改|调整|改写|润色|处理).*(正文|章节)?").unwrap();
    if re.is_match(line) {
        return true;
    }
    // 英文：here's/below is ... chapter/draft/content/rewrite/...
    let re = Regex::new(r"(?i)^(here(?:'s| is)|below is).*(chapter|draft|content|rewrite|revised|compressed|expanded|normalized|adjusted|output|version|result)").unwrap();
    if re.is_match(line) {
        return true;
    }
    // 英文：I'll/I will rewrite/revise/reword/compress/expand/...
    let re = Regex::new(r"(?i)^i(?:'ll| will)\s+(rewrite|revise|reword|compress|expand|normalize|adjust|shorten|lengthen|trim|fix)\b").unwrap();
    if re.is_match(line) {
        return true;
    }
    false
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::length_metrics::{LengthCountingMode, LengthSpec};

    fn spec(target: u32, language: &str) -> LengthSpec {
        LengthSpec::build(target, language)
    }

    // ── is_wrapper_line ────────────────────────────────────────

    #[test]
    fn is_wrapper_line_chinese() {
        assert!(is_wrapper_line("下面是修正后的正文："));
        assert!(is_wrapper_line("以下是压缩后的章节"));
        assert!(!is_wrapper_line("这是正文内容。"));
    }

    #[test]
    fn is_wrapper_line_english() {
        assert!(is_wrapper_line("Here's the rewritten chapter:"));
        assert!(is_wrapper_line("below is the compressed version"));
        assert!(!is_wrapper_line("The actual content."));
    }

    // ── build_system_prompt / build_user_prompt ─────────────────

    #[test]
    fn system_prompt_compress_zh() {
        let p = build_system_prompt(LengthNormalizeMode::Compress, "zh");
        assert!(p.contains("压缩"));
        assert!(p.contains("单次修正"));
    }

    #[test]
    fn system_prompt_expand_en() {
        let p = build_system_prompt(LengthNormalizeMode::Expand, "en");
        assert!(p.contains("expand"));
        assert!(p.contains("single correction"));
    }

    #[test]
    fn user_prompt_includes_length_spec() {
        let s = spec(3000, "zh");
        let p = build_user_prompt("正文", &s, 2000, LengthNormalizeMode::Expand, "zh");
        assert!(p.contains("Target: 3000"));
        assert!(p.contains("## Chapter Content"));
        assert!(p.contains("正文"));
    }

    // ── looks_truncated ─────────────────────────────────────────

    #[test]
    fn truncated_empty_is_not_truncated() {
        assert!(!looks_truncated(""));
        assert!(!looks_truncated("   "));
    }

    #[test]
    fn truncated_code_fence_close_is_not_truncated() {
        assert!(!looks_truncated("some text\n```"));
    }

    #[test]
    fn truncated_sentence_end_punctuation_is_not_truncated() {
        assert!(!looks_truncated("完整的一句话。"));
        assert!(!looks_truncated("Hello world!"));
        assert!(!looks_truncated("Is it true?"));
        assert!(!looks_truncated("「对话」"));
    }

    #[test]
    fn truncated_comma_newline_is_truncated() {
        assert!(looks_truncated("半截的话，\n"));
    }

    #[test]
    fn truncated_alphanumeric_end_is_truncated() {
        assert!(looks_truncated("半截的话"));
        assert!(looks_truncated("half a sentence"));
    }

    // ── crosses_opposite_hard_bound ─────────────────────────────

    #[test]
    fn crosses_from_too_long_to_too_short() {
        let s = spec(3000, "zh");
        // 原文超 hard_max，候选低于 hard_min
        assert!(crosses_opposite_hard_bound(s.hard_max + 100, s.hard_min - 100, &s));
    }

    #[test]
    fn crosses_from_too_short_to_too_long() {
        let s = spec(3000, "zh");
        assert!(crosses_opposite_hard_bound(s.hard_min - 100, s.hard_max + 100, &s));
    }

    #[test]
    fn crosses_not_crossed_when_stays_on_same_side() {
        let s = spec(3000, "zh");
        // 原文超 hard_max，候选也超 hard_max（但仍高于 hard_min）
        assert!(!crosses_opposite_hard_bound(s.hard_max + 100, s.hard_max + 50, &s));
    }

    #[test]
    fn crosses_not_crossed_when_within_range() {
        let s = spec(3000, "zh");
        assert!(!crosses_opposite_hard_bound(s.hard_max + 100, s.target, &s));
    }

    // ── sanitize_normalized_content ─────────────────────────────

    #[test]
    fn sanitize_empty_returns_fallback() {
        let result = sanitize_normalized_content("", "原文");
        assert_eq!(result, "原文");
    }

    #[test]
    fn sanitize_whitespace_only_returns_fallback() {
        let result = sanitize_normalized_content("   \n  ", "原文");
        assert_eq!(result, "原文");
    }

    #[test]
    fn sanitize_extracts_fenced_block() {
        let raw = "Below is the result:\n\n```\n这是正文。\n```\n\nHope this helps!";
        let result = sanitize_normalized_content(raw, "原文");
        assert_eq!(result, "这是正文。");
    }

    #[test]
    fn sanitize_strips_wrapper_lines_zh() {
        // 内容需足够长，确保剥离包装行后剩余内容 > 原始长度 50%（否则 50% 安全网会回退到原文）
        let raw = "下面是修正后的正文：\n\n这是一段足够长的正文内容，用于确保剥离包装行后剩余内容超过原始长度的百分之五十，这样安全网不会触发回退到原始文本。";
        let result = sanitize_normalized_content(raw, "原文");
        assert_eq!(result, "这是一段足够长的正文内容，用于确保剥离包装行后剩余内容超过原始长度的百分之五十，这样安全网不会触发回退到原始文本。");
    }

    #[test]
    fn sanitize_strips_wrapper_lines_en() {
        let raw = "Here's the rewritten chapter:\n\nThe actual content is long enough to ensure that stripping the wrapper line leaves more than fifty percent of the original length, so the safety guard does not trigger.";
        let result = sanitize_normalized_content(raw, "原文");
        assert_eq!(result, "The actual content is long enough to ensure that stripping the wrapper line leaves more than fifty percent of the original length, so the safety guard does not trigger.");
    }

    #[test]
    fn sanitize_no_wrapper_returns_trimmed() {
        let raw = "  纯正文内容。  ";
        let result = sanitize_normalized_content(raw, "原文");
        assert_eq!(result, "纯正文内容。");
    }

    #[test]
    fn sanitize_wrapper_only_returns_fallback() {
        // 只有包装行，剥离后为空 → 回退原文
        let raw = "下面是修正后的正文：";
        let result = sanitize_normalized_content(raw, "原文");
        assert_eq!(result, "原文");
    }

    // ── build_warning ───────────────────────────────────────────

    #[test]
    fn warning_none_when_within_soft_range() {
        let s = spec(3000, "zh");
        assert_eq!(build_warning(s.target, &s), None);
    }

    #[test]
    fn warning_soft_when_outside_soft_but_within_hard() {
        let s = spec(3000, "zh");
        let w = build_warning(s.soft_min - 10, &s).expect("should have warning");
        assert!(w.contains("soft range"));
    }

    #[test]
    fn warning_hard_when_outside_hard_range() {
        let s = spec(3000, "zh");
        let w = build_warning(s.hard_min - 100, &s).expect("should have warning");
        assert!(w.contains("hard range"));
    }

    // ── format_counting_mode ────────────────────────────────────

    #[test]
    fn format_counting_mode_strings() {
        assert_eq!(format_counting_mode(LengthCountingMode::ZhChars), "zh_chars");
        assert_eq!(format_counting_mode(LengthCountingMode::EnWords), "en_words");
    }
}
