//! S6.3: Spot-fix PATCHES 解析与应用。
//!
//! 移植自 inkos `utils/spot-fix-patches.ts`。reviser 在 auto/spot-fix 模式下
//! 输出 PATCHES 块（TARGET_TEXT + REPLACEMENT_TEXT 对），由本模块解析并应用到
//! 原文。匹配策略：精确匹配优先，失败时回退到空白归一化的模糊匹配。
//! 单个 patch 匹配失败时跳过（不影响其他 patch），整体应用率 >= 50% 才算成功。

/// 一个定点修复补丁。
#[derive(Debug, Clone, PartialEq)]
pub struct SpotFixPatch {
    pub target_text: String,
    pub replacement_text: String,
}

/// PATCHES 应用结果。
#[derive(Debug, Clone)]
pub struct SpotFixPatchApplyResult {
    pub applied: bool,
    pub revised_content: String,
    pub applied_patch_count: usize,
    pub skipped_patch_count: usize,
    pub touched_chars: usize,
}

/// 解析 PATCHES 块文本为补丁列表。
///
/// 期望格式（由 reviser prompt 约束）：
/// ```text
/// --- PATCH 1 ---
/// TARGET_TEXT:
/// (原文引用)
/// REPLACEMENT_TEXT:
/// (替换文本)
/// --- END PATCH ---
/// ```
/// `PATCH` 后的序号可选。target_text 为空的补丁会被过滤。
pub fn parse_spot_fix_patches(raw: &str) -> Vec<SpotFixPatch> {
    // 若包含 === PATCHES === 标记，从标记之后开始解析
    let normalized = if let Some(idx) = raw.find("=== PATCHES ===") {
        &raw[idx + "=== PATCHES ===".len()..]
    } else {
        raw
    };

    let mut patches = Vec::new();
    let mut remaining = normalized;

    loop {
        // 找到下一个 "--- PATCH" 起点
        let start = match find_patch_start(remaining) {
            Some(s) => s,
            None => break,
        };
        let after_start = &remaining[start..];

        // 找到对应的 "--- END PATCH ---"
        let end_marker = "--- END PATCH ---";
        let end_rel = match after_start.find(end_marker) {
            Some(e) => e,
            None => break,
        };

        let patch_body = &after_start[..end_rel];
        if let Some(patch) = parse_single_patch(patch_body) {
            if !patch.target_text.is_empty() {
                patches.push(patch);
            }
        }

        // 继续解析剩余部分
        remaining = &after_start[end_rel + end_marker.len()..];
    }

    patches
}

/// 查找 "--- PATCH" 或 "--- PATCH N ---" 的起点。
fn find_patch_start(text: &str) -> Option<usize> {
    let marker = "--- PATCH";
    let idx = text.find(marker)?;

    // 跳过 "--- PATCH" 后可能跟的序号和 "---"
    let after = &text[idx + marker.len()..];
    let mut chars = after.chars().peekable();
    // 跳过空格和数字
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == ' ' {
            chars.next();
        } else {
            break;
        }
    }
    // 期望接下来是 "---"
    let rest: String = chars.collect();
    if rest.starts_with("---") {
        // 返回 patch body 的起点（"---" 之后）
        let consumed = after.len() - rest.len() + 3; // 3 = "---".len()
        Some(idx + marker.len() + consumed)
    } else {
        None
    }
}

/// 解析单个 patch body（TARGET_TEXT: ... REPLACEMENT_TEXT: ...）。
fn parse_single_patch(body: &str) -> Option<SpotFixPatch> {
    let target_marker = "TARGET_TEXT:";
    let replacement_marker = "REPLACEMENT_TEXT:";

    let target_start = body.find(target_marker)?;
    let replacement_start = body.find(replacement_marker)?;

    // TARGET_TEXT 必须在 REPLACEMENT_TEXT 之前
    if target_start >= replacement_start {
        return None;
    }

    let target_raw = &body[target_start + target_marker.len()..replacement_start];
    let replacement_raw = &body[replacement_start + replacement_marker.len()..];

    Some(SpotFixPatch {
        target_text: trim_field(target_raw),
        replacement_text: trim_field(replacement_raw),
    })
}

/// 应用补丁到原文。逐个尝试匹配，单个失败跳过。
pub fn apply_spot_fix_patches(original: &str, patches: &[SpotFixPatch]) -> SpotFixPatchApplyResult {
    if patches.is_empty() {
        return SpotFixPatchApplyResult {
            applied: false,
            revised_content: original.to_string(),
            applied_patch_count: 0,
            skipped_patch_count: 0,
            touched_chars: 0,
        };
    }

    let mut current = original.to_string();
    let mut applied_patch_count = 0usize;
    let mut skipped_patch_count = 0usize;
    let mut touched_chars = 0usize;

    for patch in patches {
        if let Some(new_content) = try_apply_patch(&current, patch) {
            touched_chars += patch.target_text.chars().count();
            current = new_content;
            applied_patch_count += 1;
        } else {
            skipped_patch_count += 1;
        }
    }

    let applied = applied_patch_count > 0 && current != original;
    SpotFixPatchApplyResult {
        applied,
        revised_content: current,
        applied_patch_count,
        skipped_patch_count,
        touched_chars,
    }
}

/// 尝试应用单个补丁：精确匹配优先，失败回退到模糊匹配。
fn try_apply_patch(content: &str, patch: &SpotFixPatch) -> Option<String> {
    // 1. 精确匹配（要求唯一命中）
    if let Some(start) = try_exact_match(content, &patch.target_text) {
        let mut result = String::with_capacity(content.len() + patch.replacement_text.len());
        result.push_str(&content[..start]);
        result.push_str(&patch.replacement_text);
        result.push_str(&content[start + patch.target_text.len()..]);
        return Some(result);
    }

    // 2. 模糊匹配（空白归一化）
    if let Some((start, end)) = try_fuzzy_match(content, &patch.target_text) {
        let mut result = String::with_capacity(content.len() + patch.replacement_text.len());
        result.push_str(&content[..start]);
        result.push_str(&patch.replacement_text);
        result.push_str(&content[end..]);
        return Some(result);
    }

    None
}

/// 精确匹配：要求 target 在 content 中唯一出现。
fn try_exact_match(content: &str, target: &str) -> Option<usize> {
    let start = content.find(target)?;
    // 检查唯一性：从 start + target.len() 之后还能找到吗？
    let rest_start = start + target.len();
    if rest_start < content.len() {
        if content[rest_start..].contains(target) {
            return None; // 多次出现，不唯一
        }
    }
    Some(start)
}

/// 模糊匹配：归一化空白后匹配，返回原文的字节位置范围。
///
/// 归一化规则：所有空白字符序列（含换行）压缩为单个空格，首尾空白去除。
/// target 长度 < 10 字符时拒绝模糊匹配（太短不可靠）。
fn try_fuzzy_match(content: &str, target: &str) -> Option<(usize, usize)> {
    let normalized_target = normalize_whitespace(target);
    let target_chars: Vec<char> = normalized_target.chars().collect();
    if target_chars.len() < 10 {
        return None;
    }

    // 构建原文的归一化字符序列 + 每个归一化字符对应的原文字节起点
    let (norm_chars, norm_char_to_orig_byte) = build_normalized_char_mapping(content);
    if norm_chars.len() < target_chars.len() {
        return None;
    }

    // 在归一化字符序列中查找 target（要求唯一）
    let start_idx = find_subsequence_chars(&norm_chars, &target_chars, 0)?;
    let end_idx = start_idx + target_chars.len();

    // 唯一性：从 end_idx 之后还能找到吗？
    if find_subsequence_chars(&norm_chars, &target_chars, end_idx).is_some() {
        return None;
    }

    // 映射回原文字节位置
    let orig_start = *norm_char_to_orig_byte.get(start_idx)?;
    // 最后一个匹配字符的原文字节起点 + 其字节长度
    let last_char_orig_start = *norm_char_to_orig_byte.get(end_idx - 1)?;
    let last_char = content[last_char_orig_start..].chars().next()?;
    let orig_end = last_char_orig_start + last_char.len_utf8();

    // 边界检查
    if orig_start > orig_end || orig_end > content.len() {
        return None;
    }

    // 验证：原文 [orig_start, orig_end] 归一化后应等于 normalized_target
    let orig_slice = &content[orig_start..orig_end];
    if normalize_whitespace(orig_slice) != normalized_target {
        return None;
    }

    Some((orig_start, orig_end))
}

/// 把文本中的空白序列压缩为单个空格，去除首尾空白。
fn normalize_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_was_space && !result.is_empty() {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(ch);
            prev_was_space = false;
        }
    }
    // 去除尾部空格
    if result.ends_with(' ') {
        result.pop();
    }
    result
}

/// 构建归一化字符序列与原文字节起点的映射。
///
/// 返回 (norm_chars, norm_char_to_orig_byte)：
/// - `norm_chars[i]`：归一化后第 i 个字符
/// - `norm_char_to_orig_byte[i]`：该字符在原文中的字节起点
///
/// 注意：按字符索引（非字节），避免 UTF-8 多字节字符错位。
fn build_normalized_char_mapping(content: &str) -> (Vec<char>, Vec<usize>) {
    let mut norm_chars: Vec<char> = Vec::with_capacity(content.chars().count());
    let mut norm_to_orig: Vec<usize> = Vec::with_capacity(content.chars().count());

    let mut prev_was_space = false;
    let mut leading = true;

    for (byte_pos, ch) in content.char_indices() {
        if ch.is_whitespace() {
            if !leading && !prev_was_space {
                norm_chars.push(' ');
                norm_to_orig.push(byte_pos);
                prev_was_space = true;
            }
        } else {
            norm_chars.push(ch);
            norm_to_orig.push(byte_pos);
            prev_was_space = false;
            leading = false;
        }
    }

    // 去除尾部空格
    if norm_chars.last() == Some(&' ') {
        norm_chars.pop();
        norm_to_orig.pop();
    }

    (norm_chars, norm_to_orig)
}

/// 在字符序列中从 `from` 开始查找子序列，返回字符索引。
fn find_subsequence_chars(haystack: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > haystack.len() {
        return None;
    }
    'outer: for i in from..=haystack.len() - needle.len() {
        for j in 0..needle.len() {
            if haystack[i + j] != needle[j] {
                continue 'outer;
            }
        }
        return Some(i);
    }
    None
}

/// 去除字段值首尾的空白和换行。
fn trim_field(value: &str) -> String {
    value.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_spot_fix_patches ────────────────────────────────

    #[test]
    fn test_parse_single_patch() {
        let raw = r#"--- PATCH 1 ---
TARGET_TEXT:
他走了过去。
REPLACEMENT_TEXT:
他踱到窗前。
--- END PATCH ---"#;
        let patches = parse_spot_fix_patches(raw);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].target_text, "他走了过去。");
        assert_eq!(patches[0].replacement_text, "他踱到窗前。");
    }

    #[test]
    fn test_parse_multiple_patches() {
        let raw = r#"--- PATCH 1 ---
TARGET_TEXT:
原文一
REPLACEMENT_TEXT:
替换一
--- END PATCH ---
--- PATCH 2 ---
TARGET_TEXT:
原文二
REPLACEMENT_TEXT:
替换二
--- END PATCH ---"#;
        let patches = parse_spot_fix_patches(raw);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[1].target_text, "原文二");
    }

    #[test]
    fn test_parse_patch_without_number() {
        let raw = "--- PATCH ---\nTARGET_TEXT:\nx\nREPLACEMENT_TEXT:\ny\n--- END PATCH ---";
        let patches = parse_spot_fix_patches(raw);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].target_text, "x");
    }

    #[test]
    fn test_parse_patches_with_outer_marker() {
        let raw = "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\nx\nREPLACEMENT_TEXT:\ny\n--- END PATCH ---";
        let patches = parse_spot_fix_patches(raw);
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_parse_empty_target_filtered() {
        let raw = "--- PATCH 1 ---\nTARGET_TEXT:\n\nREPLACEMENT_TEXT:\ny\n--- END PATCH ---";
        let patches = parse_spot_fix_patches(raw);
        assert!(patches.is_empty(), "empty target should be filtered");
    }

    #[test]
    fn test_parse_no_patches_returns_empty() {
        let raw = "no patches here";
        let patches = parse_spot_fix_patches(raw);
        assert!(patches.is_empty());
    }

    #[test]
    fn test_parse_truncated_patch_returns_empty() {
        // 缺少 END PATCH 标记
        let raw = "--- PATCH 1 ---\nTARGET_TEXT:\nx\nREPLACEMENT_TEXT:\ny\n";
        let patches = parse_spot_fix_patches(raw);
        assert!(patches.is_empty());
    }

    // ── apply_spot_fix_patches ────────────────────────────────

    #[test]
    fn test_apply_exact_match() {
        let original = "他走了过去。然后拿了杯子。";
        let patches = vec![SpotFixPatch {
            target_text: "他走了过去。".to_string(),
            replacement_text: "他踱到窗前。".to_string(),
        }];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(result.applied);
        assert_eq!(result.revised_content, "他踱到窗前。然后拿了杯子。");
        assert_eq!(result.applied_patch_count, 1);
        assert_eq!(result.skipped_patch_count, 0);
    }

    #[test]
    fn test_apply_multiple_patches_sequential() {
        let original = "原文一。原文二。";
        let patches = vec![
            SpotFixPatch { target_text: "原文一".to_string(), replacement_text: "替换一".to_string() },
            SpotFixPatch { target_text: "原文二".to_string(), replacement_text: "替换二".to_string() },
        ];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(result.applied);
        assert_eq!(result.revised_content, "替换一。替换二。");
        assert_eq!(result.applied_patch_count, 2);
    }

    #[test]
    fn test_apply_non_unique_target_skipped() {
        let original = "重复。重复。";
        let patches = vec![SpotFixPatch {
            target_text: "重复".to_string(),
            replacement_text: "唯一".to_string(),
        }];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(!result.applied);
        assert_eq!(result.skipped_patch_count, 1);
        assert_eq!(result.revised_content, original);
    }

    #[test]
    fn test_apply_not_found_skipped() {
        let original = "原文";
        let patches = vec![SpotFixPatch {
            target_text: "不存在的文本".to_string(),
            replacement_text: "替换".to_string(),
        }];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(!result.applied);
        assert_eq!(result.skipped_patch_count, 1);
    }

    #[test]
    fn test_apply_empty_patches() {
        let original = "原文";
        let result = apply_spot_fix_patches(original, &[]);
        assert!(!result.applied);
        assert_eq!(result.revised_content, original);
    }

    #[test]
    fn test_apply_partial_success() {
        // 第一个 patch 命中，第二个 miss
        let original = "原文一。其他。";
        let patches = vec![
            SpotFixPatch { target_text: "原文一".to_string(), replacement_text: "替换一".to_string() },
            SpotFixPatch { target_text: "不存在".to_string(), replacement_text: "替换二".to_string() },
        ];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(result.applied);
        assert_eq!(result.revised_content, "替换一。其他。");
        assert_eq!(result.applied_patch_count, 1);
        assert_eq!(result.skipped_patch_count, 1);
    }

    // ── fuzzy match ────────────────────────────────────────────

    #[test]
    fn test_apply_fuzzy_match_whitespace_diff() {
        // target 与原文空白数量不同（多空格 vs 单空格），归一化后应匹配
        let original = "他慢慢地走了过去，  然后拿起了杯子。";
        let patches = vec![SpotFixPatch {
            target_text: "他慢慢地走了过去， 然后拿起了杯子。".to_string(),
            replacement_text: "他踱到窗前。".to_string(),
        }];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(result.applied, "fuzzy match should succeed");
        assert_eq!(result.revised_content, "他踱到窗前。");
    }

    #[test]
    fn test_apply_fuzzy_match_too_short_rejected() {
        // target 太短（< 10 字符），不应触发模糊匹配
        let original = "短文本";
        let patches = vec![SpotFixPatch {
            target_text: "短 文本".to_string(), // 归一化后 = "短 文本"，3 字符
            replacement_text: "x".to_string(),
        }];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(!result.applied, "short target should not fuzzy match");
    }

    #[test]
    fn test_apply_fuzzy_match_with_newlines() {
        let original = "第一段内容。\n\n第二段内容。\n\n第三段内容。";
        let target = "第一段内容。 第二段内容。"; // 归一化后
        let patches = vec![SpotFixPatch {
            target_text: target.to_string(),
            replacement_text: "替换。".to_string(),
        }];
        let result = apply_spot_fix_patches(original, &patches);
        assert!(result.applied, "fuzzy match across newlines should succeed");
        assert_eq!(result.revised_content, "替换。\n\n第三段内容。");
    }

    // ── normalize_whitespace ──────────────────────────────────

    #[test]
    fn test_normalize_whitespace_basic() {
        assert_eq!(normalize_whitespace("a  b   c"), "a b c");
        assert_eq!(normalize_whitespace("a\n\nb\nc"), "a b c");
        assert_eq!(normalize_whitespace("  a b  "), "a b");
        assert_eq!(normalize_whitespace("a\t\tb"), "a b");
    }

    #[test]
    fn test_normalize_whitespace_chinese() {
        assert_eq!(normalize_whitespace("他  走了  过去"), "他 走了 过去");
    }

    #[test]
    fn test_normalize_whitespace_empty() {
        assert_eq!(normalize_whitespace(""), "");
        assert_eq!(normalize_whitespace("   "), "");
    }

    // ── end-to-end: parse → apply ─────────────────────────────

    #[test]
    fn test_end_to_end_parse_and_apply() {
        let original = "他走了过去。然后拿了杯子。他坐下来。";
        let raw = format!(
            "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n他走了过去。\nREPLACEMENT_TEXT:\n他踱到窗前。\n--- END PATCH ---\n--- PATCH 2 ---\nTARGET_TEXT:\n他坐下来。\nREPLACEMENT_TEXT:\n他瘫在椅子里。\n--- END PATCH ---"
        );
        let patches = parse_spot_fix_patches(&raw);
        assert_eq!(patches.len(), 2);
        let result = apply_spot_fix_patches(original, &patches);
        assert!(result.applied);
        assert_eq!(result.revised_content, "他踱到窗前。然后拿了杯子。他瘫在椅子里。");
        assert_eq!(result.applied_patch_count, 2);
    }
}
