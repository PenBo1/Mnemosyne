// AI Tells 检测 —— 结构化 AI 味检测（纯规则，无 LLM）。
//
// 4 维度纯规则分析：
//   - dim 20: 段落长度均匀度（变异系数 < 0.15 触发 warning）
//   - dim 21: 套话词密度（> 3/千字触发 warning）
//   - dim 22: 公式化转折词重复（同一词 ≥ 3 次触发 warning）
//   - dim 23: 列表式结构（≥ 3 句同前缀触发 info）
//
// 输出 AuditIssue，可直接合并到 AuditResult.issues。

use crate::domain::pipeline::agents::continuity::{AuditIssue, IssueSeverity, RepairScope};
use crate::domain::pipeline::types::Language;

// ── 词表 ─────────────────────────────────────────────────────

const HEDGE_WORDS_ZH: &[&str] = &[
    "似乎", "可能", "或许", "大概", "某种程度上", "一定程度上", "在某种意义上",
];

const HEDGE_WORDS_EN: &[&str] = &[
    "seems", "seemed", "perhaps", "maybe", "apparently", "in some ways", "to some extent",
];

const TRANSITION_WORDS_ZH: &[&str] = &[
    "然而", "不过", "与此同时", "另一方面", "尽管如此", "话虽如此", "但值得注意的是",
];

const TRANSITION_WORDS_EN: &[&str] = &[
    "however", "meanwhile", "on the other hand", "nevertheless", "even so", "still",
];

// ── 主函数 ───────────────────────────────────────────────────

/// 分析文本的 AI 味结构特征，返回 issues 列表。
pub fn analyze_ai_tells(content: &str, language: Language) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    let is_english = matches!(language, Language::En);

    let paragraphs: Vec<&str> = content
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    // dim 20: 段落长度均匀度（需 ≥5 段；短文本 <5 段跳过避免噪声）
    if paragraphs.len() >= 5 {
        let lengths: Vec<f32> = paragraphs.iter().map(|p| p.chars().count() as f32).collect();
        let mean = lengths.iter().sum::<f32>() / lengths.len() as f32;
        if mean > 0.0 {
            let variance = lengths.iter().map(|l| (l - mean).powi(2)).sum::<f32>()
                / lengths.len() as f32;
            let std_dev = variance.sqrt();
            let cv = std_dev / mean;
            if cv < 0.15 {
                issues.push(AuditIssue {
                    severity: IssueSeverity::Warning,
                    repair_scope: Some(RepairScope::Local),
                    category: if is_english {
                        "Paragraph uniformity".to_string()
                    } else {
                        "段落等长".to_string()
                    },
                    description: if is_english {
                        format!(
                            "Paragraph-length coefficient of variation is only {:.3} (threshold <0.15), which suggests unnaturally uniform paragraph sizing",
                            cv
                        )
                    } else {
                        format!(
                            "段落长度变异系数仅{:.3}（阈值<0.15），段落长度过于均匀，呈现AI生成特征",
                            cv
                        )
                    },
                    suggestion: if is_english {
                        "Increase paragraph-length contrast: use shorter beats for impact and longer blocks for immersive detail".to_string()
                    } else {
                        "增加段落长度差异：短段落用于节奏加速或冲击，长段落用于沉浸描写".to_string()
                    },
                });
            }
        }
    }

    // dim 21: 套话词密度（> 3/千字）
    let total_chars = content.chars().count();
    if total_chars > 0 {
        let hedge_words = if is_english { HEDGE_WORDS_EN } else { HEDGE_WORDS_ZH };
        let hedge_count: usize = if is_english {
            let lower = content.to_lowercase();
            hedge_words.iter().map(|w| lower.matches(w).count()).sum()
        } else {
            hedge_words.iter().map(|w| content.matches(w).count()).sum()
        };
        let hedge_density = hedge_count as f32 / (total_chars as f32 / 1000.0);
        if hedge_density > 3.0 {
            issues.push(AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: Some(RepairScope::Local),
                category: if is_english {
                    "Hedge density".to_string()
                } else {
                    "套话密度".to_string()
                },
                description: if is_english {
                    format!(
                        "Hedge-word density is {:.1} per 1k characters (threshold >3), making the prose sound overly tentative",
                        hedge_density
                    )
                } else {
                    format!(
                        "套话词（似乎/可能/或许等）密度为{:.1}次/千字（阈值>3），语气过于模糊犹豫",
                        hedge_density
                    )
                },
                suggestion: if is_english {
                    "Replace hedges with firmer narration: remove vague qualifiers and use concrete detail instead".to_string()
                } else {
                    "用确定性叙述替代模糊表达：去掉「似乎」直接描述状态，用具体细节替代「可能」".to_string()
                },
            });
        }
    }

    // dim 22: 公式化转折词重复（同一词 ≥3 次）
    let transition_words = if is_english { TRANSITION_WORDS_EN } else { TRANSITION_WORDS_ZH };
    let mut transition_counts: Vec<(&str, usize)> = Vec::new();
    for &word in transition_words {
        let count = if is_english {
            content.to_lowercase().matches(word).count()
        } else {
            content.matches(word).count()
        };
        if count > 0 {
            transition_counts.push((word, count));
        }
    }
    let repeated: Vec<&(&str, usize)> = transition_counts.iter().filter(|(_, c)| *c >= 3).collect();
    if !repeated.is_empty() {
        let joiner = if is_english { ", " } else { "、" };
        let detail = repeated
            .iter()
            .map(|(w, c)| format!("\"{}\"×{}", w, c))
            .collect::<Vec<_>>()
            .join(joiner);
        issues.push(AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: if is_english {
                "Formulaic transitions".to_string()
            } else {
                "公式化转折".to_string()
            },
            description: if is_english {
                format!(
                    "Transition words repeat too often: {}. Reusing the same transition pattern 3+ times creates a formulaic AI texture",
                    detail
                )
            } else {
                format!("转折词重复使用：{}。同一转折模式≥3次暴露AI生成痕迹", detail)
            },
            suggestion: if is_english {
                "Let scenes pivot through action, timing, or viewpoint shifts instead of repeating the same transitions".to_string()
            } else {
                "用情节自然转折替代转折词，或换用不同的过渡手法（动作切入、时间跳跃、视角切换）".to_string()
            },
        });
    }

    // dim 23: 列表式结构（连续同前缀句子 ≥3）
    let sentences: Vec<String> = if is_english {
        content
            .split(['.', '!', '?', '\n'])
            .map(|s| s.trim().to_string())
            .filter(|s| s.chars().count() > 2)
            .collect()
    } else {
        content
            .split(['。', '！', '？', '\n'])
            .map(|s| s.trim().to_string())
            .filter(|s| s.chars().count() > 2)
            .collect()
    };

    if sentences.len() >= 3 {
        let mut max_consecutive = 1usize;
        let mut consecutive = 1usize;
        for i in 1..sentences.len() {
            let prev_prefix = sentence_prefix(&sentences[i - 1], is_english);
            let curr_prefix = sentence_prefix(&sentences[i], is_english);
            if prev_prefix == curr_prefix && !prev_prefix.is_empty() {
                consecutive += 1;
                if consecutive > max_consecutive {
                    max_consecutive = consecutive;
                }
            } else {
                consecutive = 1;
            }
        }
        if max_consecutive >= 3 {
            issues.push(AuditIssue {
                severity: IssueSeverity::Info,
                repair_scope: Some(RepairScope::Local),
                category: if is_english {
                    "List-like structure".to_string()
                } else {
                    "列表式结构".to_string()
                },
                description: if is_english {
                    format!(
                        "Detected {} consecutive sentences with the same opening pattern, creating a list-like generated cadence",
                        max_consecutive
                    )
                } else {
                    format!(
                        "检测到{}句连续以相同开头的句子，呈现列表式AI生成结构",
                        max_consecutive
                    )
                },
                suggestion: if is_english {
                    "Vary how sentences open: change subject, timing, or action entry to break the list effect".to_string()
                } else {
                    "变换句式开头：用不同主语、时间词、动作词开头，打破列表感".to_string()
                },
            });
        }
    }

    issues
}

/// 提取句子前缀：英文取首个单词（小写），中文取前 2 字符。
fn sentence_prefix(sentence: &str, is_english: bool) -> String {
    if is_english {
        sentence
            .split_whitespace()
            .next()
            .map(|w| w.to_lowercase())
            .unwrap_or_default()
    } else {
        sentence.chars().take(2).collect()
    }
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_issues_for_varied_paragraphs() {
        let content = "短段。\n\n这是一个中等长度的段落，包含一些叙述内容。\n\n这是一个更长的段落，用于沉浸式描写，包含更多的细节和感官描写，让读者能够身临其境地感受到场景。";
        let issues = analyze_ai_tells(content, Language::Zh);
        // 不应触发段落均匀度（CV 应较高）
        assert!(!issues.iter().any(|i| i.category == "段落等长"));
    }

    #[test]
    fn detects_uniform_paragraphs() {
        // 五个长度几乎相同的段落（满足 dim 20 的 ≥5 段阈值）
        let content = "一二三四五六七八九十一。\n\n一二三四五六七八九十二。\n\n一二三四五六七八九十三。\n\n一二三四五六七八九十四。\n\n一二三四五六七八九十五。";
        let issues = analyze_ai_tells(content, Language::Zh);
        assert!(
            issues.iter().any(|i| i.category == "段落等长"),
            "应检测到段落等长，got: {:?}",
            issues.iter().map(|i| &i.category).collect::<Vec<_>>()
        );
    }

    #[test]
    fn detects_hedge_density() {
        // 在短文本中高频出现套话词
        let content = "这似乎是一个可能的故事。或许大概如此。似乎可能或许大概。";
        let issues = analyze_ai_tells(content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "套话密度"));
    }

    #[test]
    fn detects_repeated_transitions() {
        let content = "然而第一段。\n\n然而第二段。\n\n然而第三段。";
        let issues = analyze_ai_tells(content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "公式化转折"));
    }

    #[test]
    fn detects_list_like_structure() {
        // 连续 3 句以相同 2 字前缀开头
        let content = "他走出门外。他走过石桥。他走向远处。";
        let issues = analyze_ai_tells(content, Language::Zh);
        assert!(issues.iter().any(|i| i.category == "列表式结构"));
    }

    #[test]
    fn empty_content_returns_no_issues() {
        let issues = analyze_ai_tells("", Language::Zh);
        assert!(issues.is_empty());
    }

    #[test]
    fn english_hedge_detection() {
        let content = "It seems perhaps maybe that this is apparently the case. Seemed maybe perhaps seemingly so. Perhaps maybe apparently seemed.";
        let issues = analyze_ai_tells(content, Language::En);
        assert!(issues.iter().any(|i| i.category == "Hedge density"));
    }

    #[test]
    fn all_issues_have_local_repair_scope() {
        let content = "然而第一段。\n\n然而第二段。\n\n然而第三段。";
        let issues = analyze_ai_tells(content, Language::Zh);
        for issue in &issues {
            assert_eq!(issue.repair_scope, Some(RepairScope::Local));
        }
    }
}
