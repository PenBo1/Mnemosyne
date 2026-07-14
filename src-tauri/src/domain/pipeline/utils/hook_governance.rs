// Hook 准入治理。
//
// 核心逻辑：
// 1. evaluate_hook_admission: 对 newHookCandidate 做准入决策
//    - missing_type: type 字段为空 → 拒绝
//    - missing_payoff_signal: expectedPayoff + notes 全空 → 拒绝
//    - duplicate_family: 与已有 hook 在 type+terms 重叠 → 拒绝
// 2. 文本规范化：小写 + 去除非字母数字中文
// 3. 重叠判定：英文 terms(>=4字符) 重叠 >= 2 或 中文 bigrams 重叠 >= 3

use crate::domain::pipeline::state::types::HookRecord;

/// 准入决策原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionReason {
    Admit,
    MissingType,
    MissingPayoffSignal,
    DuplicateFamily,
}

/// 准入决策结果
#[derive(Debug, Clone)]
pub struct HookAdmissionDecision {
    pub admit: bool,
    pub reason: AdmissionReason,
    /// 若为 DuplicateFamily，匹配到的已有 hook 的 hookId
    pub matched_hook_id: Option<String>,
}

/// 准入候选输入（与 NewHookCandidate 字段对齐 + startChapter）
pub struct HookAdmissionCandidate<'a> {
    pub r#type: &'a str,
    pub expected_payoff: &'a str,
    pub payoff_timing: Option<&'a str>,
    pub notes: &'a str,
}

/// 评估 hook 准入。
///
/// 输入：
/// - candidate: 待准入的 hook 候选
/// - active_hooks: 当前已存在的 hook 列表
///
/// 输出：HookAdmissionDecision
pub fn evaluate_hook_admission(
    candidate: &HookAdmissionCandidate<'_>,
    active_hooks: &[HookRecord],
) -> HookAdmissionDecision {
    // 1. missing_type 检查
    if normalize_text(candidate.r#type).is_empty() {
        return HookAdmissionDecision {
            admit: false,
            reason: AdmissionReason::MissingType,
            matched_hook_id: None,
        };
    }

    // 2. missing_payoff_signal 检查
    let payoff_signal = format!("{} {}", candidate.expected_payoff, candidate.notes);
    if normalize_text(&payoff_signal).is_empty() {
        return HookAdmissionDecision {
            admit: false,
            reason: AdmissionReason::MissingPayoffSignal,
            matched_hook_id: None,
        };
    }

    // 3. duplicate_family 检查
    let candidate_text = normalize_text(&format!(
        "{} {} {} {}",
        candidate.r#type,
        candidate.expected_payoff,
        candidate.payoff_timing.unwrap_or(""),
        candidate.notes,
    ));

    for hook in active_hooks {
        let hook_text = normalize_text(&format!(
            "{} {} {:?} {}",
            hook.r#type, hook.expected_payoff, hook.payoff_timing, hook.notes,
        ));

        // 完全相等 → 立即判重
        if hook_text == candidate_text {
            return HookAdmissionDecision {
                admit: false,
                reason: AdmissionReason::DuplicateFamily,
                matched_hook_id: Some(hook.hook_id.clone()),
            };
        }

        // type 不同 → 跳过
        let hook_type_norm = normalize_text(&hook.r#type);
        let candidate_type_norm = normalize_text(candidate.r#type);
        if hook_type_norm != candidate_type_norm {
            continue;
        }

        // type 相同 → 检查 terms 重叠
        if has_term_overlap(&candidate_text, &hook_text) {
            return HookAdmissionDecision {
                admit: false,
                reason: AdmissionReason::DuplicateFamily,
                matched_hook_id: Some(hook.hook_id.clone()),
            };
        }
    }

    HookAdmissionDecision {
        admit: true,
        reason: AdmissionReason::Admit,
        matched_hook_id: None,
    }
}

/// 判定两个文本的 terms 重叠是否达到阈值。
///
/// - 英文 terms：>= 4 字符，过滤 STOP_WORDS，重叠 >= 2 → true
/// - 中文 bigrams：2 字符，重叠 >= 3 → true
fn has_term_overlap(text_a: &str, text_b: &str) -> bool {
    let terms_a = extract_english_terms(text_a);
    let terms_b = extract_english_terms(text_b);

    let overlap_en: usize = terms_a.iter().filter(|t| terms_b.contains(t)).count();
    if overlap_en >= 2 {
        return true;
    }

    let bigrams_a = extract_chinese_bigrams(text_a);
    let bigrams_b = extract_chinese_bigrams(text_b);

    let overlap_zh: usize = bigrams_a.iter().filter(|b| bigrams_b.contains(b)).count();
    overlap_zh >= 3
}

/// 提取英文 terms（>= 4 字符，过滤停用词）。
fn extract_english_terms(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "that", "this", "with", "from", "have", "will",
        "been", "they", "their", "there", "what", "which", "when", "where",
        "who", "whom", "whose", "into", "about", "after", "before", "between",
        "through", "during", "above", "below", "over", "under", "again",
        "then", "once", "here", "such", "more", "most", "some", "any", "each",
    ];

    let mut terms = Vec::new();
    let re = match regex::Regex::new(r"[A-Za-z]{4,}") {
        Ok(re) => re,
        Err(_) => return terms,
    };
    for m in re.find_iter(text) {
        let word = m.as_str().to_lowercase();
        if STOP_WORDS.contains(&word.as_str()) {
            continue;
        }
        if !terms.contains(&word) {
            terms.push(word);
        }
    }
    terms
}

/// 提取中文 bigrams（2 字符）。
fn extract_chinese_bigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text
        .chars()
        .filter(|c| is_chinese_char(*c))
        .collect();
    if chars.len() < 2 {
        return Vec::new();
    }
    let mut bigrams = Vec::with_capacity(chars.len() - 1);
    for i in 0..chars.len() - 1 {
        let bg = format!("{}{}", chars[i], chars[i + 1]);
        if !bigrams.contains(&bg) {
            bigrams.push(bg);
        }
    }
    bigrams
}

fn is_chinese_char(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
        || ('\u{3400}'..='\u{4DBF}').contains(&c)
        || ('\u{F900}'..='\u{FAFF}').contains(&c)
}

/// 文本规范化：小写 + 保留字母数字中文，其余转为空白。
pub fn normalize_text(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || is_chinese_char(c) {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pipeline::state::types::HookStatus;

    fn make_active_hook(
        id: &str,
        type_: &str,
        expected_payoff: &str,
        notes: &str,
    ) -> HookRecord {
        HookRecord {
            hook_id: id.to_string(),
            start_chapter: 1,
            r#type: type_.to_string(),
            status: HookStatus::Open,
            last_advanced_chapter: 1,
            expected_payoff: expected_payoff.to_string(),
            payoff_timing: None,
            notes: notes.to_string(),
            depends_on: None,
            pays_off_in_arc: None,
            core_hook: None,
            half_life_chapters: None,
            advanced_count: None,
            promoted: None,
        }
    }

    #[test]
    fn admits_clean_candidate() {
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "reveal the killer's identity",
            payoff_timing: None,
            notes: "planted in chapter 3",
        };
        let active = vec![make_active_hook("h1", "relationship", "trust built", "ch1")];
        let decision = evaluate_hook_admission(&candidate, &active);
        assert!(decision.admit);
        assert_eq!(decision.reason, AdmissionReason::Admit);
    }

    #[test]
    fn rejects_missing_type() {
        let candidate = HookAdmissionCandidate {
            r#type: "  ",
            expected_payoff: "payoff",
            payoff_timing: None,
            notes: "notes",
        };
        let decision = evaluate_hook_admission(&candidate, &[]);
        assert!(!decision.admit);
        assert_eq!(decision.reason, AdmissionReason::MissingType);
    }

    #[test]
    fn rejects_missing_payoff_signal() {
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "  ",
            payoff_timing: None,
            notes: "  ",
        };
        let decision = evaluate_hook_admission(&candidate, &[]);
        assert!(!decision.admit);
        assert_eq!(decision.reason, AdmissionReason::MissingPayoffSignal);
    }

    #[test]
    fn rejects_exact_duplicate() {
        let active = vec![make_active_hook(
            "h1",
            "mystery",
            "reveal the killer",
            "planted in ch3",
        )];
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "reveal the killer",
            payoff_timing: None,
            notes: "planted in ch3",
        };
        let decision = evaluate_hook_admission(&candidate, &active);
        assert!(!decision.admit);
        assert_eq!(decision.reason, AdmissionReason::DuplicateFamily);
        assert_eq!(decision.matched_hook_id.as_deref(), Some("h1"));
    }

    #[test]
    fn rejects_term_overlap_duplicate() {
        // 同 type + 多个英文 term 重叠
        let active = vec![make_active_hook(
            "h1",
            "mystery",
            "reveal killer identity chapter",
            "planted detective",
        )];
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "reveal killer identity mystery",
            payoff_timing: None,
            notes: "planted detective chapter",
        };
        let decision = evaluate_hook_admission(&candidate, &active);
        assert!(!decision.admit, "expected duplicate_family, got {:?}", decision.reason);
        assert_eq!(decision.reason, AdmissionReason::DuplicateFamily);
    }

    #[test]
    fn rejects_chinese_bigram_overlap() {
        let active = vec![make_active_hook(
            "h1",
            "mystery",
            "揭示宝藏的下落",
            "主角发现藏宝图",
        )];
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "揭示宝藏的位置",
            payoff_timing: None,
            notes: "主角发现地图",
        };
        let decision = evaluate_hook_admission(&candidate, &active);
        // 中文 bigram 重叠：揭示/宝藏/主角/发现 等 >= 3 → duplicate
        assert!(!decision.admit, "expected duplicate_family, got {:?}", decision.reason);
        assert_eq!(decision.reason, AdmissionReason::DuplicateFamily);
    }

    #[test]
    fn different_type_not_duplicate() {
        let active = vec![make_active_hook(
            "h1",
            "relationship",
            "reveal the killer",
            "planted in ch3",
        )];
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "reveal the killer",
            payoff_timing: None,
            notes: "planted in ch3",
        };
        let decision = evaluate_hook_admission(&candidate, &active);
        assert!(decision.admit);
    }

    #[test]
    fn ignores_payoff_timing_variant() {
        // payoff_timing 不参与文本规范化比对（如 "near-term" vs "slow-burn"）
        let candidate = HookAdmissionCandidate {
            r#type: "mystery",
            expected_payoff: "reveal the truth",
            payoff_timing: Some("near-term"),
            notes: "planted",
        };
        let active = vec![make_active_hook("h1", "mystery", "reveal the truth", "planted")];
        let decision = evaluate_hook_admission(&candidate, &active);
        // 完全相等（不含 timing） → duplicate
        assert!(!decision.admit);
    }
}
