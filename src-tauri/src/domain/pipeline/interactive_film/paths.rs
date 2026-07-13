// 路径枚举。
//
// DFS 枚举所有从 start 到 ending（或死路）的可玩路径。
// 状态去重：visit_key = "{node_id}\0{serialize_var_state}"，同一路径上相同 visitKey 不重复访问。

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::evaluator::{apply_effects, init_var_state, visible_choices, VarState};
use super::graph_schema::{NodeType, StoryGraph, StoryNode};

pub const MAX_PATHS: usize = 200;
pub const MAX_DEPTH: usize = 50;

/// 运行时路径
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimePath {
    pub node_ids: Vec<String>,
    pub ending_id: Option<String>,
    pub length: usize,
}

/// 路径枚举结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct PathEnumerationResult {
    pub paths: Vec<RuntimePath>,
    pub truncated: bool,
}

/// 序列化变量状态为稳定 key（key 排序，值用 JSON 序列化）。
fn serialize_var_state(state: &VarState) -> String {
    let mut keys: Vec<&String> = state.keys().collect();
    keys.sort();
    keys.iter()
        .map(|k| {
            let val = state.get(*k).unwrap_or(&Value::Null);
            format!("{}:{}", k, val)
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// DFS 上下文（借用 graph 数据，避免重复建表）。
struct WalkCtx<'a> {
    node_by_id: HashMap<&'a str, &'a StoryNode>,
    ending_by_node_id: HashMap<&'a str, &'a str>,
}

/// DFS 递归体。
fn walk(
    node_id: &str,
    vars: &VarState,
    trail: &[String],
    on_path: &HashSet<String>,
    depth: usize,
    ctx: &WalkCtx,
    paths: &mut Vec<RuntimePath>,
    truncated: &mut bool,
) {
    if paths.len() >= MAX_PATHS {
        *truncated = true;
        return;
    }
    if depth > MAX_DEPTH {
        *truncated = true;
        return;
    }
    let visit_key = format!("{}\u{0000}{}", node_id, serialize_var_state(vars));
    if on_path.contains(&visit_key) {
        return;
    }
    let mut next_on_path = on_path.clone();
    next_on_path.insert(visit_key);

    let node = match ctx.node_by_id.get(node_id) {
        Some(n) => *n,
        None => return,
    };

    let mut next_trail = trail.to_vec();
    next_trail.push(node_id.to_string());

    if node.node_type == NodeType::Ending {
        let ending_id = ctx
            .ending_by_node_id
            .get(node_id)
            .map(|s| s.to_string());
        let length = next_trail.len();
        paths.push(RuntimePath {
            node_ids: next_trail,
            ending_id,
            length,
        });
        return;
    }

    let choices = visible_choices(node, vars);
    if choices.is_empty() {
        // dead-end leaf (no ending): record as a terminal path with null ending
        let length = next_trail.len();
        paths.push(RuntimePath {
            node_ids: next_trail,
            ending_id: None,
            length,
        });
        return;
    }

    for choice in choices {
        if paths.len() >= MAX_PATHS {
            *truncated = true;
            return;
        }
        let mut next_vars = vars.clone();
        apply_effects(&choice.effects, &mut next_vars);
        walk(
            &choice.target_node_id,
            &next_vars,
            &next_trail,
            &next_on_path,
            depth + 1,
            ctx,
            paths,
            truncated,
        );
    }
}

/// DFS 枚举所有从 start 到 ending 的可玩路径。
pub fn enumerate_runtime_paths(graph: &StoryGraph) -> PathEnumerationResult {
    let mut paths: Vec<RuntimePath> = Vec::new();
    let mut truncated = false;

    let node_by_id: HashMap<&str, &StoryNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let ending_by_node_id: HashMap<&str, &str> = graph
        .endings
        .iter()
        .map(|e| (e.node_id.as_str(), e.id.as_str()))
        .collect();

    let start = graph.nodes.iter().find(|n| n.node_type == NodeType::Start);
    let start = match start {
        Some(s) => s,
        None => return PathEnumerationResult { paths, truncated },
    };

    let ctx = WalkCtx {
        node_by_id,
        ending_by_node_id,
    };

    let initial_state = init_var_state(graph);
    walk(
        &start.id,
        &initial_state,
        &[],
        &HashSet::new(),
        0,
        &ctx,
        &mut paths,
        &mut truncated,
    );

    PathEnumerationResult { paths, truncated }
}
