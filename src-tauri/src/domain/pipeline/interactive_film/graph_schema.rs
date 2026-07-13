// StoryGraph 数据结构。
//
// serde 规则：
// - enum 变体用 `#[serde(rename_all = "snake_case")]`（ConditionOp/EffectOp 除外，含特殊符号）
// - `#[serde(default)]` 对应 zod `.default()`
// - `#[serde(skip_serializing_if = "Option::is_none")]` 对应 zod `.optional()`

use serde::{Deserialize, Serialize};

/// 变量类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    Flag,
    Counter,
    Relationship,
    Item,
}

/// 变量定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: VariableType,
    #[serde(default)]
    pub default: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
}

/// 条件操作符（含特殊符号，手动 rename）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConditionOp {
    #[serde(rename = ">=")]
    Gte,
    #[serde(rename = "<=")]
    Lte,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Neq,
}

/// 条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub var: String,
    pub op: ConditionOp,
    pub value: serde_json::Value,
}

/// 效果操作符（lowercase：set/add/sub）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EffectOp {
    Set,
    Add,
    Sub,
}

/// 效果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub var: String,
    pub op: EffectOp,
    pub value: serde_json::Value,
}

/// 选择权重
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceWeight {
    Light,
    Heavy,
    Critical,
}

/// 选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub target_node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<ChoiceWeight>,
}

/// 对白行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueLine {
    pub speaker: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emotion: Option<String>,
}

/// 节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Start,
    Normal,
    Branch,
    Merge,
    Ending,
    Explore,
}

/// 图片槽位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSlot {
    #[serde(default)]
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_ref: Option<String>,
}

/// 节点位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// 故事节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryNode {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    #[serde(default)]
    pub scene_desc: String,
    #[serde(default)]
    pub dialogue: Vec<DialogueLine>,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_slot: Option<ImageSlot>,
    // 注意：act 字段使用 String 类型（默认 ""），
    // 这里保持与源 schema 一致使用 String（而非 u32），以确保 JSON 兼容。
    #[serde(default)]
    pub act: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

/// 结局类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EndingType {
    Good,
    Bad,
    Neutral,
    Secret,
}

/// 结局
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ending {
    pub id: String,
    pub node_id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub ending_type: EndingType,
    #[serde(default)]
    pub description: String,
}

/// 角色身份
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterRole {
    Protagonist,
    Antagonist,
    Support,
    Other,
}

impl Default for CharacterRole {
    fn default() -> Self {
        Self::Other
    }
}

/// 语音画像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    #[serde(default)]
    pub speaking_rhythm: String,
    #[serde(default)]
    pub vocabulary: String,
    #[serde(default)]
    pub sample_lines: Vec<String>,
}

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub role: CharacterRole,
    #[serde(default)]
    pub motivation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_profile: Option<VoiceProfile>,
}

/// 世界锚点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldAnchor {
    #[serde(default)]
    pub story_core: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub world_rules: String,
    // 注意：durationMinutes 使用 number 类型（默认 0），
    // 这里保持与源 schema 一致使用 u32 + default（而非 Option），以确保 JSON 兼容。
    #[serde(default)]
    pub duration_minutes: u32,
}

/// StoryGraph（顶层结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryGraph {
    pub schema_version: u32,
    pub project_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_anchor: Option<WorldAnchor>,
    #[serde(default)]
    pub characters: Vec<Character>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    pub nodes: Vec<StoryNode>,
    #[serde(default)]
    pub endings: Vec<Ending>,
}
