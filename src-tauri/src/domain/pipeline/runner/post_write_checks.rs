// Post-write 确定性校验。
//
// 提供：
//   - normalize_post_write_surface: 剥离 meta 备注行 + 替换破折号
//   - assert_chapter_content_not_empty: 内容非空断言
//   - run_post_write_checks: content-only 确定性规则校验
//   - run_post_write_checks_with_config: 完整校验（content-only + 配置依赖）
//   - detect_cross_chapter_repetition: 跨章重复检测（独立入口）

use std::sync::OnceLock;

use crate::domain::pipeline::agents::continuity::{AuditIssue, IssueSeverity, RepairScope};
use crate::domain::pipeline::types::Language;
use crate::shared::error::AppError;

// ── 标记词表 ─────────────────────────────────────────────────

/// AI 转折/惊讶标记词
const SURPRISE_MARKERS: &[&str] = &[
    "仿佛", "忽然", "竟然", "猛地", "猛然", "不禁", "宛如",
];

/// 元叙事/编剧旁白模式（正则）
const META_NARRATION_PATTERNS: &[&str] = &[
    r"到这里[，,]?算是",
    r"接下来[，,]?(?:就是|将会|即将)",
    r"(?:后面|之后)[，,]?(?:会|将|还会)",
    r"(?:故事|剧情)(?:发展)?到了",
    r"读者[，,]?(?:可能|应该|也许)",
    r"我们[，,]?(?:可以|不妨|来看)",
];

/// 分析报告式术语（禁止出现在正文中）
const REPORT_TERMS: &[&str] = &[
    "核心动机", "信息边界", "信息落差", "核心风险", "利益最大化",
    "当前处境", "行为约束", "性格过滤", "情绪外化", "锚定效应",
    "沉没成本", "认知共鸣",
];

/// 作者说教词
const SERMON_WORDS: &[&str] = &[
    "显然", "毋庸置疑", "不言而喻", "众所周知", "不难看出",
];

/// 全场震惊类集体反应模式（正则）
const COLLECTIVE_SHOCK_PATTERNS: &[&str] = &[
    r"(?:全场|众人|所有人|在场的人)[，,]?(?:都|全|齐齐|纷纷)?(?:震惊|惊呆|倒吸凉气|目瞪口呆|哗然|惊呼)",
    r"(?:全场|一片)[，,]?(?:寂静|哗然|沸腾|震动)",
];

/// 英文 AI-tell 词
const EN_AI_TELL_WORDS: &[&str] = &[
    "delve", "tapestry", "testament", "intricate", "pivotal",
    "vibrant", "embark", "comprehensive", "nuanced",
];

// ── 正则缓存（OnceLock，避免每章/每次调用重新编译）─────────

/// 缓存 META_NARRATION_PATTERNS 编译后的 Vec<Regex>
static META_NARRATION_REGEXES: OnceLock<Vec<regex::Regex>> = OnceLock::new();
fn meta_narration_regexes() -> &'static [regex::Regex] {
    META_NARRATION_REGEXES.get_or_init(|| {
        META_NARRATION_PATTERNS
            .iter()
            .map(|p| regex::Regex::new(p).expect("valid meta-narration regex"))
            .collect()
    })
}

/// 缓存 COLLECTIVE_SHOCK_PATTERNS 编译后的 Vec<Regex>
static COLLECTIVE_SHOCK_REGEXES: OnceLock<Vec<regex::Regex>> = OnceLock::new();
fn collective_shock_regexes() -> &'static [regex::Regex] {
    COLLECTIVE_SHOCK_REGEXES.get_or_init(|| {
        COLLECTIVE_SHOCK_PATTERNS
            .iter()
            .map(|p| regex::Regex::new(p).expect("valid collective-shock regex"))
            .collect()
    })
}

/// 缓存 EN_AI_TELL_WORDS 编译后的 Vec<Regex>（含 word boundary）
static EN_AI_TELL_REGEXES: OnceLock<Vec<regex::Regex>> = OnceLock::new();
fn en_ai_tell_regexes() -> &'static [regex::Regex] {
    EN_AI_TELL_REGEXES.get_or_init(|| {
        EN_AI_TELL_WORDS
            .iter()
            .map(|w| {
                regex::Regex::new(&format!(r"(?i)\b{}\b", w))
                    .expect("valid word-boundary regex")
            })
            .collect()
    })
}

// ── normalize / assert ───────────────────────────────────────

/// 归一化 post-write 表面文本：剥离 meta 备注行 + 替换破折号（仅 zh）。
/// - 剥离 [polisher-note] / [writer-note] / [reviser-note] / [reviewer-note] 行
/// - 剥离 [润色备注] / [写作备注] / [修订备注] / [审稿备注] 行
/// - zh: 将 `——`（2+ 破折号）替换为 `，`
pub fn normalize_post_write_surface(content: &str, language: Language) -> String {
    let stripped = strip_post_write_meta_lines(content);
    let normalized = if matches!(language, Language::En) {
        stripped
    } else {
        // 替换 2+ 连续破折号为逗号（缓存正则避免重复编译）
        static EM_DASH: OnceLock<regex::Regex> = OnceLock::new();
        let re = EM_DASH.get_or_init(|| {
            regex::Regex::new(r"——+").expect("valid em-dash regex")
        });
        re.replace_all(&stripped, "，").to_string()
    };
    normalized.trim_end().to_string()
}

fn strip_post_write_meta_lines(content: &str) -> String {
    content
        .lines()
        .filter(|line| !is_meta_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_meta_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let lower = trimmed.to_lowercase();
    lower.starts_with("[polisher-note]")
        || lower.starts_with("[writer-note]")
        || lower.starts_with("[reviser-note]")
        || lower.starts_with("[reviewer-note]")
        || trimmed.starts_with("[润色备注]")
        || trimmed.starts_with("[写作备注]")
        || trimmed.starts_with("[修订备注]")
        || trimmed.starts_with("[审稿备注]")
}

/// 断言章节内容非空（normalize 后）。
///
/// 空内容返回 Err（对齐 "no silent fallback"，不静默跳过）。
pub fn assert_chapter_content_not_empty(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::invalid_input(
            "Chapter content is empty after normalization",
        ));
    }
    Ok(())
}

// ── 主校验函数 ───────────────────────────────────────────────

/// 运行 post-write 确定性校验。
///
/// severity 映射：TS "error" → Critical（强制 passed=false），TS "warning" → Warning。
pub fn run_post_write_checks(content: &str, language: Language) -> Vec<AuditIssue> {
    match language {
        Language::En => validate_post_write_english(content),
        Language::Zh => validate_post_write_chinese(content),
    }
}

fn validate_post_write_chinese(content: &str) -> Vec<AuditIssue> {
    let mut issues = Vec::new();

    // 1. "不是…而是…" 句式（Critical）
    static BU_SHI: OnceLock<regex::Regex> = OnceLock::new();
    let bu_shi_re = BU_SHI.get_or_init(|| {
        regex::Regex::new(r"不是[^，。！？\n]{0,30}[，,]?\s*而是").expect("valid regex")
    });
    if bu_shi_re.is_match(content) {
        issues.push(AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Local),
            category: "禁止句式".to_string(),
            description: "出现了「不是……而是……」句式".to_string(),
            suggestion: "改用直述句".to_string(),
        });
    }

    // 2. 破折号（Critical）
    if content.contains("——") {
        issues.push(AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Local),
            category: "禁止破折号".to_string(),
            description: "出现了破折号「——」".to_string(),
            suggestion: "用逗号或句号断句".to_string(),
        });
    }

    // 3. 转折/惊讶标记词密度 ≤ 1次/3000字
    let mut marker_counts: Vec<(&str, usize)> = Vec::new();
    let mut total_marker_count = 0usize;
    for &word in SURPRISE_MARKERS {
        let count = content.matches(word).count();
        if count > 0 {
            marker_counts.push((word, count));
            total_marker_count += count;
        }
    }
    let content_len = content.chars().count();
    let marker_limit = (content_len / 3000).max(1);
    if total_marker_count > marker_limit {
        let detail = marker_counts
            .iter()
            .map(|(w, c)| format!("\"{}\"×{}", w, c))
            .collect::<Vec<_>>()
            .join("、");
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: "转折词密度".to_string(),
            description: format!(
                "转折/惊讶标记词共{}次（上限{}次/{}字），明细：{}",
                total_marker_count, marker_limit, content_len, detail
            ),
            suggestion: "改用具体动作或感官描写传递突然性".to_string(),
        });
    }

    // 4. 元叙事检查（编剧旁白）
    for re in meta_narration_regexes() {
        if let Some(m) = re.find(content) {
            issues.push(AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "元叙事".to_string(),
                description: format!("出现编剧旁白式表述：\"{}\"", m.as_str()),
                suggestion: "删除元叙事，让剧情自然展开".to_string(),
            });
            break; // 报一次即可
        }
    }

    // 5. 分析报告式术语（Critical）
    let found_terms: Vec<&str> = REPORT_TERMS.iter().filter(|t| content.contains(**t)).copied().collect();
    if !found_terms.is_empty() {
        let detail = found_terms.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join("、");
        issues.push(AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Local),
            category: "报告术语".to_string(),
            description: format!("正文中出现分析报告术语：{}", detail),
            suggestion: "这些术语只能用于内部推理，正文中用口语化表达替代".to_string(),
        });
    }

    // 6. 章节号指称（Critical）
    static CHAPTER_REF: OnceLock<regex::Regex> = OnceLock::new();
    let chapter_ref_re = CHAPTER_REF.get_or_init(|| {
        regex::Regex::new(r"(?:第\s*\d+\s*章|[Cc]hapter\s+\d+)").expect("valid regex")
    });
    let chapter_refs: Vec<String> = chapter_ref_re
        .find_iter(content)
        .map(|m| m.as_str().to_string())
        .collect();
    if !chapter_refs.is_empty() {
        // 去重
        let mut unique: Vec<String> = Vec::new();
        for r in &chapter_refs {
            if !unique.contains(r) {
                unique.push(r.clone());
            }
        }
        let detail = unique.iter().map(|r| format!("\"{}\"", r)).collect::<Vec<_>>().join("、");
        issues.push(AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Local),
            category: "章节号指称".to_string(),
            description: format!("正文中出现了章节号指称：{}。角色不知道自己在第几章", detail),
            suggestion: "改成自然表达：「那天晚上」、「仓库出事那次」、「码头上的事」".to_string(),
        });
    }

    // 7. 作者说教词（Warning）
    let found_sermons: Vec<&str> = SERMON_WORDS.iter().filter(|w| content.contains(**w)).copied().collect();
    if !found_sermons.is_empty() {
        let detail = found_sermons.iter().map(|w| format!("\"{}\"", w)).collect::<Vec<_>>().join("、");
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: "作者说教".to_string(),
            description: format!("出现说教词：{}", detail),
            suggestion: "删除说教词，让读者自己从情节中判断".to_string(),
        });
    }

    // 8. 全场震惊类集体反应（Warning）
    for re in collective_shock_regexes() {
        if let Some(m) = re.find(content) {
            issues.push(AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "集体反应".to_string(),
                description: format!("出现集体反应套话：\"{}\"", m.as_str()),
                suggestion: "改写成1-2个具体角色的身体反应".to_string(),
            });
            break;
        }
    }

    // 9. 连续"了"字检查（3句以上连续含"了"，阈值≥6）
    let sentences: Vec<&str> = content
        .split(['。', '！', '？', '\n'])
        .map(|s| s.trim())
        .filter(|s| s.chars().count() > 2)
        .collect();
    let mut max_consecutive_le = 0usize;
    let mut consecutive_le = 0usize;
    for sentence in &sentences {
        if sentence.contains('了') {
            consecutive_le += 1;
            if consecutive_le > max_consecutive_le {
                max_consecutive_le = consecutive_le;
            }
        } else {
            consecutive_le = 0;
        }
    }
    if max_consecutive_le >= 6 {
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: "连续了字".to_string(),
            description: format!("检测到{}句连续包含\"了\"字，节奏拖沓", max_consecutive_le),
            suggestion: "保留最有力的一个「了」，其余改为无「了」句式".to_string(),
        });
    }

    // 10. 段落过长（手机阅读适配：50-250字/段为宜，>300字为过长）
    let paragraphs = extract_paragraphs(content);
    let long_paragraphs: Vec<&String> = paragraphs.iter().filter(|p| p.chars().count() > 300).collect();
    if long_paragraphs.len() >= 2 {
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: "段落过长".to_string(),
            description: format!("{}个段落超过300字，不适合手机阅读", long_paragraphs.len()),
            suggestion: "长段落拆分为3-5行的短段落，在动作切换或情绪节点处断开".to_string(),
        });
    }

    // 11. 段落形状分析
    issues.extend(detect_paragraph_shape_warnings(content, Language::Zh));

    issues
}

fn validate_post_write_english(content: &str) -> Vec<AuditIssue> {
    let mut issues = Vec::new();

    // 1. AI-tell word density（1 per 3000 chars）
    let limit = content.chars().count().div_ceil(3000);
    let words = EN_AI_TELL_WORDS;
    let regexes = en_ai_tell_regexes();
    for (word, re) in words.iter().zip(regexes.iter()) {
        let count = re.find_iter(content).count();
        if count > limit {
            issues.push(AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "AI-tell word density".to_string(),
                description: format!("\"{}\" appears {} times (limit: {} per 3000 chars)", word, count, limit),
                suggestion: "Replace with a more specific word".to_string(),
            });
        }
    }

    // 2. Paragraph overflow (>500 chars)
    let paragraphs = extract_paragraphs(content);
    let long_paragraphs: Vec<&String> = paragraphs.iter().filter(|p| p.chars().count() > 500).collect();
    if long_paragraphs.len() >= 2 {
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: "Paragraph length".to_string(),
            description: format!("{} paragraphs exceed 500 characters", long_paragraphs.len()),
            suggestion: "Break into shorter paragraphs for readability".to_string(),
        });
    }

    // 3. Paragraph shape
    issues.extend(detect_paragraph_shape_warnings(content, Language::En));

    issues
}

// ── 配置依赖检查（Phase 5c 补全）─────────────────────────────

/// 叙事人称
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NarrativePerson {
    First,
    Third,
}

/// Post-write 配置依赖检查的输入配置。
///
/// 聚焦 4 项配置依赖检查所需的字段。
#[derive(Default)]
pub struct PostWriteCheckConfig<'a> {
    /// 高疲劳词列表（单章每词 ≤ 1 次）
    pub fatigue_words: &'a [String],
    /// 本书禁忌内容列表（短词 2-30 字做子串匹配，长词跳过）
    pub prohibitions: &'a [String],
    /// 叙事人称（None 表示不检测人称漂移）
    pub narrative_person: Option<NarrativePerson>,
    /// 主角名（用于第一人称漂移检测的 nameCount 统计）
    pub protagonist_name: Option<&'a str>,
    /// 近期章节正文（用于跨章重复检测，None 或过短则跳过）
    pub recent_chapters_content: Option<&'a str>,
}


/// 运行完整 post-write 校验：content-only 检查 + 配置依赖检查。
pub fn run_post_write_checks_with_config(
    content: &str,
    language: Language,
    config: &PostWriteCheckConfig<'_>,
) -> Vec<AuditIssue> {
    let mut issues = run_post_write_checks(content, language);

    // 高疲劳词
    issues.extend(check_fatigue_words(content, language, config.fatigue_words));

    // 本书禁忌
    issues.extend(check_prohibitions(content, language, config.prohibitions));

    // 叙事人称漂移（仅第一人称）
    if matches!(config.narrative_person, Some(NarrativePerson::First)) {
        issues.extend(check_narrative_person_drift(content, config.protagonist_name));
    }

    // 跨章重复
    if let Some(recent) = config.recent_chapters_content {
        issues.extend(detect_cross_chapter_repetition(content, recent, language));
    }

    issues
}

/// 高疲劳词检查：单章每词 ≤ 1 次。
/// - zh: 直接子串匹配
/// - en: `\b word \b` 词边界匹配
fn check_fatigue_words(
    content: &str,
    language: Language,
    fatigue_words: &[String],
) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    if fatigue_words.is_empty() {
        return issues;
    }
    let is_english = matches!(language, Language::En);
    for word in fatigue_words {
        let trimmed = word.trim();
        if trimmed.is_empty() {
            continue;
        }
        let count = if is_english {
            let re_str = format!(r"(?i)\b{}\b", regex::escape(trimmed));
            regex::Regex::new(&re_str)
                .map(|re| re.find_iter(content).count())
                .unwrap_or(0)
        } else {
            content.matches(trimmed).count()
        };
        if count > 1 {
            issues.push(AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: if is_english {
                    "Fatigue word".to_string()
                } else {
                    "高疲劳词".to_string()
                },
                description: if is_english {
                    format!("\"{}\" appears {} times (max 1 per chapter)", trimmed, count)
                } else {
                    format!("高疲劳词\"{}\"出现{}次（上限1次/章）", trimmed, count)
                },
                suggestion: if is_english {
                    "Vary the vocabulary".to_string()
                } else {
                    format!("替换多余的\"{}\"为同义但不同形式的表达", trimmed)
                },
            });
        }
    }
    issues
}

/// 本书禁忌内容检查：短词（2-30 字符）做子串匹配。
/// 长词（>30 字符）跳过——这些是概念性规则，由 prompt 层面强制。
fn check_prohibitions(
    content: &str,
    language: Language,
    prohibitions: &[String],
) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    if prohibitions.is_empty() {
        return issues;
    }
    let is_english = matches!(language, Language::En);
    let max_len = if is_english { 50 } else { 30 };
    let content_lower = if is_english {
        content.to_lowercase()
    } else {
        String::new() // zh 不需要 lower
    };
    for prohibition in prohibitions {
        let trimmed = prohibition.trim();
        if trimmed.len() < 2 || trimmed.chars().count() > max_len {
            continue;
        }
        let hit = if is_english {
            content_lower.contains(&trimmed.to_lowercase())
        } else {
            content.contains(trimmed)
        };
        if hit {
            issues.push(AuditIssue {
                severity: IssueSeverity::Critical,
                repair_scope: Some(RepairScope::Local),
                category: if is_english {
                    "Book prohibition".to_string()
                } else {
                    "本书禁忌".to_string()
                },
                description: if is_english {
                    format!("Found banned content: \"{}\"", trimmed)
                } else {
                    format!("出现了本书禁忌内容：\"{}\"", trimmed)
                },
                suggestion: if is_english {
                    "Remove or rewrite this content".to_string()
                } else {
                    "删除或改写该内容".to_string()
                },
            });
        }
    }
    issues
}

/// 叙事人称漂移检测（第一人称书中的第三人称漂移）。
/// 两个信号：
/// 1. 内感泄漏：以「他/她」开头 + 觉得/感到/意识到/明白/想起/脑子里/心里/太阳穴
/// 2. 第三人称叙述：长章节（≥800 字），「我」< 12 次，主角名 ≥ 6 次，且 nameCount > woCount
fn check_narrative_person_drift(content: &str, protagonist_name: Option<&str>) -> Vec<AuditIssue> {
    let mut issues = Vec::new();

    // 信号 1：内感泄漏
    if let Some(slipped) = detect_first_person_inner_state_slip(content) {
        issues.push(AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Local),
            category: "叙事人称".to_string(),
            description: format!(
                "本书设定为第一人称，但出现了第三人称内感叙述：\"{}\"",
                slipped
            ),
            suggestion: "把这类主观感受、意识、脑内活动改回「我」的内心视角；不要切到第三人称或全知视角。".to_string(),
        });
        return issues;
    }

    // 信号 2：第三人称叙述
    if let Some(name) = protagonist_name {
        let name = name.trim();
        if !name.is_empty() {
            let content_len = content.chars().count();
            let wo_count = content.matches('我').count();
            let name_count = content.matches(name).count();
            if content_len >= 800 && wo_count < 12 && name_count >= 6 && name_count > wo_count {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Critical,
                    repair_scope: Some(RepairScope::Local),
                    category: "叙事人称".to_string(),
                    description: format!(
                        "本书设定为第一人称，但本章几乎不用「我」（{} 次）却反复以「{}」第三人称叙述（{} 次）",
                        wo_count, name, name_count
                    ),
                    suggestion: "改用第一人称（主角内心视角）重写本章叙事".to_string(),
                });
            }
        }
    }

    issues
}

/// 检测第一人称内感泄漏。
///
/// 匹配以「他/她」开头，后接 内感动词 的句子。
/// 返回截断后的句子（≤40 字符）。
fn detect_first_person_inner_state_slip(content: &str) -> Option<String> {
    static INNER_STATE: OnceLock<regex::Regex> = OnceLock::new();
    let inner_state_re = INNER_STATE.get_or_init(|| {
        regex::Regex::new(
            r"^[他她][^。！？!?]{0,18}(?:觉得|感到|意识到|明白|想起|脑子里|心里|太阳穴)",
        )
        .expect("valid inner-state regex")
    });

    // 按句号/感叹号/问号/换行 分句
    static SENTENCE_SPLIT: OnceLock<regex::Regex> = OnceLock::new();
    let sentence_split_re = SENTENCE_SPLIT.get_or_init(|| {
        regex::Regex::new(r"[。！？!?]").expect("valid sentence-split regex")
    });
    for line in content.lines() {
        for sentence in sentence_split_re.split(line) {
            let trimmed = sentence.trim();
            if !trimmed.is_empty() && inner_state_re.is_match(trimmed) {
                let truncated: String = trimmed.chars().take(39).collect();
                let display = if trimmed.chars().count() > 40 {
                    format!("{}…", truncated)
                } else {
                    truncated
                };
                return Some(display);
            }
        }
    }
    None
}

/// 跨章重复检测。
/// - zh: 6-char ngram（纯汉字），当前章内出现 ≥ 2 次 且 近期章节也包含 → 计入
/// - en: 3-word phrase，当前章内出现 ≥ 2 次 且 近期章节也包含 → 计入
/// 累计 ≥ 3 个重复短语才报告。
pub fn detect_cross_chapter_repetition(
    current_content: &str,
    recent_chapters_content: &str,
    language: Language,
) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    if recent_chapters_content.len() < 100 {
        return issues;
    }

    let is_english = matches!(language, Language::En);
    let cross_repeats: Vec<String> = if is_english {
        // 提取 3-word phrases
        let cleaned: String = current_content
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '\'')
            .collect();
        let words: Vec<&str> = cleaned
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();
        let mut phrase_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for window in words.windows(3) {
            let phrase = format!("{} {} {}", window[0], window[1], window[2]);
            *phrase_counts.entry(phrase).or_insert(0) += 1;
        }
        let recent_lower = recent_chapters_content.to_lowercase();
        let mut repeats = Vec::new();
        for (phrase, count) in &phrase_counts {
            if *count >= 2 && recent_lower.contains(phrase.as_str()) {
                repeats.push(format!("\"{}\" (×{})", phrase, count));
            }
        }
        repeats
    } else {
        // zh: 6-char ngram（纯汉字）
        let chars: String = current_content
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let mut ngram_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        let char_vec: Vec<char> = chars.chars().collect();
        for i in 0..char_vec.len().saturating_sub(5) {
            let ngram: String = char_vec[i..i + 6].iter().collect();
            // 仅统计纯汉字 ngram
            if ngram.chars().all(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
                *ngram_counts.entry(ngram).or_insert(0) += 1;
            }
        }
        let recent_clean: String = recent_chapters_content
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let mut repeats = Vec::new();
        for (ngram, count) in &ngram_counts {
            if *count >= 2 && recent_clean.contains(ngram.as_str()) {
                repeats.push(format!("\"{}\"(×{})", ngram, count));
            }
        }
        repeats
    };

    if cross_repeats.len() >= 3 {
        let detail = cross_repeats.iter().take(5).cloned().collect::<Vec<_>>().join(if is_english { ", " } else { "、" });
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: if is_english {
                "Cross-chapter repetition".to_string()
            } else {
                "跨章重复".to_string()
            },
            description: if is_english {
                format!(
                    "{} repeated phrases also found in recent chapters: {}",
                    cross_repeats.len(),
                    detail
                )
            } else {
                format!(
                    "{}个重复短语在近期章节中也出现过：{}",
                    cross_repeats.len(),
                    detail
                )
            },
            suggestion: if is_english {
                "Vary action verbs and descriptive phrases to avoid cross-chapter repetition".to_string()
            } else {
                "变换动作描写和场景用语，避免跨章节机械重复".to_string()
            },
        });
    }

    issues
}

// ── 段落形状分析 ─────────────────────────────────────────────

/// 段落形状警告（段落过碎 / 连续短段）
fn detect_paragraph_shape_warnings(content: &str, language: Language) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    let shape = analyze_paragraph_shape(content, language);
    if shape.paragraphs.len() < 4 {
        return issues;
    }

    let short_threshold = shape.short_threshold;
    if shape.short_paragraphs.len() >= 4 && shape.short_ratio >= 0.6 {
        issues.push(if matches!(language, Language::En) {
            AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "Paragraph fragmentation".to_string(),
                description: format!(
                    "{} of {} paragraphs are shorter than {} characters.",
                    shape.short_paragraphs.len(),
                    shape.paragraphs.len(),
                    short_threshold
                ),
                suggestion: "Merge adjacent action, observation, and reaction beats so the chapter does not collapse into one-line paragraphs.".to_string(),
            }
        } else {
            AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "段落过碎".to_string(),
                description: format!(
                    "{}个段落里有{}个不足{}字，段落被切得过碎。",
                    shape.paragraphs.len(),
                    shape.short_paragraphs.len(),
                    short_threshold
                ),
                suggestion: "把相邻的动作、观察、反应适当并段，不要每句话都单独起段。".to_string(),
            }
        });
    }

    if shape.max_consecutive_short >= 3 {
        issues.push(if matches!(language, Language::En) {
            AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "Consecutive short paragraphs".to_string(),
                description: format!("{} short paragraphs appear back to back.", shape.max_consecutive_short),
                suggestion: "Break the one-beat-per-paragraph rhythm by folding connected beats into fuller paragraphs.".to_string(),
            }
        } else {
            AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: "连续短段".to_string(),
                description: format!("连续出现{}个不足{}字的短段，容易形成短句堆砌。", shape.max_consecutive_short, short_threshold),
                suggestion: "把连续的碎动作重新编组，至少让一个段落承载完整的动作链或情绪推进。".to_string(),
            }
        });
    }

    issues
}

struct ParagraphShape {
    paragraphs: Vec<String>,
    short_threshold: usize,
    short_paragraphs: Vec<String>,
    short_ratio: f32,
    #[allow(dead_code)]
    average_length: f32,
    max_consecutive_short: usize,
}

fn analyze_paragraph_shape(content: &str, language: Language) -> ParagraphShape {
    let paragraphs = extract_paragraphs(content);
    let narrative_paragraphs: Vec<&String> = paragraphs.iter().filter(|p| !is_dialogue_paragraph(p)).collect();
    let short_threshold = if matches!(language, Language::En) { 120 } else { 35 };
    let short_paragraphs: Vec<String> = narrative_paragraphs
        .iter()
        .filter(|p| p.chars().count() < short_threshold)
        .map(|p| (*p).clone())
        .collect();
    let average_length = if !paragraphs.is_empty() {
        paragraphs.iter().map(|p| p.chars().count()).sum::<usize>() as f32 / paragraphs.len() as f32
    } else {
        0.0
    };

    let mut max_consecutive_short = 0usize;
    let mut current = 0usize;
    for p in &narrative_paragraphs {
        if p.chars().count() < short_threshold {
            current += 1;
            if current > max_consecutive_short {
                max_consecutive_short = current;
            }
        } else {
            current = 0;
        }
    }

    let short_ratio = if !narrative_paragraphs.is_empty() {
        short_paragraphs.len() as f32 / narrative_paragraphs.len() as f32
    } else {
        0.0
    };

    ParagraphShape {
        paragraphs,
        short_threshold,
        short_paragraphs,
        short_ratio,
        average_length,
        max_consecutive_short,
    }
}

fn is_dialogue_paragraph(paragraph: &str) -> bool {
    let trimmed = paragraph.trim();
    let first = trimmed.chars().next();
    matches!(first, Some('"') | Some('\u{201C}') | Some('\u{201D}') | Some('「') | Some('『') | Some('\u{2019}') | Some('《'))
        || trimmed.starts_with("——")
}

fn extract_paragraphs(content: &str) -> Vec<String> {
    static PARAGRAPH_SPLIT: OnceLock<regex::Regex> = OnceLock::new();
    let re = PARAGRAPH_SPLIT.get_or_init(|| {
        regex::Regex::new(r"\n\s*\n").expect("valid paragraph-split regex")
    });
    re.split(content)
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .filter(|p| p != "---")
        .filter(|p| !p.starts_with('#'))
        .collect()
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_meta_lines() {
        let content = "[writer-note]\n这是正文。\n[polisher-note]\n继续正文。";
        let normalized = normalize_post_write_surface(content, Language::Zh);
        assert!(!normalized.contains("[writer-note]"));
        assert!(!normalized.contains("[polisher-note]"));
        assert!(normalized.contains("这是正文。"));
        assert!(normalized.contains("继续正文。"));
    }

    #[test]
    fn normalize_replaces_em_dash_zh() {
        let content = "他走了过来——然后停下。";
        let normalized = normalize_post_write_surface(content, Language::Zh);
        assert!(!normalized.contains("——"));
        assert!(normalized.contains("，"));
    }

    #[test]
    fn normalize_keeps_em_dash_en() {
        let content = "He came over — then stopped.";
        let normalized = normalize_post_write_surface(content, Language::En);
        // 英文不替换破折号（但单个 — 不在替换范围，—— 才替换）
        assert_eq!(normalized, content.trim_end());
    }

    #[test]
    fn assert_empty_fails() {
        assert!(assert_chapter_content_not_empty("").is_err());
        assert!(assert_chapter_content_not_empty("   \n  ").is_err());
    }

    #[test]
    fn assert_non_empty_passes() {
        assert!(assert_chapter_content_not_empty("内容").is_ok());
    }

    #[test]
    fn detects_bu_shi_pattern() {
        let content = "这不是勇气，而是鲁莽。";
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "禁止句式" && i.severity == IssueSeverity::Critical));
    }

    #[test]
    fn detects_em_dash() {
        let content = "他走了过来——然后停下。";
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "禁止破折号" && i.severity == IssueSeverity::Critical));
    }

    #[test]
    fn detects_report_terms() {
        let content = "他的核心动机是利益最大化，这造成了信息落差。";
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "报告术语" && i.severity == IssueSeverity::Critical));
    }

    #[test]
    fn detects_chapter_ref() {
        let content = "回想第33章的事，他叹了口气。";
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "章节号指称"));
    }

    #[test]
    fn detects_sermon_words() {
        let content = "显然，他赢了。毋庸置疑，这是最好的结果。";
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "作者说教"));
    }

    #[test]
    fn detects_consecutive_le() {
        // 6+ 句连续含"了"
        let content = "他走了。他跑了。他停了。他笑了。他哭了。他醒了。他睡了。";
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "连续了字"));
    }

    #[test]
    fn no_issues_for_clean_content() {
        let content = "山风穿过松林，发出低沉的呜咽。\n\n月光洒在石阶上，像一层薄霜。\n\n他握紧了手中的剑柄，指节微微泛白。远处的灯火忽明忽暗，仿佛在等待什么人。";
        let issues = run_post_write_checks(&content, Language::Zh);
        // 不应有关键违规
        assert!(!issues.iter().any(|i| i.severity == IssueSeverity::Critical), "got critical: {:?}", issues.iter().filter(|i| i.severity == IssueSeverity::Critical).collect::<Vec<_>>());
    }

    #[test]
    fn english_detects_ai_tell_words() {
        // "delve" 出现 2 次，超过 ceil(len/3000)=1 的限额
        let content = "Let us delve into this. We delve deeper into the matter.";
        let issues = run_post_write_checks(&content, Language::En);
        assert!(issues.iter().any(|i| i.category == "AI-tell word density"));
    }

    #[test]
    fn detects_long_paragraphs() {
        let long = "一".repeat(350);
        let content = format!("{}\n\n{}", long, long);
        let issues = run_post_write_checks(&content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "段落过长"));
    }
}
