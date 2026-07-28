//! ═══════════════════════════════════════════════════════════════════════════
//! CuratorState - 后台编排器状态机
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 纯函数，无 I/O 依赖，便于单元测试。
//!
//! 设计说明：
//! 1. `now` 使用 `DateTime<Utc>` 而非 `Instant`：curator 的状态转换基于 wall-clock 日期
//! 2. 入参使用 `SkillReviewItem` 而非 `application::skill::Skill`：
//!    `SkillReviewItem` 封装 curator 审查所需的最小字段集
//! 3. 阈值通过 `TransitionConfig` 参数传入，避免隐藏全局状态

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 技能状态（curator 视角）。
///
/// 注意：`application::skill::evolution::SkillState` 是同构枚举，但属于 application 层，
/// `core/` 不反向依赖该层，故 curator 模块自带本枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillState {
    /// 活跃：近期被使用
    Active,
    /// 陈旧：超过 stale 阈值未被使用
    Stale,
    /// 归档：超过 archive 阈值，不再自动恢复
    Archived,
}

/// Curator 审查条目 —— 状态转换 + umbrella 检测所需的最小字段集。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReviewItem {
    /// 技能名称
    pub name: String,
    /// 分类标签
    pub category: String,
    /// 技能描述
    pub description: String,
    /// 当前状态
    pub state: SkillState,
    /// 最后使用时间（wall-clock）
    pub last_used_at: DateTime<Utc>,
    /// 是否固定（固定技能不参与自动状态转换）
    pub pinned: bool,
}

/// 状态转换阈值配置。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TransitionConfig {
    /// Active → Stale 的天数阈值（对照 hermes-agent DEFAULT_STALE_AFTER_DAYS = 30）
    pub stale_after_days: u64,
    /// Stale → Archived 的天数阈值（对照 hermes-agent DEFAULT_ARCHIVE_AFTER_DAYS = 90）
    pub archive_after_days: u64,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            stale_after_days: 30,
            archive_after_days: 90,
        }
    }
}

/// Umbrella 合并模式（对照 hermes-agent CURATOR_REVIEW_PROMPT §3 三种合并方式）。
///
/// LLM 在审查每个相似 skills 集群时选择其中一种：
/// - `MergeIntoExisting`：集群中已存在足够宽泛的 umbrella，patch 它并归档 siblings
/// - `CreateNewUmbrella`：无现成 umbrella，新建一个 class-level SKILL.md 并归档 siblings
/// - `DemoteToReferences`：sibling 有窄但有价值的内容，移动到 umbrella 的
///   `references/` / `templates/` / `scripts/` 子目录后归档原 skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UmbrellaMode {
    /// 合并进现有 umbrella skill（patch + archive siblings）
    MergeIntoExisting,
    /// 创建新 umbrella SKILL.md（create + archive siblings）
    CreateNewUmbrella,
    /// 降级为 references/templates/scripts 支持文件后归档
    DemoteToReferences,
}

impl UmbrellaMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MergeIntoExisting => "merge_into_existing",
            Self::CreateNewUmbrella => "create_new_umbrella",
            Self::DemoteToReferences => "demote_to_references",
        }
    }
}

/// 单个相似集群的合并结果（对照 hermes-agent `_classify_removed_skills` 的输出）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    /// LLM 选择的合并模式
    pub mode: UmbrellaMode,
    /// 目标 umbrella skill 名称（MergeIntoExisting 为已有 / CreateNewUmbrella 为新建 /
    /// DemoteToReferences 为目标 umbrella）；None 表示 LLM 未指明
    pub umbrella_name: Option<String>,
    /// LLM 返回的合并产物 JSON（merged skill / new SKILL.md / demoted refs 清单）
    pub merged_content: String,
}

/// 应用自动状态转换（纯函数）。
///
/// 转换规则：
/// - `Active → Stale`：`last_used_at` 距 `now` 超过 `stale_after_days`
/// - `Stale → Archived`：`last_used_at` 距 `now` 超过 `archive_after_days`
/// - `Archived` 不自动恢复（需人工重新激活）
/// - `pinned = true` 的技能跳过转换
///
/// 返回新 Vec，输入不被修改（值语义，便于测试断言）。
pub fn apply_automatic_transitions(
    items: Vec<SkillReviewItem>,
    now: DateTime<Utc>,
    config: &TransitionConfig,
) -> Vec<SkillReviewItem> {
    items
        .into_iter()
        .map(|mut item| {
            // 固定技能不参与转换
            if item.pinned {
                return item;
            }
            let age_days = (now - item.last_used_at).num_days().max(0) as u64;
            match item.state {
                SkillState::Active => {
                    if age_days > config.stale_after_days {
                        item.state = SkillState::Stale;
                    }
                }
                SkillState::Stale => {
                    if age_days > config.archive_after_days {
                        item.state = SkillState::Archived;
                    }
                }
                SkillState::Archived => {
                    // 归档状态不自动恢复
                }
            }
            item
        })
        .collect()
}

/// 检测相似 skills 分组（纯函数）。
///
/// 相似判定：
/// 1. 相同 `category`（大小写不敏感精确匹配）
/// 2. 名称"相似"：分词后共享至少一个长度 >= 3 的 token，或互为子串
///
/// 返回 `Vec<Vec<usize>>`，每个内层 Vec 为输入切片中的索引，仅包含 size >= 2 的分组。
/// 使用 union-find 处理传递性相似（A~B, B~C ⇒ A,B,C 同组）。
pub fn detect_similar_groups(items: &[SkillReviewItem]) -> Vec<Vec<usize>> {
    let n = items.len();
    if n < 2 {
        return Vec::new();
    }
    let mut parent: Vec<usize> = (0..n).collect();
    let mut find = |p: &mut Vec<usize>, x: usize| -> usize {
        let mut root = x;
        while p[root] != root {
            root = p[root];
        }
        // 路径压缩
        let mut cur = x;
        while p[cur] != root {
            let next = p[cur];
            p[cur] = root;
            cur = next;
        }
        root
    };
    let union = |p: &mut Vec<usize>, find: &mut dyn FnMut(&mut Vec<usize>, usize) -> usize,
                 a: usize, b: usize| {
        let ra = find(p, a);
        let rb = find(p, b);
        if ra != rb {
            p[ra] = rb;
        }
    };

    // 预计算每个 item 的 name tokens（小写、长度 >= 3）
    let name_tokens: Vec<Vec<String>> = items
        .iter()
        .map(|it| {
            it.name
                .split(|c: char| c.is_whitespace() || c == '_' || c == '-')
                .map(|t| t.to_lowercase())
                .filter(|t| t.chars().count() >= 3)
                .collect()
        })
        .collect();
    let name_lower: Vec<String> = items.iter().map(|it| it.name.to_lowercase()).collect();

    for i in 0..n {
        for j in (i + 1)..n {
            // 1. category 匹配
            if !items[i].category.eq_ignore_ascii_case(&items[j].category) {
                continue;
            }
            // 2. name 相似
            let similar = shares_token(&name_tokens[i], &name_tokens[j])
                || name_lower[i].contains(&name_lower[j])
                || name_lower[j].contains(&name_lower[i]);
            if similar {
                union(&mut parent, &mut find, i, j);
            }
        }
    }

    // 收集分组
    let mut groups: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }
    // 只保留 size >= 2 的分组，按组内首个索引排序以保持确定性
    let mut result: Vec<Vec<usize>> = groups
        .into_values()
        .filter(|g| g.len() >= 2)
        .map(|mut g| {
            g.sort_unstable();
            g
        })
        .collect();
    result.sort_by_key(|g| g.first().copied().unwrap_or(usize::MAX));
    result
}

/// 两个 token 列表是否共享至少一个 token。
fn shares_token(a: &[String], b: &[String]) -> bool {
    a.iter().any(|ta| b.iter().any(|tb| ta == tb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn item(name: &str, state: SkillState, days_ago: i64) -> SkillReviewItem {
        SkillReviewItem {
            name: name.to_string(),
            category: "general".to_string(),
            description: String::new(),
            state,
            last_used_at: Utc::now() - chrono::Duration::days(days_ago),
            pinned: false,
        }
    }

    fn config() -> TransitionConfig {
        TransitionConfig::default() // 30 / 90
    }

    #[test]
    fn active_to_stale_at_30_days() {
        // 超过 30 天未使用 → Stale（边界：31 天触发）
        let now = Utc::now();
        let items = vec![item("skill-a", SkillState::Active, 31)];
        let out = apply_automatic_transitions(items, now, &config());
        assert_eq!(out[0].state, SkillState::Stale);
    }

    #[test]
    fn active_stays_active_within_threshold() {
        // 边界：刚好 30 天不触发（用 > 而非 >=）
        let now = Utc::now();
        let items = vec![item("skill-a", SkillState::Active, 30)];
        let out = apply_automatic_transitions(items, now, &config());
        assert_eq!(out[0].state, SkillState::Active);
    }

    #[test]
    fn stale_to_archived_at_90_days() {
        // 超过 90 天未使用 → Archived（边界：91 天触发）
        let now = Utc::now();
        let items = vec![item("skill-b", SkillState::Stale, 91)];
        let out = apply_automatic_transitions(items, now, &config());
        assert_eq!(out[0].state, SkillState::Archived);
    }

    #[test]
    fn stale_stays_stale_within_threshold() {
        let now = Utc::now();
        let items = vec![item("skill-b", SkillState::Stale, 90)];
        let out = apply_automatic_transitions(items, now, &config());
        assert_eq!(out[0].state, SkillState::Stale);
    }

    #[test]
    fn archived_never_auto_recovers() {
        // Archived 即使过很久也不变
        let now = Utc::now();
        let items = vec![item("skill-c", SkillState::Archived, 365)];
        let out = apply_automatic_transitions(items, now, &config());
        assert_eq!(out[0].state, SkillState::Archived);
    }

    #[test]
    fn pinned_skills_skip_transitions() {
        // pinned 即使超过阈值也不转换
        let now = Utc::now();
        let mut pinned = item("pinned", SkillState::Active, 100);
        pinned.pinned = true;
        let out = apply_automatic_transitions(vec![pinned], now, &config());
        assert_eq!(out[0].state, SkillState::Active);
        assert!(out[0].pinned);
    }

    #[test]
    fn input_item_clone_not_affected() {
        let now = Utc::now();
        let src = item("skill-a", SkillState::Active, 100);
        let cloned = src.clone();
        let _out = apply_automatic_transitions(vec![cloned], now, &config());
        // 原始 src.state 不变
        assert_eq!(src.state, SkillState::Active);
    }

    #[test]
    fn multiple_items_mixed_transitions() {
        let now = Utc::now();
        let items = vec![
            item("recent-active", SkillState::Active, 1),   // stays Active
            item("old-active", SkillState::Active, 45),     // → Stale (>30)
            item("old-stale", SkillState::Stale, 120),      // → Archived (>90)
            item("archived", SkillState::Archived, 365),    // stays Archived
        ];
        let out = apply_automatic_transitions(items, now, &config());
        assert_eq!(out[0].state, SkillState::Active);
        assert_eq!(out[1].state, SkillState::Stale);
        assert_eq!(out[2].state, SkillState::Archived);
        assert_eq!(out[3].state, SkillState::Archived);
    }

    #[test]
    fn detect_similar_by_shared_token() {
        // 相同 category + 共享 token "writing"
        let items = vec![
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "essay_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "data_analysis".to_string(),
                category: "analysis".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let groups = detect_similar_groups(&items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![0, 1]);
    }

    #[test]
    fn detect_similar_by_substring() {
        // 互为子串
        let items = vec![
            SkillReviewItem {
                name: "writer".to_string(),
                category: "core".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "writer-pro".to_string(),
                category: "core".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let groups = detect_similar_groups(&items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![0, 1]);
    }

    #[test]
    fn detect_similar_different_category_no_group() {
        let items = vec![
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "analysis".to_string(), // 不同 category
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let groups = detect_similar_groups(&items);
        assert!(groups.is_empty());
    }

    #[test]
    fn detect_similar_transitive_grouping() {
        // A~B (共享 token), B~C (子串) ⇒ A,B,C 同组
        let items = vec![
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "essay_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "essay".to_string(), // 与 "essay_writing" 子串相似
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let groups = detect_similar_groups(&items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![0, 1, 2]);
    }

    #[test]
    fn detect_similar_empty_input() {
        let groups = detect_similar_groups(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn detect_similar_single_item() {
        let now = Utc::now();
        let items = vec![item("solo", SkillState::Active, 0)];
        let groups = detect_similar_groups(&items);
        assert!(groups.is_empty());
    }

    #[test]
    fn transition_config_default_values() {
        let c = TransitionConfig::default();
        assert_eq!(c.stale_after_days, 30);
        assert_eq!(c.archive_after_days, 90);
    }

    #[test]
    fn umbrella_mode_as_str() {
        assert_eq!(UmbrellaMode::MergeIntoExisting.as_str(), "merge_into_existing");
        assert_eq!(UmbrellaMode::CreateNewUmbrella.as_str(), "create_new_umbrella");
        assert_eq!(UmbrellaMode::DemoteToReferences.as_str(), "demote_to_references");
    }

    #[test]
    fn umbrella_mode_serde_roundtrip() {
        for mode in [UmbrellaMode::MergeIntoExisting, UmbrellaMode::CreateNewUmbrella, UmbrellaMode::DemoteToReferences] {
            let s = serde_json::to_string(&mode).unwrap();
            let back: UmbrellaMode = serde_json::from_str(&s).unwrap();
            assert_eq!(mode, back);
        }
    }
}
