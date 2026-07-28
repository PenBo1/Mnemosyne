//! ═══════════════════════════════════════════════════════════════════════════
//! Play 类型 - 数据模型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 一个"世界"由实体/边/状态槽/事件构成有向图，
//! 玩家每回合提交自然语言动作，经 4-agent 流水线归一→变更→渲染→对账，
//! 最终生成 PlayMutation 提交到 SQLite 图谱。
//!
//! serde 规则（与 interactive_film 保持一致）：
//! - struct 字段用 camelCase（IPC 约定）
//! - enum 变体用 snake_case

use serde::{Deserialize, Serialize};

/// 玩家动作类型（5 类）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlayActionKind {
    Look,
    Say,
    Move,
    Do,
    Wait,
}

/// 玩家动作意图（归一后的结构化动作）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayActionIntent {
    pub action_kind: PlayActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_entity_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_location_label: Option<String>,
    #[serde(default)]
    pub intent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambiguity: Option<String>,
    #[serde(default)]
    pub secondary_actions: Vec<String>,
}

/// 实体类型（11 种）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlayEntityType {
    Actor,
    Location,
    Item,
    Evidence,
    Clue,
    Claim,
    ProofChain,
    Organization,
    Rule,
    Scene,
    Event,
}

/// 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEntity {
    pub id: String,
    pub label: String,
    pub entity_type: PlayEntityType,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical: Option<bool>,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

/// 关系边
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEdge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String, // relation / holding / observed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_event_id: Option<String>,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

/// 边过期条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEdgeExpire {
    pub edge_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 状态槽类型（7 种）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlayStateSlotKind {
    Resource,
    Relation,
    Pressure,
    Clue,
    Evidence,
    Flag,
    Timer,
}

/// 状态槽
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayStateSlot {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_entity_id: Option<String>,
    pub slot_kind: PlayStateSlotKind,
    pub key: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// 证据状态（8 阶，不可倒退）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlayEvidenceStatus {
    Unknown,
    Hinted,
    Seen,
    Collected,
    Verified,
    Weaponized,
    Exposed,
    Exhausted,
}

impl PlayEvidenceStatus {
    /// 阶序：用于判定证据变迁是否合法（不可倒退）
    pub fn rank(&self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Hinted => 1,
            Self::Seen => 2,
            Self::Collected => 3,
            Self::Verified => 4,
            Self::Weaponized => 5,
            Self::Exposed => 6,
            Self::Exhausted => 7,
        }
    }
}

/// 证据变迁
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEvidenceTransition {
    pub claim_id: String,
    pub from_status: PlayEvidenceStatus,
    pub to_status: PlayEvidenceStatus,
}

/// 时间推进
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayTimeAdvance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// 事件（一个回合对应一个事件）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEvent {
    pub event_id: String,
    pub turn: u32,
    pub action_kind: PlayActionKind,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_advance: Option<PlayTimeAdvance>,
    pub timestamp: String,
}

/// Mutation（一回合的完整变更包）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayMutation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_kind: Option<PlayActionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_advance: Option<PlayTimeAdvance>,
    #[serde(default)]
    pub entities_upsert: Vec<PlayEntity>,
    #[serde(default)]
    pub edges_upsert: Vec<PlayEdge>,
    #[serde(default)]
    pub edges_expire: Vec<PlayEdgeExpire>,
    #[serde(default)]
    pub state_slots_upsert: Vec<PlayStateSlot>,
    #[serde(default)]
    pub evidence_transitions: Vec<PlayEvidenceTransition>,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// 世界配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayWorld {
    pub world_id: String,
    pub premise: String,
    #[serde(default)]
    pub world_contract: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_contract: Option<serde_json::Value>,
    #[serde(default = "default_mode")]
    pub mode: String, // open / guided
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_persona: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_mode() -> String {
    "open".to_string()
}

fn default_language() -> String {
    "zh".to_string()
}

/// 回合结果（PlayRunner.step 的返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayStepResult {
    pub turn: u32,
    pub scene_text: String,
    pub action: PlayActionIntent,
    pub mutation: PlayMutation,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
    pub blocked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
}

/// 图谱快照（DB 整体导出/恢复用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayGraphSnapshot {
    #[serde(default)]
    pub entities: Vec<PlayEntity>,
    #[serde(default)]
    pub edges: Vec<PlayEdge>,
    #[serde(default)]
    pub state_slots: Vec<PlayStateSlot>,
    #[serde(default)]
    pub events: Vec<PlayEvent>,
}
