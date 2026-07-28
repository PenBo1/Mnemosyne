//! ═══════════════════════════════════════════════════════════════════════════
//! Sensitive Words Detection - 敏感词检测
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 基础词表 + 字面匹配。
//! 词表内嵌为常量（不依赖外部文件），覆盖中文网络小说平台常见审查类别：
//!   - 政治敏感（Critical）
//!   - 色情低俗（Critical）
//!   - 极端暴力（Warning）
//!   - 自残自杀（Warning）
//!   - 毒品违禁（Warning）
//!
//! 设计为可扩展：未来可通过 McpConfig 或独立配置文件扩展词表。
//! 当前仅提供基线检测，避免误伤（词表保守）。

use crate::domain::pipeline::agents::continuity::{AuditIssue, IssueSeverity, RepairScope};

// ── 敏感词表（内嵌常量） ───────────────────────────────────────────────────

/// 敏感词条目：(词, 类别, 严重级别)
const SENSITIVE_WORDS: &[(&str, &str, IssueSeverity)] = &[
    // 政治敏感（保守，仅保留明确违禁的泛化表述）
    ("推翻政府", "政治敏感", IssueSeverity::Critical),
    ("颠覆国家", "政治敏感", IssueSeverity::Critical),
    ("分裂国家", "政治敏感", IssueSeverity::Critical),
    // 色情低俗
    ("性交", "色情低俗", IssueSeverity::Critical),
    ("做爱", "色情低俗", IssueSeverity::Critical),
    ("性器官", "色情低俗", IssueSeverity::Critical),
    ("裸体", "色情低俗", IssueSeverity::Warning),
    ("性虐", "色情低俗", IssueSeverity::Critical),
    // 极端暴力
    ("肢解", "极端暴力", IssueSeverity::Warning),
    ("分尸", "极端暴力", IssueSeverity::Warning),
    ("虐杀", "极端暴力", IssueSeverity::Warning),
    ("活剥", "极端暴力", IssueSeverity::Warning),
    // 自残自杀
    ("自杀方法", "自残自杀", IssueSeverity::Critical),
    ("割腕", "自残自杀", IssueSeverity::Warning),
    ("上吊", "自残自杀", IssueSeverity::Warning),
    // 毒品违禁
    ("冰毒", "毒品违禁", IssueSeverity::Critical),
    ("海洛因", "毒品违禁", IssueSeverity::Critical),
    ("摇头丸", "毒品违禁", IssueSeverity::Critical),
    ("吸毒", "毒品违禁", IssueSeverity::Warning),
    ("制毒", "毒品违禁", IssueSeverity::Critical),
];

// ── 主函数 ───────────────────────────────────────────────────

type CategoryIssues<'a> = std::collections::BTreeMap<&'a str, (IssueSeverity, Vec<(String, usize)>)>;

/// 检测文本中的敏感词，返回 issues 列表。
///
/// 同一类别下多个匹配聚合为一条 issue（避免 issues 列表爆炸）。
/// 每条 issue 的 description 列出该类别下所有命中的词及次数。
pub fn analyze_sensitive_words(content: &str) -> Vec<AuditIssue> {
    let lower = content.to_lowercase();
    let mut by_category: CategoryIssues<'_> = std::collections::BTreeMap::new();

    for (word, category, severity) in SENSITIVE_WORDS {
        // 英文词用小写匹配，中文词直接匹配
        let count = if word.is_ascii() {
            lower.matches(&word.to_lowercase()).count()
        } else {
            content.matches(*word).count()
        };
        if count > 0 {
            let entry = by_category
                .entry(category)
                .or_insert_with(|| (severity.clone(), Vec::new()));
            // 保留该类别下的最高严重级别
            if severity_is_higher(severity, &entry.0) {
                entry.0 = severity.clone();
            }
            entry.1.push((word.to_string(), count));
        }
    }

    let mut issues = Vec::new();
    for (category, (severity, hits)) in by_category {
        let detail = hits
            .iter()
            .map(|(w, c)| format!("\"{}\"×{}", w, c))
            .collect::<Vec<_>>()
            .join("、");
        let total: usize = hits.iter().map(|(_, c)| c).sum();
        issues.push(AuditIssue {
            severity,
            repair_scope: Some(RepairScope::Local),
            category: category.to_string(),
            description: format!(
                "检测到{}类敏感词共{}次：{}",
                category, total, detail
            ),
            suggestion: format!("删除或替换{}类敏感词，避免平台审查风险", category),
        });
    }
    issues
}

/// 判断 severity_a 是否比 severity_b 更严重。
fn severity_is_higher(a: &IssueSeverity, b: &IssueSeverity) -> bool {
    fn rank(s: &IssueSeverity) -> u8 {
        match s {
            IssueSeverity::Critical => 3,
            IssueSeverity::Warning => 2,
            IssueSeverity::Info => 1,
        }
    }
    rank(a) > rank(b)
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_sensitive_words() {
        let content = "这是一个普通的故事，没有敏感内容。";
        let issues = analyze_sensitive_words(content);
        assert!(issues.is_empty());
    }

    #[test]
    fn detects_drug_word() {
        let content = "他发现了冰毒的踪迹。";
        let issues = analyze_sensitive_words(content);
        assert!(issues.iter().any(|i| i.category == "毒品违禁" && i.severity == IssueSeverity::Critical));
    }

    #[test]
    fn detects_multiple_categories() {
        let content = "冰毒和海洛因都是毒品。他看见分尸现场。";
        let issues = analyze_sensitive_words(content);
        let categories: Vec<&str> = issues.iter().map(|i| i.category.as_str()).collect();
        assert!(categories.contains(&"毒品违禁"));
        assert!(categories.contains(&"极端暴力"));
    }

    #[test]
    fn aggregates_same_category() {
        let content = "冰毒和海洛因都是毒品。摇头丸也是。";
        let issues = analyze_sensitive_words(content);
        let drug_issues: Vec<_> = issues.iter().filter(|i| i.category == "毒品违禁").collect();
        assert_eq!(drug_issues.len(), 1, "同类别应聚合为一条");
        // description 应包含三个词
        assert!(drug_issues[0].description.contains("冰毒"));
        assert!(drug_issues[0].description.contains("海洛因"));
        assert!(drug_issues[0].description.contains("摇头丸"));
    }

    #[test]
    fn critical_severity_for_political() {
        let content = "有人企图推翻政府。";
        let issues = analyze_sensitive_words(content);
        assert!(issues.iter().any(|i| i.category == "政治敏感" && i.severity == IssueSeverity::Critical));
    }

    #[test]
    fn empty_content_returns_no_issues() {
        let issues = analyze_sensitive_words("");
        assert!(issues.is_empty());
    }

    #[test]
    fn count_multiple_occurrences() {
        let content = "冰毒冰毒冰毒";
        let issues = analyze_sensitive_words(content);
        let drug = issues.iter().find(|i| i.category == "毒品违禁").unwrap();
        assert!(drug.description.contains("×3"));
    }
}
