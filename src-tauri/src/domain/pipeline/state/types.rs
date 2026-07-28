//! ═══════════════════════════════════════════════════════════════════════════
//! RuntimeState Types - 状态类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 这是双轨制状态管理的结构化层（JSON 加速索引）。
//! markdown 真相文件（current_state.md/pending_hooks.md/chapter_summaries.md）是权威源，
//! JSON 是从 markdown 引导出的加速索引，由 state-bootstrap 建立、state-reducer 更新、state-projections 反向渲染。

use serde::{Deserialize, Serialize};

use super::super::types::Language;

// ── Manifest ────────────────────────────────────────────────────────────────

/// 状态清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManifest {
    pub schema_version: u32,
    pub language: Language,
    pub last_applied_chapter: u32,
    pub projection_version: u32,
    #[serde(default)]
    pub migration_warnings: Vec<String>,
}

impl StateManifest {
    pub fn new(language: Language) -> Self {
        Self {
            schema_version: 2,
            language,
            last_applied_chapter: 0,
            projection_version: 1,
            migration_warnings: Vec::new(),
        }
    }
}

// ── Hooks ────────────────────────────────────────────────────

/// 钩子状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookStatus {
    Open,
    Progressing,
    Deferred,
    Resolved,
}

/// 钩子兑现时机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookPayoffTiming {
    Immediate,
    NearTerm,
    MidArc,
    SlowBurn,
    Endgame,
}

impl HookPayoffTiming {
    /// 返回 kebab-case 字符串表示（与 serde 序列化一致）。
    ///
    /// 用于替代 `{:?}` Debug 格式化，避免 `Some(MidArc)` 这样的输出污染文本比较。
    pub fn as_str(&self) -> &'static str {
        match self {
            HookPayoffTiming::Immediate => "immediate",
            HookPayoffTiming::NearTerm => "near-term",
            HookPayoffTiming::MidArc => "mid-arc",
            HookPayoffTiming::SlowBurn => "slow-burn",
            HookPayoffTiming::Endgame => "endgame",
        }
    }
}

/// 钩子记录
/// Phase 7 promotion 字段为 optional，兼容 pre-Phase-7 markdown 解析。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRecord {
    pub hook_id: String,
    pub start_chapter: u32,
    pub r#type: String,
    pub status: HookStatus,
    pub last_advanced_chapter: u32,
    #[serde(default)]
    pub expected_payoff: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payoff_timing: Option<HookPayoffTiming>,
    #[serde(default)]
    pub notes: String,
    // Phase 7 — hook causality / promotion metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pays_off_in_arc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_hook: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half_life_chapters: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted: Option<bool>,
}

/// 钩子状态集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksState {
    #[serde(default)]
    pub hooks: Vec<HookRecord>,
}

// ── Chapter Summaries ────────────────────────────────────────

/// 章节摘要行
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChapterSummaryRow {
    pub chapter: u32,
    pub title: String,
    #[serde(default)]
    pub characters: String,
    #[serde(default)]
    pub events: String,
    #[serde(default)]
    pub state_changes: String,
    #[serde(default)]
    pub hook_activity: String,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub chapter_type: String,
}

/// 章节摘要集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChapterSummariesState {
    #[serde(default)]
    pub rows: Vec<ChapterSummaryRow>,
}

// ── Current State ────────────────────────────────────────────

/// 当前状态事实
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStateFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from_chapter: u32,
    pub valid_until_chapter: Option<u32>,
    pub source_chapter: u32,
}

/// 当前状态集合
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CurrentStateState {
    pub chapter: u32,
    #[serde(default)]
    pub facts: Vec<CurrentStateFact>,
}


/// 当前状态补丁
/// settler 输出的增量补丁，应用到 currentState。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentStatePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protagonist_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_goal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_constraint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_alliances: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_conflict: Option<String>,
}

// ── Delta（settler 输出） ────────────────────────────────────

/// 钩子操作
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookOps {
    #[serde(default)]
    pub upsert: Vec<HookRecord>,
    #[serde(default)]
    pub mention: Vec<String>,
    #[serde(default)]
    pub resolve: Vec<String>,
    #[serde(default)]
    pub defer: Vec<String>,
}

/// 新钩子候选
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewHookCandidate {
    pub r#type: String,
    #[serde(default)]
    pub expected_payoff: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payoff_timing: Option<HookPayoffTiming>,
    #[serde(default)]
    pub notes: String,
}

/// 松散操作（键值对结构，对应 z.record(z.string(), z.unknown())）
/// 用于 subplotOps/emotionalArcOps/characterMatrixOps，保持灵活结构。
pub type LooseOp = serde_json::Value;

/// 运行时状态增量
/// settler agent 解析写手正文后输出，由 reducer 应用到 snapshot。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStateDelta {
    pub chapter: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_state_patch: Option<CurrentStatePatch>,
    #[serde(default)]
    pub hook_ops: HookOps,
    #[serde(default)]
    pub new_hook_candidates: Vec<NewHookCandidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_summary: Option<ChapterSummaryRow>,
    #[serde(default)]
    pub subplot_ops: Vec<LooseOp>,
    #[serde(default)]
    pub emotional_arc_ops: Vec<LooseOp>,
    #[serde(default)]
    pub character_matrix_ops: Vec<LooseOp>,
    #[serde(default)]
    pub notes: Vec<String>,
}

// ── Snapshot ─────────────────────────────────────────────────

/// 运行时状态快照
/// manifest + currentState + hooks + chapterSummaries 的聚合视图。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStateSnapshot {
    pub manifest: StateManifest,
    pub current_state: CurrentStateState,
    pub hooks: HooksState,
    pub chapter_summaries: ChapterSummariesState,
}

impl RuntimeStateSnapshot {
    pub fn empty(language: Language) -> Self {
        Self {
            manifest: StateManifest::new(language),
            current_state: CurrentStateState::default(),
            hooks: HooksState::default(),
            chapter_summaries: ChapterSummariesState::default(),
        }
    }
}
