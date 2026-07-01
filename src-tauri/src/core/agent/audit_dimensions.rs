// S6.1: 37 维度动态审计维度定义与组合逻辑。
//
// 移植自 inkos `continuity.ts` 的 DIMENSION_LABELS + buildDimensionList +
// buildDimensionNote。与 inkos 的差异：
// - inkos 用 GenreProfile / BookRules 对象，Mnemosyne 用轻量 AuditDimensionContext
//   只携带维度组合所需的最小信息（genre 维度集 + 书级补充 + 番外/同人模式）
// - inkos 的 buildDimensionNote 有大量基于 GenreProfile 字段的动态内容，
//   Mnemosyne 移植了核心维度（1/6/7/10/15/19/24/25/26/28-37）的 note，
//   其余维度返回空 note（审计 prompt 中的维度名已足够指导 LLM）
// - FanficMode 简化为 Canon / Ooc / AU 三态（inkos 还有其他变体）

use crate::features::story::RepairScope;

/// 同人模式 —— 决定维度 34-37 的启用与严格度。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FanficMode {
    /// 原作向：角色必须严格遵守性格底色
    Canon,
    /// OOC 模式：角色可偏离性格底色，仅记录不判定失败
    Ooc,
    /// 平行宇宙：世界规则可重构，但需内部自洽
    AU,
}

/// 维度组合上下文 —— 携带 build_dimension_list 所需的最小信息。
///
/// 调用方（ContinuityAuditor）从 book_dir 的 story 配置文件中读取这些字段。
/// 缺失的字段用 Default（空 Vec / false / None）填充，对应维度不会被激活。
#[derive(Debug, Clone, Default)]
pub struct AuditDimensionContext {
    /// 题材 profile 激活的维度 ID 列表（对应 inkos GenreProfile.auditDimensions）
    pub genre_dimensions: Vec<u32>,
    /// 书级额外维度（对应 inkos BookRules.additionalAuditDimensions）
    pub additional_dimensions: Vec<u32>,
    /// 是否存在 parent_canon.md（激活番外维度 28-31）
    pub has_parent_canon: bool,
    /// 同人模式（激活同人维度 34-37，替换番外维度）
    pub fanfic_mode: Option<FanficMode>,
    /// 是否启用年代考据（激活维度 12）
    pub era_research: bool,
    /// 高疲劳词列表（维度 10 的 note 使用）
    pub fatigue_words: Vec<String>,
    /// 爽点类型列表（维度 15 的 note 使用）
    pub satisfaction_types: Vec<String>,
}

/// 一个审计维度的渲染信息。
#[derive(Debug, Clone)]
pub struct DimensionInfo {
    pub id: u32,
    pub name: String,
    /// 维度检查指导（空字符串表示用维度名即可，无需额外说明）
    pub note: String,
}

/// 37 个审计维度的双语标签。
///
/// ID 与 inkos `DIMENSION_LABELS` 完全对齐：
/// - 1-11: 基础结构维度（OOC/时间线/设定/战力/数值/伏笔/节奏/文风/信息越界/词汇疲劳/利益链）
/// - 12-18: 进阶结构维度（年代/配角降智/配角工具人化/爽点/台词/流水账/知识库污染）
/// - 19-27: 工程维度（视角/段落/套话/公式化/列表式/支线/弧线/节奏单调/敏感词）
/// - 28-31: 番外维度（正传冲突/未来信息/跨书规则/伏笔隔离）
/// - 32-33: 永久启用维度（读者期待/章节备忘偏离）
/// - 34-37: 同人维度（角色还原/世界规则/关系动态/正典事件）
pub static DIMENSION_LABELS: &[(u32, &str, &str)] = &[
    (1,  "OOC检查",          "OOC Check"),
    (2,  "时间线检查",        "Timeline Check"),
    (3,  "设定冲突",          "Lore Conflict Check"),
    (4,  "战力崩坏",          "Power Scaling Check"),
    (5,  "数值检查",          "Numerical Consistency Check"),
    (6,  "伏笔检查",          "Hook Check"),
    (7,  "节奏检查",          "Pacing Check"),
    (8,  "文风检查",          "Style Check"),
    (9,  "信息越界",          "Information Boundary Check"),
    (10, "词汇疲劳",          "Lexical Fatigue Check"),
    (11, "利益链断裂",        "Incentive Chain Check"),
    (12, "年代考据",          "Era Accuracy Check"),
    (13, "配角降智",          "Side Character Competence Check"),
    (14, "配角工具人化",      "Side Character Instrumentalization Check"),
    (15, "爽点虚化",          "Payoff Dilution Check"),
    (16, "台词失真",          "Dialogue Authenticity Check"),
    (17, "流水账",            "Chronicle Drift Check"),
    (18, "知识库污染",        "Knowledge Base Pollution Check"),
    (19, "视角一致性",        "POV Consistency Check"),
    (20, "段落等长",          "Paragraph Uniformity Check"),
    (21, "套话密度",          "Cliche Density Check"),
    (22, "公式化转折",        "Formulaic Twist Check"),
    (23, "列表式结构",        "List-like Structure Check"),
    (24, "支线停滞",          "Subplot Stagnation Check"),
    (25, "弧线平坦",          "Arc Flatline Check"),
    (26, "节奏单调",          "Pacing Monotony Check"),
    (27, "敏感词检查",        "Sensitive Content Check"),
    (28, "正传事件冲突",      "Mainline Canon Event Conflict"),
    (29, "未来信息泄露",      "Future Knowledge Leak Check"),
    (30, "世界规则跨书一致性", "Cross-Book World Rule Check"),
    (31, "番外伏笔隔离",      "Spinoff Hook Isolation Check"),
    (32, "读者期待管理",      "Reader Expectation Check"),
    (33, "章节备忘偏离",      "Chapter Memo Drift Check"),
    (34, "角色还原度",        "Character Fidelity Check"),
    (35, "世界规则遵守",      "World Rule Compliance Check"),
    (36, "关系动态",          "Relationship Dynamics Check"),
    (37, "正典事件一致性",    "Canon Event Consistency Check"),
];

/// 获取维度的双语标签。
pub fn dimension_name(id: u32, language: &str) -> Option<&'static str> {
    DIMENSION_LABELS.iter()
        .find(|(dim_id, _, _)| *dim_id == id)
        .map(|(_, zh, en)| if language == "en" { *en } else { *zh })
}

/// 动态构建审计维度列表。
///
/// 组合规则（对齐 inkos `buildDimensionList`）：
/// 1. 从 genre_dimensions + additional_dimensions 起步
/// 2. 永久启用 32（读者期待）+ 33（章节备忘偏离）
/// 3. era_research → 启用 12（年代考据）
/// 4. has_parent_canon 且非同人 → 启用 28-31（番外维度）
/// 5. fanfic_mode → 启用 34-37（同人维度）
/// 6. 排序后为每个维度生成 note
pub fn build_dimension_list(ctx: &AuditDimensionContext, language: &str) -> Vec<DimensionInfo> {
    let mut active: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();

    // 1. Genre + book-level dimensions
    for &id in &ctx.genre_dimensions {
        active.insert(id);
    }
    for &id in &ctx.additional_dimensions {
        active.insert(id);
    }

    // 2. Always-active dimensions
    active.insert(32);
    active.insert(33);

    // 3. Era research → dimension 12
    if ctx.era_research {
        active.insert(12);
    }

    // 4. Spinoff dimensions (when parent_canon exists and NOT in fanfic mode)
    if ctx.has_parent_canon && ctx.fanfic_mode.is_none() {
        active.insert(28);
        active.insert(29);
        active.insert(30);
        active.insert(31);
    }

    // 5. Fanfic dimensions (replace spinoff dims)
    if ctx.fanfic_mode.is_some() {
        active.insert(34);
        active.insert(35);
        active.insert(36);
        active.insert(37);
    }

    // 6. Build DimensionInfo with notes
    active.into_iter()
        .filter_map(|id| {
            let name = dimension_name(id, language)?.to_string();
            let note = build_dimension_note(id, language, ctx);
            Some(DimensionInfo { id, name, note })
        })
        .collect()
}

/// 为单个维度生成检查指导文本。
///
/// 移植自 inkos `buildDimensionNote`。只为核心维度生成动态 note；
/// 其余维度返回空字符串（维度名本身已足够指导 LLM）。
fn build_dimension_note(id: u32, language: &str, ctx: &AuditDimensionContext) -> String {
    let en = language == "en";

    match id {
        1 => {
            // OOC 检查 —— 同人模式下有特殊规则
            match ctx.fanfic_mode {
                Some(FanficMode::Ooc) => if en {
                    "In OOC mode, personality drift can be intentional; record only, do not fail.".to_string()
                } else {
                    "OOC模式下角色可偏离性格底色，此维度仅记录不判定失败。".to_string()
                },
                Some(FanficMode::Canon) => if en {
                    "Canon-faithful fanfic: characters must stay close to their original personality core.".to_string()
                } else {
                    "原作向同人：角色必须严格遵守性格底色。".to_string()
                },
                _ => String::new(),
            }
        }
        6 => if en {
            "Read the pending hooks ledger. Escalate promoted core hooks stale >10 chapters to critical. Blocked hooks (Y chapters >= 6) → warning. Quote hook_id verbatim.".to_string()
        } else {
            "阅读伏笔池：升级=是且 core_hook=是 的伏笔过期超过 10 章 → critical；受阻 Y≥6 章 → warning。description 中引用 hook_id。".to_string()
        },
        7 => if en {
            "Check pacing rhythm: Do the recent 3-5 chapters form a complete mini-goal cycle (build-up → escalation → climax → aftermath)? 5+ consecutive chapters without a climax → pacing stagnation.".to_string()
        } else {
            "检查节奏波形：最近 3-5 章是否形成完整的「蓄压→升级→爆发→后效」周期？连续 5 章无爆发 → 节奏停滞。".to_string()
        },
        10 => {
            // 词汇疲劳 —— 动态注入疲劳词列表
            if ctx.fatigue_words.is_empty() {
                String::new()
            } else if en {
                format!("Fatigue words: {}. Also check AI markers (仿佛/不禁/宛如/竟然/忽然/猛地); warn when any appears more than once per 3,000 words.", ctx.fatigue_words.join(", "))
            } else {
                format!("高疲劳词：{}。同时检查AI标记词（仿佛/不禁/宛如/竟然/忽然/猛地）密度，每3000字超过1次即warning", ctx.fatigue_words.join("、"))
            }
        }
        15 => {
            // 爽点虚化 —— 动态注入爽点类型
            if ctx.satisfaction_types.is_empty() {
                String::new()
            } else if en {
                format!("Payoff types: {}. A payoff that only satisfies 70% of built-up anticipation counts as diluted.", ctx.satisfaction_types.join(", "))
            } else {
                format!("爽点类型：{}。只满足读者70%期待的兑现等于爽点虚化。", ctx.satisfaction_types.join("、"))
            }
        }
        19 => if en {
            "Check whether POV shifts are signaled clearly and stay consistent with the configured viewpoint.".to_string()
        } else {
            "检查视角切换是否有过渡、是否与设定视角一致。".to_string()
        },
        24 => if en {
            "Cross-check subplot_board and chapter_summaries: flag dormant or only-restated subplots.".to_string()
        } else {
            "对照支线进度板和章节摘要：标记沉寂或仅重复提及的支线。".to_string()
        },
        25 => if en {
            "Cross-check emotional_arcs: flag characters whose emotional line holds one pressure shape without new pressure, release, or reversal.".to_string()
        } else {
            "对照情感弧线：标记角色情绪线停滞，没有新压力、释放或转折。".to_string()
        },
        26 => if en {
            "Cross-check chapter_summaries for chapter-type distribution: warn when recent chapters stay in the same mode too long.".to_string()
        } else {
            "对照章节摘要的章节类型分布：近期章节长时间停留同种模式时 warning。".to_string()
        },
        28 => if en {
            "Check whether spinoff events contradict the mainline canon constraints.".to_string()
        } else {
            "检查番外事件是否与正典约束表矛盾。".to_string()
        },
        29 => if en {
            "Check whether characters reference information that should only be revealed after the divergence point.".to_string()
        } else {
            "检查角色是否引用了分歧点之后才揭示的信息。".to_string()
        },
        30 => if en {
            "Check whether the spinoff violates mainline world rules (power system, geography, factions).".to_string()
        } else {
            "检查番外是否违反正传世界规则（力量体系、地理、阵营）。".to_string()
        },
        31 => if en {
            "Check whether the spinoff resolves mainline hooks without authorization (warning level).".to_string()
        } else {
            "检查番外是否越权回收正传伏笔（warning级别）。".to_string()
        },
        32 => if en {
            "Check whether the ending renews curiosity, promised payoffs are landing, pressure gets release, and expectation gaps are being satisfied.".to_string()
        } else {
            "检查：章尾是否重新点燃好奇心，承诺的回收是否按节奏落地，压力是否释放，期待缺口在被满足还是累积。".to_string()
        },
        33 => if en {
            "Cross-check chapter_memo. Does the prose deliver the memo's goal? Missing or contradicted sections → critical. Sparse memo is legitimate — only flag drift against populated sections.".to_string()
        } else {
            "对照章节备忘。成稿是否兑现 memo 的 goal？段落缺失或写反 → critical。稀疏 memo 合法，只检查 memo 实际写出的段落。".to_string()
        },
        34 => if en {
            "Check dialogue tics, speaking style, and behavior against character dossiers. Deviations need clear situational motivation.".to_string()
        } else {
            "检查对话癖、说话风格和行为是否符合角色档案。偏离需有明确的情境动机。".to_string()
        },
        35 => if en {
            "Check whether the chapter violates world rules (geography, power system, faction relations).".to_string()
        } else {
            "检查章节是否违反世界规则（地理、力量体系、阵营关系）。".to_string()
        },
        36 => if en {
            "Check whether relationship beats remain plausible and aligned with key relationships.".to_string()
        } else {
            "检查关系节拍是否合理且与关键关系一致或有意义的发展。".to_string()
        },
        37 => if en {
            "Check whether the chapter contradicts the key event timeline.".to_string()
        } else {
            "检查章节是否与关键事件时间线矛盾。".to_string()
        },
        _ => String::new(),
    }
}

/// 把 AuditIssue 的 repair_scope 字符串解析为 RepairScope 枚举。
///
/// 用于审计输出 JSON 解析时把 `"local"` / `"structural"` / `"unknown"` 字符串
/// 转换为强类型。无效值返回 None（等价于 "unknown" 但让 reviser 自行判断）。
pub fn parse_repair_scope(value: &str) -> Option<RepairScope> {
    match value.trim().to_lowercase().as_str() {
        "local" => Some(RepairScope::Local),
        "structural" => Some(RepairScope::Structural),
        "unknown" => Some(RepairScope::Unknown),
        _ => None,
    }
}

/// 把维度列表渲染为 prompt 文本。
///
/// 格式：`ID. 维度名（note）` 或 `ID. 维度名`（note 为空时）。
pub fn render_dimension_list(dimensions: &[DimensionInfo], language: &str) -> String {
    dimensions.iter().map(|d| {
        if d.note.is_empty() {
            format!("{}. {}", d.id, d.name)
        } else if language == "en" {
            format!("{}. {} ({})", d.id, d.name, d.note)
        } else {
            format!("{}. {}（{}）", d.id, d.name, d.note)
        }
    }).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_labels_count() {
        assert_eq!(DIMENSION_LABELS.len(), 37, "must have exactly 37 dimensions");
    }

    #[test]
    fn test_dimension_labels_ids_sequential() {
        for (i, (id, _, _)) in DIMENSION_LABELS.iter().enumerate() {
            assert_eq!(*id, (i + 1) as u32, "dimension IDs must be 1-37 sequential");
        }
    }

    #[test]
    fn test_dimension_name_zh() {
        assert_eq!(dimension_name(1, "zh"), Some("OOC检查"));
        assert_eq!(dimension_name(37, "zh"), Some("正典事件一致性"));
    }

    #[test]
    fn test_dimension_name_en() {
        assert_eq!(dimension_name(1, "en"), Some("OOC Check"));
        assert_eq!(dimension_name(37, "en"), Some("Canon Event Consistency Check"));
    }

    #[test]
    fn test_dimension_name_invalid_id() {
        assert_eq!(dimension_name(0, "zh"), None);
        assert_eq!(dimension_name(38, "en"), None);
    }

    #[test]
    fn test_build_dimension_list_always_active_32_33() {
        let ctx = AuditDimensionContext::default();
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert!(ids.contains(&32), "dimension 32 must always be active");
        assert!(ids.contains(&33), "dimension 33 must always be active");
        assert_eq!(dims.len(), 2, "empty context should only have 32 + 33");
    }

    #[test]
    fn test_build_dimension_list_genre_dimensions() {
        let ctx = AuditDimensionContext {
            genre_dimensions: vec![1, 2, 3, 7, 15],
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert_eq!(ids, vec![1, 2, 3, 7, 15, 32, 33]);
    }

    #[test]
    fn test_build_dimension_list_era_research() {
        let ctx = AuditDimensionContext {
            era_research: true,
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert!(ids.contains(&12), "era_research should activate dimension 12");
    }

    #[test]
    fn test_build_dimension_list_parent_canon() {
        let ctx = AuditDimensionContext {
            has_parent_canon: true,
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert!(ids.contains(&28), "parent_canon should activate spinoff dims 28-31");
        assert!(ids.contains(&29));
        assert!(ids.contains(&30));
        assert!(ids.contains(&31));
        assert!(!ids.contains(&34), "parent_canon without fanfic should NOT activate fanfic dims");
    }

    #[test]
    fn test_build_dimension_list_fanfic_mode() {
        let ctx = AuditDimensionContext {
            fanfic_mode: Some(FanficMode::Canon),
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert!(ids.contains(&34), "fanfic mode should activate fanfic dims 34-37");
        assert!(ids.contains(&35));
        assert!(ids.contains(&36));
        assert!(ids.contains(&37));
        assert!(!ids.contains(&28), "fanfic mode should NOT activate spinoff dims");
    }

    #[test]
    fn test_build_dimension_list_parent_canon_with_fanfic_excludes_spinoff() {
        let ctx = AuditDimensionContext {
            has_parent_canon: true,
            fanfic_mode: Some(FanficMode::AU),
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert!(!ids.contains(&28), "fanfic mode overrides parent_canon for spinoff dims");
        assert!(ids.contains(&34));
    }

    #[test]
    fn test_build_dimension_list_additional_dimensions() {
        let ctx = AuditDimensionContext {
            additional_dimensions: vec![13, 14, 27],
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert!(ids.contains(&13));
        assert!(ids.contains(&14));
        assert!(ids.contains(&27));
    }

    #[test]
    fn test_build_dimension_list_deduplicates() {
        let ctx = AuditDimensionContext {
            genre_dimensions: vec![1, 2, 32],
            additional_dimensions: vec![2, 33],
            ..Default::default()
        };
        let dims = build_dimension_list(&ctx, "zh");
        let ids: Vec<u32> = dims.iter().map(|d| d.id).collect();
        assert_eq!(ids, vec![1, 2, 32, 33], "duplicates should be removed");
    }

    #[test]
    fn test_build_dimension_note_ooc_fanfic_ooc() {
        let ctx = AuditDimensionContext {
            fanfic_mode: Some(FanficMode::Ooc),
            ..Default::default()
        };
        let note = build_dimension_note(1, "zh", &ctx);
        assert!(note.contains("仅记录不判定失败"));
    }

    #[test]
    fn test_build_dimension_note_ooc_fanfic_canon() {
        let ctx = AuditDimensionContext {
            fanfic_mode: Some(FanficMode::Canon),
            ..Default::default()
        };
        let note = build_dimension_note(1, "zh", &ctx);
        assert!(note.contains("严格遵守性格底色"));
    }

    #[test]
    fn test_build_dimension_note_fatigue_words() {
        let ctx = AuditDimensionContext {
            fatigue_words: vec!["果然".to_string(), "似乎".to_string()],
            ..Default::default()
        };
        let note = build_dimension_note(10, "zh", &ctx);
        assert!(note.contains("果然"));
        assert!(note.contains("似乎"));
        assert!(note.contains("AI标记词"));
    }

    #[test]
    fn test_build_dimension_note_fatigue_words_empty() {
        let ctx = AuditDimensionContext::default();
        let note = build_dimension_note(10, "zh", &ctx);
        assert!(note.is_empty(), "empty fatigue words should produce empty note");
    }

    #[test]
    fn test_build_dimension_note_satisfaction_types() {
        let ctx = AuditDimensionContext {
            satisfaction_types: vec!["打脸".to_string(), "逆袭".to_string()],
            ..Default::default()
        };
        let note = build_dimension_note(15, "zh", &ctx);
        assert!(note.contains("打脸"));
        assert!(note.contains("逆袭"));
    }

    #[test]
    fn test_build_dimension_note_empty_for_no_special_dim() {
        let ctx = AuditDimensionContext::default();
        let note = build_dimension_note(2, "zh", &ctx);
        assert!(note.is_empty(), "dimension 2 has no special note");
    }

    #[test]
    fn test_build_dimension_note_en() {
        let ctx = AuditDimensionContext {
            fanfic_mode: Some(FanficMode::Ooc),
            ..Default::default()
        };
        let note = build_dimension_note(1, "en", &ctx);
        assert!(note.contains("record only"));
    }

    #[test]
    fn test_parse_repair_scope() {
        assert_eq!(parse_repair_scope("local"), Some(RepairScope::Local));
        assert_eq!(parse_repair_scope("structural"), Some(RepairScope::Structural));
        assert_eq!(parse_repair_scope("unknown"), Some(RepairScope::Unknown));
        assert_eq!(parse_repair_scope("  Local  "), Some(RepairScope::Local));
        assert_eq!(parse_repair_scope("invalid"), None);
        assert_eq!(parse_repair_scope(""), None);
    }

    #[test]
    fn test_render_dimension_list_with_notes() {
        let dims = vec![
            DimensionInfo { id: 1, name: "OOC检查".to_string(), note: "测试note".to_string() },
            DimensionInfo { id: 2, name: "时间线检查".to_string(), note: String::new() },
        ];
        let rendered = render_dimension_list(&dims, "zh");
        assert!(rendered.contains("1. OOC检查（测试note）"));
        assert!(rendered.contains("2. 时间线检查"));
    }

    #[test]
    fn test_render_dimension_list_en() {
        let dims = vec![
            DimensionInfo { id: 1, name: "OOC Check".to_string(), note: "test note".to_string() },
        ];
        let rendered = render_dimension_list(&dims, "en");
        assert!(rendered.contains("1. OOC Check (test note)"));
    }
}
