//! S6.3: Auto 模式路由逻辑。
//!
//! 移植自 inkos `agents/reviser.ts` 的 `resolveAutoOutputMode` +
//! `LOCAL_ONLY_PATTERNS` / `STRUCTURAL_PATTERNS`。根据 audit issues 的
//! `repair_scope` 字段和 category 文本模式，决定 reviser 应当：
//! - `PatchOnly`：只输出 PATCHES（局部问题）
//! - `RewriteOnly`：只输出 REVISED_CONTENT（结构问题，PATCHES 无法修复）
//! - `AllowFull`：让 reviser 自行选择（混合或未知问题集）

use crate::features::story::{AuditIssue, AuditSeverity, RepairScope};

/// Auto 模式下 reviser 的输出路由。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoOutputMode {
    /// 只输出 PATCHES —— 阻塞问题都是局部的
    PatchOnly,
    /// 只输出 REVISED_CONTENT —— 阻塞问题含结构问题
    RewriteOnly,
    /// 让 reviser 自行选择（混合或未知问题集）
    AllowFull,
}

/// 局部问题模式：措辞、段落形状、疲劳词、信息越界、知识污染等。
///
/// 这类问题用 PATCHES 修复更安全 —— 整章重写有引入新问题的风险。
/// 匹配方式：在 `category + " " + description` 上做大小写不敏感的子串匹配。
const LOCAL_ONLY_KEYWORDS: &[&str] = &[
    "paragraph uniformity", "段落等长",
    "hedge density", "套话密度",
    "formulaic transitions", "公式化转折",
    "list-like structure", "列表式结构",
    "cross-chapter repetition", "跨章重复",
    "ai-tell word density",
    "fatigue word", "高疲劳词",
    "information boundary check", "信息越界",
    "knowledge base pollution", "知识库污染",
];

/// 结构/语义问题模式：人设崩、主线偏、爽点缺、时间线错、伏笔未收、视角失败等。
///
/// 这类问题 PATCHES 无法修复 —— 必须整章重写。
const STRUCTURAL_KEYWORDS: &[&str] = &[
    "ooc", "人设", "character fidelity", "character matrix", "character consistency",
    "mainline drift", "主线偏离", "outline drift", "大纲偏离",
    "chapter memo drift", "章节备忘偏离",
    "conflict", "冲突乏力", "payoff dilution", "爽点虚化",
    "timeline", "时间线",
    "hook check", "伏笔检查", "hook debt", "伏笔债", "未兑现",
    "power scaling", "战力崩坏", "金手指",
    "pacing", "节奏",
    "pov consistency", "视角",
    "subplot stagnation", "支线停滞", "arc flatline", "弧线平坦",
    "relationship dynamics", "关系动态", "情感表达",
    "incentive chain", "利益链",
    "canon event", "正典", "mainline canon",
];

/// 根据 audit issues 决定 auto 模式的输出路由。
///
/// 路由优先级（对齐 inkos `resolveAutoOutputMode`）：
/// 1. 无 issues → `AllowFull`（让 reviser 自由发挥）
/// 2. 有 `repair_scope` 标注的阻塞问题：
///    - 任一为 `Structural` → `RewriteOnly`
///    - 全部为 `Local` → `PatchOnly`
/// 3. 无 `repair_scope` 标注时，用 category 文本模式匹配做 fallback：
///    - 任一匹配结构模式 → `RewriteOnly`
///    - 全部匹配局部模式 → `PatchOnly`
///    - 混合或未知 → `AllowFull`
pub fn resolve_auto_output_mode(issues: &[AuditIssue]) -> AutoOutputMode {
    if issues.is_empty() {
        return AutoOutputMode::AllowFull;
    }

    // 优先使用 typed repair_scope
    let scoped_blocking: Vec<&AuditIssue> = issues.iter()
        .filter(|i| i.severity != AuditSeverity::Info && i.repair_scope.is_some())
        .collect();

    if !scoped_blocking.is_empty() {
        if scoped_blocking.iter().any(|i| i.repair_scope == Some(RepairScope::Structural)) {
            return AutoOutputMode::RewriteOnly;
        }
        let blocking_count = issues.iter().filter(|i| i.severity != AuditSeverity::Info).count();
        if scoped_blocking.len() == blocking_count
            && scoped_blocking.iter().all(|i| i.repair_scope == Some(RepairScope::Local))
        {
            return AutoOutputMode::PatchOnly;
        }
    }

    // Fallback：用 category + description 文本模式匹配
    let blocking: Vec<&AuditIssue> = issues.iter()
        .filter(|i| i.severity != AuditSeverity::Info)
        .collect();

    if blocking.is_empty() {
        // 只有 info 级问题 —— 至多需要局部抛光
        return AutoOutputMode::PatchOnly;
    }

    let structural_count = blocking.iter().filter(|i| is_structural(i)).count();
    let local_only_count = blocking.iter().filter(|i| is_local_only(i)).count();

    // 任一结构问题 → 必须整章重写
    if structural_count > 0 {
        return AutoOutputMode::RewriteOnly;
    }

    // 全部是局部问题 → 安全 patch
    if local_only_count == blocking.len() {
        return AutoOutputMode::PatchOnly;
    }

    // 混合或未知 —— 让 reviser 自行选择
    AutoOutputMode::AllowFull
}

fn is_structural(issue: &AuditIssue) -> bool {
    let text = format!("{} {}", issue.category, issue.description).to_lowercase();
    STRUCTURAL_KEYWORDS.iter().any(|kw| text.contains(kw))
}

fn is_local_only(issue: &AuditIssue) -> bool {
    let text = format!("{} {}", issue.category, issue.description).to_lowercase();
    LOCAL_ONLY_KEYWORDS.iter().any(|kw| text.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::story::AuditIssue;

    fn make_issue(severity: AuditSeverity, category: &str, description: &str, scope: Option<RepairScope>) -> AuditIssue {
        AuditIssue {
            severity,
            category: category.to_string(),
            description: description.to_string(),
            suggestion: String::new(),
            repair_scope: scope,
        }
    }

    // ── 空 issues ──────────────────────────────────────────────

    #[test]
    fn test_empty_issues_returns_allow_full() {
        assert_eq!(resolve_auto_output_mode(&[]), AutoOutputMode::AllowFull);
    }

    // ── typed repair_scope ────────────────────────────────────

    #[test]
    fn test_scoped_structural_returns_rewrite_only() {
        let issues = vec![
            make_issue(AuditSeverity::Critical, "OOC", "x", Some(RepairScope::Structural)),
            make_issue(AuditSeverity::Warning, "措辞", "y", Some(RepairScope::Local)),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::RewriteOnly);
    }

    #[test]
    fn test_scoped_all_local_returns_patch_only() {
        let issues = vec![
            make_issue(AuditSeverity::Warning, "措辞", "x", Some(RepairScope::Local)),
            make_issue(AuditSeverity::Warning, "套话", "y", Some(RepairScope::Local)),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }

    #[test]
    fn test_scoped_mixed_local_and_unknown_returns_allow_full() {
        let issues = vec![
            make_issue(AuditSeverity::Warning, "措辞", "x", Some(RepairScope::Local)),
            make_issue(AuditSeverity::Critical, "未知", "y", Some(RepairScope::Unknown)),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::AllowFull);
    }

    #[test]
    fn test_scoped_info_ignored_in_routing() {
        // info 级问题不参与路由决策
        let issues = vec![
            make_issue(AuditSeverity::Info, "建议", "x", Some(RepairScope::Structural)),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }

    // ── fallback: 文本模式匹配 ────────────────────────────────

    #[test]
    fn test_fallback_structural_keyword_ooc() {
        let issues = vec![
            make_issue(AuditSeverity::Critical, "OOC检查", "人设崩坏", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::RewriteOnly);
    }

    #[test]
    fn test_fallback_structural_keyword_timeline() {
        let issues = vec![
            make_issue(AuditSeverity::Critical, "时间线检查", "时间线断裂", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::RewriteOnly);
    }

    #[test]
    fn test_fallback_local_keyword_paragraph_uniformity() {
        let issues = vec![
            make_issue(AuditSeverity::Warning, "段落等长", "段落长度过于均匀", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }

    #[test]
    fn test_fallback_local_keyword_fatigue_word() {
        let issues = vec![
            make_issue(AuditSeverity::Warning, "词汇疲劳", "高疲劳词：仿佛", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }

    #[test]
    fn test_fallback_mixed_structural_and_local_returns_rewrite_only() {
        // 任一结构问题就强制 rewrite
        let issues = vec![
            make_issue(AuditSeverity::Warning, "段落等长", "均匀", None),
            make_issue(AuditSeverity::Critical, "OOC检查", "人设崩", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::RewriteOnly);
    }

    #[test]
    fn test_fallback_all_local_returns_patch_only() {
        let issues = vec![
            make_issue(AuditSeverity::Warning, "段落等长", "均匀", None),
            make_issue(AuditSeverity::Warning, "套话密度", "套话多", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }

    #[test]
    fn test_fallback_unknown_category_returns_allow_full() {
        let issues = vec![
            make_issue(AuditSeverity::Warning, "未知维度", "未知问题", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::AllowFull);
    }

    #[test]
    fn test_fallback_only_info_returns_patch_only() {
        let issues = vec![
            make_issue(AuditSeverity::Info, "建议", "仅供参考", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }

    // ── 大小写不敏感 ──────────────────────────────────────────

    #[test]
    fn test_fallback_case_insensitive() {
        let issues = vec![
            make_issue(AuditSeverity::Critical, "POV Consistency", "视角失败", None),
        ];
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::RewriteOnly);
    }

    // ── 混合 typed + fallback ─────────────────────────────────

    #[test]
    fn test_typed_scope_overrides_fallback() {
        // repair_scope=Local 即使 category 是 OOC 也走 PatchOnly（typed 优先）
        let issues = vec![
            make_issue(AuditSeverity::Critical, "OOC", "x", Some(RepairScope::Local)),
        ];
        // typed: scoped_blocking=1, all local → PatchOnly
        assert_eq!(resolve_auto_output_mode(&issues), AutoOutputMode::PatchOnly);
    }
}
