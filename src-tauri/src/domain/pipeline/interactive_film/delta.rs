// Delta 系统。
//
// - UpsertRemove<T>：upsert/remove 语义的 Vec 操作
// - StoryGraphDelta：对 graph 各集合的增量修改
// - apply_story_graph_delta：将 delta 应用到 graph（含 ending 引用完整性校验）

use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

use super::graph_schema::{Character, Ending, StoryGraph, StoryNode, Variable, WorldAnchor};

/// upsert/remove 语义的 Vec 操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRemove<T> {
    // 使用显式 default 函数避免 serde derive 错误地要求 T: Default
    #[serde(default = "Vec::new")]
    pub upsert: Vec<T>,
    #[serde(default = "Vec::new")]
    pub remove: Vec<String>,
}

/// StoryGraphDelta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryGraphDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_anchor: Option<WorldAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub characters: Option<UpsertRemove<Character>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes: Option<UpsertRemove<StoryNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<UpsertRemove<Variable>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endings: Option<UpsertRemove<Ending>>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// 拥有唯一标识的类型（用于 upsert/remove 定位）。
pub trait HasId {
    fn id(&self) -> &str;
}

impl HasId for Character {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for StoryNode {
    fn id(&self) -> &str {
        &self.id
    }
}

// Variable 用 name 作为唯一标识。
impl HasId for Variable {
    fn id(&self) -> &str {
        &self.name
    }
}

impl HasId for Ending {
    fn id(&self) -> &str {
        &self.id
    }
}

/// 应用 UpsertRemove 到 Vec：
/// - 先按 remove 列表删除匹配 id 的项
/// - 再按 upsert 列表：id 已存在则替换，否则 push
fn apply_upsert_remove<T: HasId + Clone>(vec: &mut Vec<T>, op: &UpsertRemove<T>) {
    if !op.remove.is_empty() {
        let remove_set: std::collections::HashSet<&str> =
            op.remove.iter().map(|s| s.as_str()).collect();
        vec.retain(|item| !remove_set.contains(item.id()));
    }
    for item in &op.upsert {
        let id = item.id();
        if let Some(existing) = vec.iter_mut().find(|x| x.id() == id) {
            *existing = item.clone();
        } else {
            vec.push(item.clone());
        }
    }
}

/// 应用 delta 到 graph。
///
/// world_anchor：delta 提供则直接替换（简化版，完整实现是 partial 合并）。
/// characters/nodes/variables/endings：调用 apply_upsert_remove。
/// 校验：所有 ending 的 node_id 必须存在于 nodes 中（阻塞型完整性约束）。
pub fn apply_story_graph_delta(
    graph: &mut StoryGraph,
    delta: &StoryGraphDelta,
) -> Result<(), AppError> {
    if let Some(ref anchor) = delta.world_anchor {
        graph.world_anchor = Some(anchor.clone());
    }

    if let Some(ref ops) = delta.characters {
        apply_upsert_remove(&mut graph.characters, ops);
    }
    if let Some(ref ops) = delta.nodes {
        apply_upsert_remove(&mut graph.nodes, ops);
    }
    if let Some(ref ops) = delta.variables {
        apply_upsert_remove(&mut graph.variables, ops);
    }
    if let Some(ref ops) = delta.endings {
        apply_upsert_remove(&mut graph.endings, ops);
    }

    // Referential integrity (blocking): every ending must point at an existing node.
    let node_ids: std::collections::HashSet<&str> =
        graph.nodes.iter().map(|n| n.id.as_str()).collect();
    for ending in &graph.endings {
        if !node_ids.contains(ending.node_id.as_str()) {
            return Err(AppError::invalid_state(format!(
                "ending {} references missing node {}",
                ending.id, ending.node_id
            )));
        }
    }

    Ok(())
}
