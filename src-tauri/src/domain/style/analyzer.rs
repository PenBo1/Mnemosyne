//! ═══════════════════════════════════════════════════════════════════════════
//! 风格分析器 - 纯文本统计分析
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 中英文双轨:
//! - 中文:按字符计量句长/段落长度,TTR 按字符集(去除标点/空白/数字)
//! - 英文:按单词计量句长/段落长度,TTR 按单词集(小写化)
//!
//! 修辞模式识别(regex,出现 ≥2 次才记录):
//! - 中文 6 种:比喻/排比/反问/夸张/拟人/短句节奏
//! - 英文 4 种:simile/rhetorical question/tricolon/short punchy rhythm

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use super::types::{ParagraphRange, StyleProfile};

/// 中文修辞模式(名称, 正则)。
const ZH_RHETORICAL_PATTERNS: &[(&str, &str)] = &[
    ("比喻(像/如/仿佛)", r"[像如仿佛似](?:是|同|一般|一样)"),
    ("排比", r"[，。；]([^，。；]{2,6})[，。；]\1"),
    ("反问", r"难道|怎么可能|岂不是|何尝不"),
    ("夸张", r"天崩地裂|惊天动地|翻天覆地|震耳欲聋"),
    ("拟人", r"[风雨雪月花树草石](?:在|像|仿佛).*?(?:笑|哭|叹|呻|吟|怒|舞)"),
    ("短句节奏", r"[。！？][^。！？]{1,8}[。！？]"),
];

/// 英文修辞模式(名称, 正则)。
const EN_RHETORICAL_PATTERNS: &[(&str, &str)] = &[
    ("simile (like/as if)", r"\b(?:like a|like an|as if|as though)\b"),
    ("rhetorical question", r"\b(?:how could|why would|what if|wasn't it|isn't it|could it be)\b[^.!?]*\?"),
    ("tricolon", r"\b\w+,\s+\w+,\s+and\s+\w+\b"),
    ("short punchy rhythm", r"[.!?]\s+[A-Z][^.!?]{1,24}[.!?]"),
];

// ── 预编译正则(OnceLock 缓存,避免每次调用重新编译) ──────────────

fn sentence_split_en_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[.!?\n]+").unwrap())
}

fn sentence_split_zh_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[。！？\n]").unwrap())
}

fn paragraph_split_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\n\s*\n").unwrap())
}

fn en_word_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z0-9]+(?:'[A-Za-z0-9]+)?").unwrap())
}

fn en_ttr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)[a-z0-9]+(?:'[a-z0-9]+)?").unwrap())
}

fn en_top_pattern_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z']+").unwrap())
}

/// 预编译中文修辞正则(过滤无效 pattern)。
fn zh_rhetorical_compiled() -> &'static [(&'static str, Regex)] {
    static COMPILED: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        ZH_RHETORICAL_PATTERNS
            .iter()
            .filter_map(|(name, pat)| {
                match Regex::new(pat) {
                    Ok(r) => Some((*name, r)),
                    Err(e) => {
                        tracing::warn!(pattern = pat, name = *name, error = %e, "Invalid rhetorical regex skipped");
                        None
                    }
                }
            })
            .collect()
    })
}

/// 预编译英文修辞正则(过滤无效 pattern)。
fn en_rhetorical_compiled() -> &'static [(&'static str, Regex)] {
    static COMPILED: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        EN_RHETORICAL_PATTERNS
            .iter()
            .filter_map(|(name, pat)| {
                match Regex::new(pat) {
                    Ok(r) => Some((*name, r)),
                    Err(e) => {
                        tracing::warn!(pattern = pat, name = *name, error = %e, "Invalid rhetorical regex skipped");
                        None
                    }
                }
            })
            .collect()
    })
}

/// 语言选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Zh,
    En,
}

impl Language {
    /// 解析语言字符串(非法值回退为 Zh)。
    pub fn parse(raw: &str) -> Self {
        match raw.to_lowercase().as_str() {
            "en" | "english" => Self::En,
            _ => Self::Zh,
        }
    }
}

/// 分析参考文本,提取风格指纹画像。
pub fn analyze_style(text: &str, source_name: Option<&str>, language: Language) -> StyleProfile {
    let start = std::time::Instant::now();
    tracing::debug!(
        text_len = text.len(),
        language = ?language,
        source = ?source_name,
        "[StyleAnalyzer] Starting style analysis"
    );
    
    let is_en = language == Language::En;

    // 1. 句子分割
    let sentences = split_sentences(text, is_en);
    let paragraphs = split_paragraphs(text);

    // 2. 句长统计(中文=字符数,英文=单词数)
    let sentence_lengths: Vec<usize> = sentences.iter().map(|s| measure(s, is_en)).collect();
    let avg_sentence_length = mean(&sentence_lengths);
    let sentence_length_std_dev = std_dev(&sentence_lengths, avg_sentence_length);

    // 3. 段落长度统计
    let paragraph_lengths: Vec<usize> = paragraphs.iter().map(|p| measure(p, is_en)).collect();
    let avg_paragraph_length = mean(&paragraph_lengths);
    let (min_paragraph, max_paragraph) = min_max(&paragraph_lengths);

    // 4. 词汇多样性(TTR)
    let vocabulary_diversity = compute_ttr(text, is_en);

    // 5. 高频句首模式
    let top_patterns = compute_top_patterns(&sentences, is_en);

    // 6. 修辞特征
    let rhetorical_features = detect_rhetorical_features(text, is_en);

    tracing::debug!(
        avg_sentence_length,
        vocabulary_diversity,
        patterns = top_patterns.len(),
        features = rhetorical_features.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "[StyleAnalyzer] Style analysis completed"
    );

    StyleProfile {
        avg_sentence_length: round1(avg_sentence_length),
        sentence_length_std_dev: round1(sentence_length_std_dev),
        avg_paragraph_length: round1(avg_paragraph_length),
        paragraph_length_range: ParagraphRange {
            min: min_paragraph,
            max: max_paragraph,
        },
        vocabulary_diversity: round3(vocabulary_diversity),
        top_patterns,
        rhetorical_features,
        source_name: source_name.map(|s| s.to_string()),
        analyzed_at: Some(chrono::Utc::now().to_rfc3339()),
    }
}

/// 句子分割:中文按 。！？\n,英文按 .!?\n。
fn split_sentences(text: &str, is_en: bool) -> Vec<String> {
    let re = if is_en {
        sentence_split_en_re()
    } else {
        sentence_split_zh_re()
    };
    re.split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 段落分割:按空行。
fn split_paragraphs(text: &str) -> Vec<String> {
    paragraph_split_re()
        .split(text)
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// 计量长度:英文=单词数,中文=去空白后的字符数。
fn measure(s: &str, is_en: bool) -> usize {
    if is_en {
        en_word_re().find_iter(s).count()
    } else {
        s.chars().filter(|c| !c.is_whitespace()).count()
    }
}

/// 计算平均值。
fn mean(values: &[usize]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let sum: usize = values.iter().sum();
    sum as f64 / values.len() as f64
}

/// 计算标准差(总体标准差)。
fn std_dev(values: &[usize], mean_val: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance: f64 = values
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean_val;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

/// 求最小值与最大值。
fn min_max(values: &[usize]) -> (usize, usize) {
    if values.is_empty() {
        return (0, 0);
    }
    let mut min_v = values[0];
    let mut max_v = values[0];
    for &v in values.iter().skip(1) {
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    (min_v, max_v)
}

/// 计算 TTR(Type-Token Ratio):
/// - 英文:单词级(小写化)
/// - 中文:字符级(去除标点/空白/数字)
fn compute_ttr(text: &str, is_en: bool) -> f64 {
    if is_en {
        let words: Vec<&str> = en_ttr_re().find_iter(text).map(|m| m.as_str()).collect();
        if words.is_empty() {
            return 0.0;
        }
        let mut unique: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for w in &words {
            unique.insert(w);
        }
        unique.len() as f64 / words.len() as f64
    } else {
        // 去除标点/空白/数字,保留汉字
        let chars: Vec<char> = text
            .chars()
            .filter(|c| {
                !c.is_whitespace()
                    && !c.is_ascii_digit()
                    && !is_cjk_punctuation(*c)
            })
            .collect();
        if chars.is_empty() {
            return 0.0;
        }
        let mut unique: std::collections::HashSet<char> = std::collections::HashSet::new();
        for c in &chars {
            unique.insert(*c);
        }
        unique.len() as f64 / chars.len() as f64
    }
}

/// 判断字符是否为中文标点(含全角标点)。
fn is_cjk_punctuation(c: char) -> bool {
    matches!(
        c,
        '，' | '。' | '！' | '？' | '、' | '：' | '；'
        | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}'
        | '（' | '）' | '【' | '】' | '《' | '》' | '…' | '—' | '·'
    ) || c.is_ascii_punctuation()
}

/// 计算高频句首模式(前 5,出现 ≥3 次才记录)。
/// - 英文:首单词(小写化)
/// - 中文:前 2 字符
fn compute_top_patterns(sentences: &[String], is_en: bool) -> Vec<String> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    let word_re = en_top_pattern_re();
    for s in sentences {
        let key = if is_en {
            word_re
                .find(s)
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_default()
        } else {
            // 取前 2 字符
            s.chars().take(2).collect::<String>()
        };
        if key.is_empty() {
            continue;
        }
        *counts.entry(key).or_insert(0) += 1;
    }
    let mut entries: Vec<(String, u32)> = counts.into_iter().collect();
    // 按计数降序排序,取前 5
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries
        .into_iter()
        .take(5)
        .filter(|(_, count)| *count >= 3)
        .map(|(pattern, count)| {
            if is_en {
                format!("{}… ({})", pattern, count)
            } else {
                format!("{}...({}次)", pattern, count)
            }
        })
        .collect()
}

/// 检测修辞特征(出现 ≥2 次才记录)。
fn detect_rhetorical_features(text: &str, is_en: bool) -> Vec<String> {
    let compiled = if is_en {
        en_rhetorical_compiled()
    } else {
        zh_rhetorical_compiled()
    };
    let mut features = Vec::new();
    for (name, re) in compiled {
        let count = re.find_iter(text).count();
        if count >= 2 {
            if is_en {
                features.push(format!("{} ({})", name, count));
            } else {
                features.push(format!("{}({}处)", name, count));
            }
        }
    }
    features
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_parse_defaults_to_zh() {
        assert_eq!(Language::parse("zh"), Language::Zh);
        assert_eq!(Language::parse("en"), Language::En);
        assert_eq!(Language::parse("english"), Language::En);
        assert_eq!(Language::parse("bogus"), Language::Zh);
        assert_eq!(Language::parse(""), Language::Zh);
    }

    #[test]
    fn analyze_style_zh_basic() {
        let text = "他像风一样走过。她仿佛在笑。天崩地裂。天崩地裂。难道这不是奇迹吗？";
        let profile = analyze_style(text, Some("test"), Language::Zh);
        assert!(profile.avg_sentence_length > 0.0);
        assert!(profile.vocabulary_diversity > 0.0);
        assert!(profile.vocabulary_diversity <= 1.0);
        assert_eq!(profile.source_name.as_deref(), Some("test"));
        assert!(profile.analyzed_at.is_some());
        // "天崩地裂" 出现 2 次,应被记为夸张
        assert!(profile.rhetorical_features.iter().any(|f| f.contains("夸张")));
    }

    #[test]
    fn analyze_style_en_basic() {
        let text = "He ran like a ghost. She smiled as if nothing happened. It was like a dream. Why would anyone do that?";
        let profile = analyze_style(text, None, Language::En);
        assert!(profile.avg_sentence_length > 0.0);
        assert!(profile.vocabulary_diversity > 0.0);
        // simile 模式出现 3 次
        assert!(profile.rhetorical_features.iter().any(|f| f.contains("simile")));
    }

    #[test]
    fn analyze_style_empty_text() {
        let profile = analyze_style("", None, Language::Zh);
        assert_eq!(profile.avg_sentence_length, 0.0);
        assert_eq!(profile.sentence_length_std_dev, 0.0);
        assert_eq!(profile.avg_paragraph_length, 0.0);
        assert_eq!(profile.paragraph_length_range.min, 0);
        assert_eq!(profile.paragraph_length_range.max, 0);
        assert_eq!(profile.vocabulary_diversity, 0.0);
        assert!(profile.top_patterns.is_empty());
        assert!(profile.rhetorical_features.is_empty());
    }

    #[test]
    fn measure_zh_counts_chars() {
        assert_eq!(measure("你好世界", false), 4);
        assert_eq!(measure("你好 世界", false), 4); // 空格被过滤
        assert_eq!(measure("", false), 0);
    }

    #[test]
    fn measure_en_counts_words() {
        assert_eq!(measure("hello world", true), 2);
        assert_eq!(measure("don't go", true), 2);
        assert_eq!(measure("", true), 0);
    }

    #[test]
    fn split_paragraphs_handles_blank_lines() {
        let text = "段落一\n\n段落二\n\n\n段落三";
        let paras = split_paragraphs(text);
        assert_eq!(paras.len(), 3);
        assert_eq!(paras[0], "段落一");
        assert_eq!(paras[2], "段落三");
    }

    #[test]
    fn split_sentences_zh() {
        let text = "第一句。第二句！第三句？";
        let sentences = split_sentences(text, false);
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn split_sentences_en() {
        let text = "First sentence. Second one! Third?";
        let sentences = split_sentences(text, true);
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn compute_ttr_zh_returns_ratio() {
        // 全部不同字符 → TTR=1.0
        let ttr = compute_ttr("你好世界", false);
        assert_eq!(ttr, 1.0);
        // 重复字符 → TTR < 1
        let ttr = compute_ttr("你好你好", false);
        assert!(ttr < 1.0);
    }

    #[test]
    fn compute_ttr_en_returns_ratio() {
        let ttr = compute_ttr("hello world", true);
        assert_eq!(ttr, 1.0);
        let ttr = compute_ttr("hello hello", true);
        assert!(ttr < 1.0);
    }

    #[test]
    fn compute_top_patterns_filters_below_threshold() {
        let sentences = vec![
            "他来了".to_string(),
            "他走了".to_string(),
            "她笑了".to_string(),
        ];
        // 各句首 2 字符均不同(他来/他走/她笑),无高频模式
        let patterns = compute_top_patterns(&sentences, false);
        assert!(patterns.is_empty());
    }

    #[test]
    fn compute_top_patterns_keeps_high_freq() {
        let sentences = vec![
            "他在笑".to_string(),
            "他在哭".to_string(),
            "他在走".to_string(),
        ];
        // "他在" 出现 3 次(首 2 字符相同),应被记录
        let patterns = compute_top_patterns(&sentences, false);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].contains("他在"));
        assert!(patterns[0].contains("3次"));
    }

    #[test]
    fn detect_rhetorical_features_requires_two_matches() {
        // 仅出现 1 次,不应记录
        let features = detect_rhetorical_features("难道只有一次", false);
        assert!(features.is_empty());
        // 出现 2 次,应记录
        let features = detect_rhetorical_features("难道一次。难道两次。", false);
        assert!(features.iter().any(|f| f.contains("反问")));
    }

    #[test]
    fn min_max_handles_empty() {
        let (mn, mx) = min_max(&[]);
        assert_eq!(mn, 0);
        assert_eq!(mx, 0);
    }

    #[test]
    fn min_max_finds_extremes() {
        let (mn, mx) = min_max(&[5, 2, 8, 3]);
        assert_eq!(mn, 2);
        assert_eq!(mx, 8);
    }

    #[test]
    fn std_dev_single_value_is_zero() {
        assert_eq!(std_dev(&[5], 5.0), 0.0);
        assert_eq!(std_dev(&[], 0.0), 0.0);
    }

    #[test]
    fn round_functions_truncate_correctly() {
        assert_eq!(round1(3.14159), 3.1);
        assert_eq!(round3(0.123456), 0.123);
    }

    #[test]
    fn is_cjk_punctuation_recognizes_full_and_half_width() {
        assert!(is_cjk_punctuation('，'));
        assert!(is_cjk_punctuation('。'));
        assert!(is_cjk_punctuation(','));
        assert!(is_cjk_punctuation('.'));
        assert!(!is_cjk_punctuation('你'));
        assert!(!is_cjk_punctuation('a'));
    }
}
