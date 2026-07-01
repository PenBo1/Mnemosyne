// S7.5: 风格指纹（style fingerprint）—— 纯统计分析，无 LLM 依赖。
//
// 移植自 inkos `style-analyzer.ts` + `models/style-profile.ts`。与 inkos 的差异：
// - 用 Rust regex 替代 JS RegExp（Rust regex crate 不支持 backreference，
//   所以中文"排比"模式用手动扫描实现，其余模式直接移植）
// - 持久化函数 save/load 用 serde_json（与 inkos 一致写到 story/style_profile.json）
// - 确定性指南 build_deterministic_style_guide 直接移植自 inkos runner.ts
//
// 设计：StyleProfile 是纯数据，可序列化；analyze_style 是纯函数，无 I/O。
// LLM 定性拆解（style_guide.md 的 8 维度分析）不在本模块，留给 writer agent 扩展。

use crate::shared::errors::AppError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 风格指纹 profile（统计特征）。
///
/// 对应 inkos `StyleProfile`。所有字段从参考文本统计得出，无 LLM 参与。
/// 序列化为 `style_profile.json` 供后续比对/校验。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    pub avg_sentence_length: f64,
    pub sentence_length_std_dev: f64,
    pub avg_paragraph_length: u32,
    pub paragraph_length_range: ParagraphLengthRange,
    /// TTR (Type-Token Ratio)：中文按字符，英文按单词
    pub vocabulary_diversity: f64,
    /// 高频句首/模式（前 5 个，出现 >= 3 次）
    pub top_patterns: Vec<String>,
    /// 修辞特征（比喻/排比/反问/夸张/拟人/短句节奏）
    pub rhetorical_features: Vec<String>,
    pub source_name: Option<String>,
    pub analyzed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphLengthRange {
    pub min: u32,
    pub max: u32,
}

/// 修辞模式定义（编译好的正则 + 双语显示名）。
struct RhetoricalPattern {
    regex: Regex,
    /// 中文显示名（如 "比喻(像/如/仿佛)"），英文显示名（如 "simile (like/as if)"）
    label_zh: String,
    label_en: String,
}

/// 中文修辞模式（移植自 inkos RHETORICAL_PATTERNS，除"排比"外）。
///
/// "排比"原正则 `[，。；]([^，。；]{2,6})[，。；]\1` 需要 backreference，
/// Rust regex crate 不支持，改用 `detect_chinese_parallelism` 手动扫描。
fn chinese_rhetorical_patterns() -> Vec<RhetoricalPattern> {
    vec![
        RhetoricalPattern {
            label_zh: "比喻(像/如/仿佛)".to_string(),
            label_en: "simile (像/如/仿佛)".to_string(),
            regex: Regex::new(r"[像如仿佛似](?:是|同|一般|一样)").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "反问".to_string(),
            label_en: "rhetorical question".to_string(),
            regex: Regex::new(r"难道|怎么可能|岂不是|何尝不").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "夸张".to_string(),
            label_en: "hyperbole".to_string(),
            regex: Regex::new(r"天崩地裂|惊天动地|翻天覆地|震耳欲聋").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "拟人".to_string(),
            label_en: "personification".to_string(),
            regex: Regex::new(r"[风雨雪月花树草石](?:在|像|仿佛).*?(?:笑|哭|叹|呻|吟|怒|舞)").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "短句节奏".to_string(),
            label_en: "short punchy rhythm".to_string(),
            regex: Regex::new(r"[。！？][^。！？]{1,8}[。！？]").unwrap(),
        },
    ]
}

/// 英文修辞模式（移植自 inkos EN_RHETORICAL_PATTERNS，除"tricolon"外）。
///
/// "tricolon"原正则用 `\w+,\s+\w+,\s+and\s+\w+`，Rust regex 可以支持，
/// 但 `\b` 边界在英文外文本上行为不一致。保留所有 4 个英文模式。
fn english_rhetorical_patterns() -> Vec<RhetoricalPattern> {
    vec![
        RhetoricalPattern {
            label_zh: "明喻".to_string(),
            label_en: "simile (like/as if)".to_string(),
            regex: Regex::new(r"(?i)\b(?:like a|like an|as if|as though)\b").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "反问".to_string(),
            label_en: "rhetorical question".to_string(),
            regex: Regex::new(r"(?i)\b(?:how could|why would|what if|wasn't it|isn't it|could it be)\b[^.!?]*\?").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "三段式".to_string(),
            label_en: "tricolon".to_string(),
            regex: Regex::new(r"(?i)\b\w+,\s+\w+,\s+and\s+\w+\b").unwrap(),
        },
        RhetoricalPattern {
            label_zh: "短句节奏".to_string(),
            label_en: "short punchy rhythm".to_string(),
            regex: Regex::new(r"[.!?]\s+[A-Z][^.!?]{1,24}[.!?]").unwrap(),
        },
    ]
}

/// 分析参考文本，提取风格指纹。
///
/// 纯函数，无 I/O。对应 inkos `analyzeStyle`。
///
/// - 中文：按字符计长（去除空白），按字符去重算 TTR，句按 `[。！？\n]` 切分
/// - 英文：按单词计长，按单词去重算 TTR，句按 `[.!?\n]` 切分
/// - 段落按双换行 `\n\s*\n` 切分
/// - 修辞模式：中文 5 个 + 排比手动扫描；英文 4 个
pub fn analyze_style(text: &str, source_name: Option<&str>, language: &str) -> StyleProfile {
    let is_en = language == "en";

    // 切句
    let sentence_splitter: &[char] = if is_en {
        &['.', '!', '?', '\n']
    } else {
        &['。', '！', '？', '\n']
    };
    let sentences: Vec<&str> = text
        .split(|c: char| sentence_splitter.contains(&c))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // 切段
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    // 长度度量：英文按单词，中文按字符（去空白）
    let measure = |s: &str| -> u32 {
        if is_en {
            count_english_words(s)
        } else {
            s.chars().filter(|c| !c.is_whitespace()).count() as u32
        }
    };

    // 句长统计
    let sentence_lengths: Vec<u32> = sentences.iter().map(|s| measure(s)).collect();
    let avg_sentence_length = if sentence_lengths.is_empty() {
        0.0
    } else {
        let sum: u32 = sentence_lengths.iter().sum();
        sum as f64 / sentence_lengths.len() as f64
    };
    let sentence_length_std_dev = if sentence_lengths.len() > 1 {
        let variance = sentence_lengths
            .iter()
            .map(|l| (*l as f64 - avg_sentence_length).powi(2))
            .sum::<f64>()
            / sentence_lengths.len() as f64;
        variance.sqrt()
    } else {
        0.0
    };

    // 段长统计
    let paragraph_lengths: Vec<u32> = paragraphs.iter().map(|p| measure(p)).collect();
    let avg_paragraph_length = if paragraph_lengths.is_empty() {
        0
    } else {
        let sum: u32 = paragraph_lengths.iter().sum();
        (sum as f64 / paragraph_lengths.len() as f64).round() as u32
    };
    let min_paragraph = paragraph_lengths.iter().copied().min().unwrap_or(0);
    let max_paragraph = paragraph_lengths.iter().copied().max().unwrap_or(0);

    // 词汇多样性（TTR）
    let vocabulary_diversity = if is_en {
        compute_english_ttr(text)
    } else {
        compute_chinese_ttr(text)
    };

    // 高频句首模式
    let top_patterns = extract_top_patterns(&sentences, is_en);

    // 修辞特征
    let rhetorical_features = extract_rhetorical_features(text, is_en);

    StyleProfile {
        avg_sentence_length: round1(avg_sentence_length),
        sentence_length_std_dev: round1(sentence_length_std_dev),
        avg_paragraph_length,
        paragraph_length_range: ParagraphLengthRange {
            min: min_paragraph,
            max: max_paragraph,
        },
        vocabulary_diversity: round3(vocabulary_diversity),
        top_patterns,
        rhetorical_features,
        source_name: source_name.map(|s| s.to_string()),
        analyzed_at: current_iso8601(),
    }
}

/// 统计英文单词数（与 inkos 的 measure 一致）。
fn count_english_words(s: &str) -> u32 {
    let re = Regex::new(r"[A-Za-z0-9]+(?:'[A-Za-z0-9]+)?").unwrap();
    re.find_iter(s).count() as u32
}

/// 英文 TTR：按单词去重 / 单词总数。
fn compute_english_ttr(text: &str) -> f64 {
    let re = Regex::new(r"(?i)[a-z0-9]+(?:'[a-z0-9]+)?").unwrap();
    let words: Vec<&str> = re.find_iter(text).map(|m| m.as_str()).collect();
    if words.is_empty() {
        return 0.0;
    }
    let unique: std::collections::HashSet<&str> = words.iter().copied().collect();
    unique.len() as f64 / words.len() as f64
}

/// 中文 TTR：按字符去重 / 字符总数（去除标点和空白）。
fn compute_chinese_ttr(text: &str) -> f64 {
    let re = Regex::new(r#"[\s\n\r，。！？、：；""''（）【】《》\d]"#).unwrap();
    let chars: String = re.replace_all(text, "").to_string();
    if chars.is_empty() {
        return 0.0;
    }
    let char_vec: Vec<char> = chars.chars().collect();
    let unique: std::collections::HashSet<char> = char_vec.iter().copied().collect();
    unique.len() as f64 / char_vec.len() as f64
}

/// 提取高频句首模式。
///
/// - 中文：前 2 个字符作为 key
/// - 英文：首个单词（小写）作为 key
///
/// 返回前 5 个出现 >= 3 次的模式，格式：
/// - 中文：`"前两字...(N次)"`
/// - 英文：`"word… (N)"`
fn extract_top_patterns(sentences: &[&str], is_en: bool) -> Vec<String> {
    let mut counts: HashMap<String, u32> = HashMap::new();

    for s in sentences {
        let key = if is_en {
            let re = Regex::new(r"[A-Za-z']+").unwrap();
            re.find(s)
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_default()
        } else {
            let chars: Vec<char> = s.chars().take(2).collect();
            if chars.len() >= 2 {
                chars.into_iter().collect()
            } else {
                String::new()
            }
        };
        if !key.is_empty() {
            *counts.entry(key).or_insert(0) += 1;
        }
    }

    let mut entries: Vec<(String, u32)> = counts.into_iter().collect();
    // 按出现次数降序排序
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    entries
        .into_iter()
        .filter(|(_, count)| *count >= 3)
        .take(5)
        .map(|(pattern, count)| {
            if is_en {
                format!("{}… ({})", pattern, count)
            } else {
                format!("{}...({}次)", pattern, count)
            }
        })
        .collect()
}

/// 提取修辞特征。
///
/// 对每个修辞模式正则，统计匹配次数 >= 2 的模式，格式：
/// - 中文：`"比喻(3处)"`
/// - 英文：`"simile (3)"`
fn extract_rhetorical_features(text: &str, is_en: bool) -> Vec<String> {
    let patterns = if is_en {
        english_rhetorical_patterns()
    } else {
        chinese_rhetorical_patterns()
    };

    let mut features = Vec::new();
    for p in &patterns {
        let count = p.regex.find_iter(text).count();
        if count >= 2 {
            let label = if is_en { &p.label_en } else { &p.label_zh };
            if is_en {
                features.push(format!("{} ({})", label, count));
            } else {
                features.push(format!("{}({}处)", label, count));
            }
        }
    }

    // 中文额外检测排比（手动扫描，因为 Rust regex 不支持 backreference）
    if !is_en {
        let parallelism_count = detect_chinese_parallelism(text);
        if parallelism_count >= 2 {
            features.push(format!("排比({}处)", parallelism_count));
        }
    }

    features
}

/// 手动扫描中文排比模式。
///
/// inkos 原正则：`[，。；]([^，。；]{2,6})[，。；]\1`
/// 含义：标点后跟 2-6 字片段，再跟标点，再跟相同片段。
///
/// 手动实现：遍历所有标点位置，取后续 2-6 字作为候选，
/// 跳过下一个标点后检查是否重复。
fn detect_chinese_parallelism(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let puncts = ['，', '。', '；'];
    let mut count = 0;

    let mut i = 0;
    while i < chars.len() {
        if puncts.contains(&chars[i]) {
            // 尝试取后续 2-6 字作为候选片段
            for frag_len in 2..=6 {
                let frag_start = i + 1;
                let frag_end = frag_start + frag_len;
                if frag_end >= chars.len() {
                    break;
                }
                let fragment: String = chars[frag_start..frag_end].iter().collect();

                // 跳过 fragment 后的标点
                let next_punct_pos = frag_end;
                if next_punct_pos >= chars.len() || !puncts.contains(&chars[next_punct_pos]) {
                    continue;
                }

                // 检查标点后是否重复 fragment
                let repeat_start = next_punct_pos + 1;
                let repeat_end = repeat_start + frag_len;
                if repeat_end > chars.len() {
                    continue;
                }
                let repeat: String = chars[repeat_start..repeat_end].iter().collect();
                if repeat == fragment {
                    count += 1;
                    // 跳过这个匹配，避免重复计数
                    i = repeat_end;
                    break;
                }
            }
        }
        i += 1;
    }
    count
}

/// 生成确定性文风指南（不调用 LLM）。
///
/// 移植自 inkos `runner.buildDeterministicStyleGuide`。当参考文本过短
/// 或 LLM 定性拆解失败时作为兜底。
pub fn build_deterministic_style_guide(
    profile: &StyleProfile,
    language: &str,
    reason: &str,
) -> String {
    if language == "en" {
        let mut lines = vec![
            "# Style Guide".to_string(),
            String::new(),
            format!("> {}", reason),
            String::new(),
            "## Statistical Fingerprint".to_string(),
            format!("- Source: {}", profile.source_name.as_deref().unwrap_or("unknown")),
            format!("- Average sentence length: {}", profile.avg_sentence_length),
            format!("- Sentence length variance: {}", profile.sentence_length_std_dev),
            format!("- Average paragraph length: {}", profile.avg_paragraph_length),
            format!("- Vocabulary diversity: {}%", (profile.vocabulary_diversity * 100.0).round() as u32),
        ];

        if profile.top_patterns.is_empty() {
            lines.push("- Repeated openings: none obvious in this sample".to_string());
        } else {
            lines.push(format!("- Repeated openings: {}", profile.top_patterns.join(", ")));
        }

        if profile.rhetorical_features.is_empty() {
            lines.push("- Rhetorical features: none obvious in this sample".to_string());
        } else {
            lines.push(format!("- Rhetorical features: {}", profile.rhetorical_features.join(", ")));
        }

        lines.push(String::new());
        lines.push("## How To Use".to_string());
        lines.push("- Treat this as a lightweight style fingerprint, not a full imitation bible.".to_string());
        lines.push("- Keep sentence and paragraph rhythm close to the sample when drafting.".to_string());
        lines.push("- If this guide feels too thin, import a longer excerpt later; the file will be replaced.".to_string());

        lines.join("\n")
    } else {
        let mut lines = vec![
            "# 文风指南".to_string(),
            String::new(),
            format!("> {}", reason),
            String::new(),
            "## 统计风格指纹".to_string(),
            format!("- 来源：{}", profile.source_name.as_deref().unwrap_or("unknown")),
            format!("- 平均句长：{}", profile.avg_sentence_length),
            format!("- 句长波动：{}", profile.sentence_length_std_dev),
            format!("- 平均段落长度：{}", profile.avg_paragraph_length),
            format!("- 词汇多样性：{}%", (profile.vocabulary_diversity * 100.0).round() as u32),
        ];

        if profile.top_patterns.is_empty() {
            lines.push("- 高频句首/模式：样本内不明显".to_string());
        } else {
            lines.push(format!("- 高频句首/模式：{}", profile.top_patterns.join("、")));
        }

        if profile.rhetorical_features.is_empty() {
            lines.push("- 修辞特征：样本内不明显".to_string());
        } else {
            lines.push(format!("- 修辞特征：{}", profile.rhetorical_features.join("、")));
        }

        lines.push(String::new());
        lines.push("## 使用方式".to_string());
        lines.push("- 这是一份轻量文风指纹，不是完整仿写圣经。".to_string());
        lines.push("- 后续写作优先参考句长、段落长度、节奏波动和可见修辞。".to_string());
        lines.push("- 如果想得到更稳定的定性拆解，后续可以导入更长片段覆盖本文件。".to_string());

        lines.join("\n")
    }
}

/// 保存风格指纹到 `{book_dir}/story/style_profile.json`。
pub fn save_style_profile(
    book_dir: &std::path::Path,
    profile: &StyleProfile,
) -> Result<(), AppError> {
    let story_dir = book_dir.join("story");
    std::fs::create_dir_all(&story_dir)
        .map_err(|e| AppError::internal(format!("Failed to create story dir: {}", e)))?;
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| AppError::internal(format!("Failed to serialize style profile: {}", e)))?;
    let path = story_dir.join("style_profile.json");
    std::fs::write(&path, json)
        .map_err(|e| AppError::internal(format!("Failed to write style_profile.json: {}", e)))
}

/// 从 `{book_dir}/story/style_profile.json` 加载风格指纹。
///
/// 文件不存在时返回 None（调用方可据此决定是否需要先分析参考文本）。
pub fn load_style_profile(book_dir: &std::path::Path) -> Option<StyleProfile> {
    let path = book_dir.join("story").join("style_profile.json");
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ── 辅助函数 ──────────────────────────────────────────────────

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn current_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 简化 ISO8601：只用 Unix 秒数（避免引入 chrono 依赖来做格式化）
    format!("epoch:{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── analyze_style 中文 ─────────────────────────────────────

    #[test]
    fn test_analyze_style_zh_basic_stats() {
        let text = "他慢慢地走了过去。然后拿起了杯子。杯子很凉。\n\n他叹了口气。";
        let profile = analyze_style(text, Some("test"), "zh");

        // 4 个句子：句长按字符去空白计长
        // "他慢慢地走了过去" = 8 字符（他/慢/慢/地/走/了/过/去）
        // "然后拿起了杯子" = 7 字符（然/后/拿/起/了/杯/子）
        // "杯子很凉" = 4 字符
        // "他叹了口气" = 5 字符
        assert_eq!(profile.avg_sentence_length, round1((8.0 + 7.0 + 4.0 + 5.0) / 4.0));
        assert!(profile.sentence_length_std_dev > 0.0);
        // 2 个段落
        assert!(profile.avg_paragraph_length > 0);
        assert_eq!(profile.source_name.as_deref(), Some("test"));
    }

    #[test]
    fn test_analyze_style_zh_vocabulary_diversity() {
        // 重复字符多 → TTR 低
        let text = "他他他他他走了走了走了";
        let profile = analyze_style(text, None, "zh");
        assert!(profile.vocabulary_diversity < 0.5, "重复字符多，TTR 应该低");

        // 字符多样 → TTR 高
        let text2 = "风云雷电雨雪霜花树草石山水天地人";
        let profile2 = analyze_style(text2, None, "zh");
        assert!(profile2.vocabulary_diversity > 0.9, "字符全不重复，TTR 应该接近 1");
    }

    #[test]
    fn test_analyze_style_zh_rhetorical_features() {
        // 包含比喻（2 次：像是 / 如同）+ 反问 + 夸张 + 短句节奏
        // 修辞阈值要求 >= 2 次匹配才记录
        let text = "他像是一个孩子一样。她如同一阵风。难道这是真的？天崩地裂般的响声。好。坏。";
        let profile = analyze_style(text, None, "zh");
        assert!(!profile.rhetorical_features.is_empty(), "应检测到至少一个修辞特征");
        // 比喻应该被检测到（"像是" + "如同" 匹配 [像如仿佛似](?:是|同|一般|一样)）
        assert!(
            profile.rhetorical_features.iter().any(|f| f.contains("比喻")),
            "应检测到比喻，实际：{:?}",
            profile.rhetorical_features
        );
    }

    #[test]
    fn test_analyze_style_zh_short_text_no_crash() {
        let text = "短。";
        let profile = analyze_style(text, None, "zh");
        assert!(profile.avg_sentence_length >= 0.0);
        assert!(profile.rhetorical_features.is_empty());
    }

    #[test]
    fn test_analyze_style_empty_text() {
        let profile = analyze_style("", None, "zh");
        assert_eq!(profile.avg_sentence_length, 0.0);
        assert_eq!(profile.avg_paragraph_length, 0);
        assert_eq!(profile.paragraph_length_range.min, 0);
        assert_eq!(profile.paragraph_length_range.max, 0);
        assert_eq!(profile.vocabulary_diversity, 0.0);
        assert!(profile.top_patterns.is_empty());
        assert!(profile.rhetorical_features.is_empty());
    }

    #[test]
    fn test_analyze_style_zh_top_patterns() {
        // 中文以句首前 2 字符做 key，需相同 2 字前缀出现 >= 3 次
        // "他说" 作为句首出现 3 次
        let text = "他说走了。他说来了。他说笑了。她哭了。";
        let profile = analyze_style(text, None, "zh");
        assert!(!profile.top_patterns.is_empty(), "应检测到高频句首");
        assert!(
            profile.top_patterns.iter().any(|p| p.contains("他说")),
            "高频句首应包含'他说'，实际：{:?}",
            profile.top_patterns
        );
    }

    #[test]
    fn test_analyze_style_zh_parallelism_detection() {
        // "他笑了，他笑了，他笑了" 形式排比
        let text = "他笑了，他笑了，他笑了。然后离开了。";
        let count = detect_chinese_parallelism(text);
        assert!(count >= 1, "应检测到至少 1 处排比，实际：{}", count);
    }

    // ── analyze_style 英文 ─────────────────────────────────────

    #[test]
    fn test_analyze_style_en_basic_stats() {
        let text = "He walked slowly. Then picked up the cup. It was cold.\n\nHe sighed.";
        let profile = analyze_style(text, Some("en-test"), "en");

        assert_eq!(profile.source_name.as_deref(), Some("en-test"));
        // 4 个句子，单词数：3, 6, 3, 2
        assert!(profile.avg_sentence_length > 0.0);
        assert!(profile.sentence_length_std_dev > 0.0);
    }

    #[test]
    fn test_analyze_style_en_vocabulary_diversity() {
        // 重复单词多 → TTR 低
        let text = "the the the the the cat cat cat";
        let profile = analyze_style(text, None, "en");
        assert!(profile.vocabulary_diversity < 0.5);

        // 单词多样 → TTR 高
        let text2 = "apple banana cherry date elderberry fig grape";
        let profile2 = analyze_style(text2, None, "en");
        assert!(profile2.vocabulary_diversity > 0.9);
    }

    #[test]
    fn test_analyze_style_en_rhetorical_features() {
        let text = "He was like a ghost. It was as if time stopped. Why would he do that? She laughed, she cried, and she left.";
        let profile = analyze_style(text, None, "en");
        assert!(!profile.rhetorical_features.is_empty(), "应检测到修辞特征");
    }

    #[test]
    fn test_analyze_style_en_top_patterns() {
        // "The" 重复 >= 3 次
        let text = "The cat sat. The dog ran. The bird flew. The fish swam. A horse slept.";
        let profile = analyze_style(text, None, "en");
        assert!(!profile.top_patterns.is_empty());
        assert!(profile.top_patterns.iter().any(|p| p.to_lowercase().contains("the")));
    }

    // ── build_deterministic_style_guide ───────────────────────

    #[test]
    fn test_deterministic_guide_zh_includes_all_stats() {
        let text = "他走了。他来了。他笑了。像是一个孩子。天崩地裂。";
        let profile = analyze_style(text, Some("样本"), "zh");
        let guide = build_deterministic_style_guide(&profile, "zh", "样本过短");

        assert!(guide.contains("# 文风指南"));
        assert!(guide.contains("> 样本过短"));
        assert!(guide.contains("## 统计风格指纹"));
        assert!(guide.contains("来源：样本"));
        assert!(guide.contains("平均句长"));
        assert!(guide.contains("句长波动"));
        assert!(guide.contains("平均段落长度"));
        assert!(guide.contains("词汇多样性"));
        assert!(guide.contains("## 使用方式"));
    }

    #[test]
    fn test_deterministic_guide_en_includes_all_stats() {
        let text = "He walked. He came. He smiled. Like a ghost. Earth-shattering.";
        let profile = analyze_style(text, Some("sample"), "en");
        let guide = build_deterministic_style_guide(&profile, "en", "sample too short");

        assert!(guide.contains("# Style Guide"));
        assert!(guide.contains("> sample too short"));
        assert!(guide.contains("## Statistical Fingerprint"));
        assert!(guide.contains("Source: sample"));
        assert!(guide.contains("Average sentence length"));
        assert!(guide.contains("Sentence length variance"));
        assert!(guide.contains("Vocabulary diversity"));
        assert!(guide.contains("## How To Use"));
    }

    #[test]
    fn test_deterministic_guide_zh_empty_features() {
        let profile = StyleProfile {
            avg_sentence_length: 5.0,
            sentence_length_std_dev: 1.0,
            avg_paragraph_length: 10,
            paragraph_length_range: ParagraphLengthRange { min: 5, max: 15 },
            vocabulary_diversity: 0.5,
            top_patterns: vec![],
            rhetorical_features: vec![],
            source_name: None,
            analyzed_at: "epoch:0".to_string(),
        };
        let guide = build_deterministic_style_guide(&profile, "zh", "测试");
        assert!(guide.contains("样本内不明显"));
    }

    // ── save/load 持久化 ───────────────────────────────────────

    #[test]
    fn test_save_and_load_style_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let text = "他走了。他来了。他笑了。像是一个孩子。";
        let profile = analyze_style(text, Some("test"), "zh");

        save_style_profile(tmp.path(), &profile).unwrap();

        let loaded = load_style_profile(tmp.path());
        assert!(loaded.is_some(), "style_profile.json should exist");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.avg_sentence_length, profile.avg_sentence_length);
        assert_eq!(loaded.vocabulary_diversity, profile.vocabulary_diversity);
        assert_eq!(loaded.source_name, profile.source_name);
        assert_eq!(loaded.top_patterns, profile.top_patterns);
    }

    #[test]
    fn test_load_style_profile_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_style_profile(tmp.path()).is_none());
    }

    #[test]
    fn test_save_style_profile_creates_story_dir() {
        let tmp = tempfile::tempdir().unwrap();
        // story 目录不存在，save 应自动创建
        let profile = analyze_style("测试。", None, "zh");
        save_style_profile(tmp.path(), &profile).unwrap();
        assert!(tmp.path().join("story").join("style_profile.json").exists());
    }

    // ── 修辞模式正则编译验证 ──────────────────────────────────

    #[test]
    fn test_chinese_rhetorical_patterns_compile() {
        let patterns = chinese_rhetorical_patterns();
        assert_eq!(patterns.len(), 5, "中文应有 5 个修辞模式（排比除外）");
        // 验证所有正则都能匹配预期文本
        for p in &patterns {
            assert!(!p.regex.is_match("无匹配文本"), "模式 {} 不应匹配空文本", p.label_zh);
        }
    }

    #[test]
    fn test_english_rhetorical_patterns_compile() {
        let patterns = english_rhetorical_patterns();
        assert_eq!(patterns.len(), 4, "英文应有 4 个修辞模式");
    }
}
