// 校验。
//
// - validate_story_graph：基础校验（4 个 error/warning 级，阻塞型）
// - review_story_graph：深度审查（在 validate 基础上追加 9 个 info/warning 级）
//
// ValidationErrorCode 扩展：任务规格只列了 4 个 error 级 code，
// 但 review 需要 9 个额外 code，因此 enum 包含全部 13 个变体。
// ValidationErrorLevel 同理追加 Info（源码使用 error/warning/info 三级）。

use std::collections::{HashMap, HashSet};

use super::graph_schema::{NodeType, StoryGraph, StoryNode};
use super::paths::enumerate_runtime_paths;

/// 校验错误级别
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationErrorLevel {
    Error,
    Warning,
    Info,
}

/// 校验错误码（snake_case 序列化，便于前端处理）
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationErrorCode {
    // validate 基础（4 个）
    BrokenLink,
    DeadEnd,
    NoPathToEnding,
    Unreachable,
    // review 追加（9 个）
    VariableUnwritten,
    VariableUnused,
    EndingVariety,
    ImageMissing,
    GatedUnreachable,
    EndingUnreachable,
    LinearGraph,
    IsolatedNode,
    IllusoryBranch,
    LongLinearChain,
}

/// 校验问题
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationError {
    pub level: ValidationErrorLevel,
    pub code: ValidationErrorCode,
    pub node_id: Option<String>,
    pub message: String,
}

fn label(node: &StoryNode) -> String {
    if node.title.is_empty() {
        node.id.clone()
    } else {
        node.title.clone()
    }
}

/// 基础校验（4 个 error/warning 级，阻塞型）。
pub fn validate_story_graph(graph: &StoryGraph) -> Vec<ValidationError> {
    let mut issues = Vec::new();
    let ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    let node_map: HashMap<&str, &StoryNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // BROKEN_LINK：选项指向不存在的节点
    for node in &graph.nodes {
        for c in &node.choices {
            if !ids.contains(c.target_node_id.as_str()) {
                issues.push(ValidationError {
                    level: ValidationErrorLevel::Error,
                    code: ValidationErrorCode::BrokenLink,
                    node_id: Some(node.id.clone()),
                    message: format!(
                        "node '{}' choice '{}' targets missing node '{}'",
                        label(node),
                        c.text,
                        c.target_node_id
                    ),
                });
            }
        }
    }

    // DEAD_END：非结局节点没有任何指向存在节点的出口
    for node in &graph.nodes {
        if node.node_type == NodeType::Ending {
            continue;
        }
        let has_exit = node
            .choices
            .iter()
            .any(|c| ids.contains(c.target_node_id.as_str()));
        if !has_exit {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Error,
                code: ValidationErrorCode::DeadEnd,
                node_id: Some(node.id.clone()),
                message: format!("node '{}' is a dead-end: no valid exit", label(node)),
            });
        }
    }

    // 可达性 BFS（start 不存在时退化为第一个节点）
    let start = graph
        .nodes
        .iter()
        .find(|n| n.node_type == NodeType::Start)
        .or_else(|| graph.nodes.first());
    let mut reachable: HashSet<String> = HashSet::new();
    if let Some(start) = start {
        let mut queue: Vec<String> = vec![start.id.clone()];
        while let Some(cur) = queue.pop() {
            if reachable.contains(&cur) {
                continue;
            }
            reachable.insert(cur.clone());
            if let Some(n) = node_map.get(cur.as_str()) {
                for c in &n.choices {
                    if ids.contains(c.target_node_id.as_str())
                        && !reachable.contains(&c.target_node_id)
                    {
                        queue.push(c.target_node_id.clone());
                    }
                }
            }
        }
    }

    // UNREACHABLE（warning）：节点从 start 不可达
    if graph.nodes.len() > 1 {
        for node in &graph.nodes {
            if !reachable.contains(&node.id) {
                issues.push(ValidationError {
                    level: ValidationErrorLevel::Warning,
                    code: ValidationErrorCode::Unreachable,
                    node_id: Some(node.id.clone()),
                    message: format!("node '{}' is unreachable from start", label(node)),
                });
            }
        }
    }

    // NO_PATH_TO_ENDING：从 start 无法到达任何结局
    if let Some(start) = start {
        let can_end = reachable.iter().any(|id| {
            node_map
                .get(id.as_str())
                .map(|n| n.node_type == NodeType::Ending)
                .unwrap_or(false)
        });
        if !can_end {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Error,
                code: ValidationErrorCode::NoPathToEnding,
                node_id: Some(start.id.clone()),
                message: format!(
                    "no path from start node '{}' reaches any ending",
                    label(start)
                ),
            });
        }
    }

    issues
}

/// 深度审查（在 validate 基础上追加 9 个 info/warning 级）。
pub fn review_story_graph(graph: &StoryGraph) -> Vec<ValidationError> {
    let mut issues = validate_story_graph(graph);

    // 读取/写入变量集合
    let mut reads: HashSet<String> = HashSet::new();
    let mut writes: HashSet<String> = HashSet::new();
    for node in &graph.nodes {
        for choice in &node.choices {
            if let Some(cond) = &choice.condition {
                reads.insert(cond.var.clone());
            }
            for effect in &choice.effects {
                writes.insert(effect.var.clone());
            }
        }
    }

    // VARIABLE_UNWRITTEN：被条件读取但从未被 effect 写入
    for v in &reads {
        if !writes.contains(v) {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Warning,
                code: ValidationErrorCode::VariableUnwritten,
                node_id: None,
                message: format!(
                    "variable '{}' is read by a condition but never written by any effect",
                    v
                ),
            });
        }
    }

    // VARIABLE_UNUSED：声明了但既没被读也没被写
    for v in &graph.variables {
        if !reads.contains(&v.name) && !writes.contains(&v.name) {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Info,
                code: ValidationErrorCode::VariableUnused,
                node_id: None,
                message: format!(
                    "variable '{}' is declared but never read or written",
                    v.name
                ),
            });
        }
    }

    // ENDING_VARIETY：所有结局同类型（重玩价值低）
    if graph.endings.len() >= 2 {
        let first_type = graph.endings.first().map(|e| &e.ending_type);
        let all_same = graph
            .endings
            .iter()
            .all(|e| Some(&e.ending_type) == first_type);
        if all_same {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Info,
                code: ValidationErrorCode::EndingVariety,
                node_id: None,
                message: format!(
                    "all {} endings share the same type — low replay value",
                    graph.endings.len()
                ),
            });
        }
    }

    // IMAGE_MISSING：非结局节点未配图（无 asset_ref）
    for node in &graph.nodes {
        if node.node_type == NodeType::Ending {
            continue;
        }
        let has_asset = node
            .image_slot
            .as_ref()
            .and_then(|s| s.asset_ref.as_ref())
            .is_some();
        if !has_asset {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Info,
                code: ValidationErrorCode::ImageMissing,
                node_id: Some(node.id.clone()),
                message: format!("node '{}' has no image asset", label(node)),
            });
        }
    }

    // 路径枚举用于 GATED_UNREACHABLE / ENDING_UNREACHABLE
    let path_result = enumerate_runtime_paths(graph);
    let mut reached_node_ids: HashSet<String> = HashSet::new();
    for p in &path_result.paths {
        for id in &p.node_ids {
            reached_node_ids.insert(id.clone());
        }
    }
    let edge_reachable = compute_edge_reachable(graph);

    // GATED_UNREACHABLE / ENDING_UNREACHABLE 依赖完整路径枚举；
    // 截断时（>200 路径）reached 集合不完整，断言不可达会出错，跳过。
    if !path_result.truncated {
        // GATED_UNREACHABLE：连边可达但运行时不可达（非 start/ending）
        for node in &graph.nodes {
            if node.node_type == NodeType::Start || node.node_type == NodeType::Ending {
                continue;
            }
            if edge_reachable.contains(&node.id) && !reached_node_ids.contains(&node.id) {
                issues.push(ValidationError {
                    level: ValidationErrorLevel::Warning,
                    code: ValidationErrorCode::GatedUnreachable,
                    node_id: Some(node.id.clone()),
                    message: format!(
                        "node '{}' is edge-reachable but no satisfying variable path reaches it",
                        label(node)
                    ),
                });
            }
        }

        // ENDING_UNREACHABLE：结局无真实路径可达
        for ending in &graph.endings {
            if !reached_node_ids.contains(&ending.node_id) {
                issues.push(ValidationError {
                    level: ValidationErrorLevel::Warning,
                    code: ValidationErrorCode::EndingUnreachable,
                    node_id: Some(ending.node_id.clone()),
                    message: format!(
                        "ending '{}' has no real path reaching it",
                        ending.title
                    ),
                });
            }
        }
    }

    // LINEAR_GRAPH：无任何分叉（且有 normal 节点 + start + ending 才有意义）
    let has_branch = graph.nodes.iter().any(|n| n.choices.len() >= 2);
    let has_normal = graph
        .nodes
        .iter()
        .any(|n| n.node_type == NodeType::Normal);
    let has_start = graph
        .nodes
        .iter()
        .any(|n| n.node_type == NodeType::Start);
    if has_start && !graph.endings.is_empty() && has_normal && !has_branch {
        issues.push(ValidationError {
            level: ValidationErrorLevel::Info,
            code: ValidationErrorCode::LinearGraph,
            node_id: None,
            message: "story has no branching choices — more linear script than interactive film"
                .into(),
        });
    }

    // ISOLATED_NODE：无入边的孤立节点（非 start/ending）；已有 UNREACHABLE 的跳过避免噪声
    let mut incoming: HashSet<String> = HashSet::new();
    for n in &graph.nodes {
        for c in &n.choices {
            incoming.insert(c.target_node_id.clone());
        }
    }
    let unreachable_ids: HashSet<String> = issues
        .iter()
        .filter(|i| i.code == ValidationErrorCode::Unreachable)
        .filter_map(|i| i.node_id.clone())
        .collect();
    for node in &graph.nodes {
        if node.node_type == NodeType::Start || node.node_type == NodeType::Ending {
            continue;
        }
        if !incoming.contains(&node.id) && !unreachable_ids.contains(&node.id) {
            issues.push(ValidationError {
                level: ValidationErrorLevel::Info,
                code: ValidationErrorCode::IsolatedNode,
                node_id: Some(node.id.clone()),
                message: format!("node '{}' has no incoming choices — isolated", label(node)),
            });
        }
    }

    // ILLUSORY_BRANCH：所有选项通向同一节点且无 effects
    for node in &graph.nodes {
        if node.choices.len() >= 2 {
            let all_same_target = node
                .choices
                .iter()
                .all(|c| c.target_node_id == node.choices[0].target_node_id);
            let all_no_effect = node.choices.iter().all(|c| c.effects.is_empty());
            if all_same_target && all_no_effect {
                issues.push(ValidationError {
                    level: ValidationErrorLevel::Info,
                    code: ValidationErrorCode::IllusoryBranch,
                    node_id: Some(node.id.clone()),
                    message: format!(
                        "node '{}' choices all target the same node with no effects — fake branch",
                        label(node)
                    ),
                });
            }
        }
    }

    // LONG_LINEAR_CHAIN：>=5 个连续单选项 normal 节点
    const CHAIN_THRESHOLD: usize = 5;
    for head_id in find_long_linear_chain_heads(graph, CHAIN_THRESHOLD) {
        issues.push(ValidationError {
            level: ValidationErrorLevel::Info,
            code: ValidationErrorCode::LongLinearChain,
            node_id: Some(head_id.clone()),
            message: format!(
                "from node '{}' a long no-branch chain (>= {} single-choice nodes) begins",
                head_id, CHAIN_THRESHOLD
            ),
        });
    }

    issues
}

/// 连边可达性 BFS（忽略变量条件，仅拓扑）。
fn compute_edge_reachable(graph: &StoryGraph) -> HashSet<String> {
    let mut reachable: HashSet<String> = HashSet::new();
    let start = graph
        .nodes
        .iter()
        .find(|n| n.node_type == NodeType::Start);
    let start = match start {
        Some(s) => s,
        None => return reachable,
    };
    let node_by_id: HashMap<&str, &StoryNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut queue: Vec<String> = vec![start.id.clone()];
    while let Some(cur) = queue.pop() {
        if reachable.contains(&cur) {
            continue;
        }
        reachable.insert(cur.clone());
        if let Some(node) = node_by_id.get(cur.as_str()) {
            for choice in &node.choices {
                if !reachable.contains(&choice.target_node_id) {
                    queue.push(choice.target_node_id.clone());
                }
            }
        }
    }
    reachable
}

/// 找出长度 >= threshold 的单选项 normal 节点直链的链头。
fn find_long_linear_chain_heads(graph: &StoryGraph, threshold: usize) -> Vec<String> {
    let node_by_id: HashMap<&str, &StoryNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let is_chain_node = |id: &str| -> bool {
        node_by_id
            .get(id)
            .map(|n| n.node_type == NodeType::Normal && n.choices.len() == 1)
            .unwrap_or(false)
    };
    // 记录哪些节点的入边来自 chain node（这些节点不是链头）
    let mut incoming_is_chain: HashSet<String> = HashSet::new();
    for n in &graph.nodes {
        if is_chain_node(&n.id) {
            for c in &n.choices {
                incoming_is_chain.insert(c.target_node_id.clone());
            }
        }
    }
    let mut heads: Vec<String> = Vec::new();
    for node in &graph.nodes {
        if !is_chain_node(&node.id) {
            continue;
        }
        if incoming_is_chain.contains(&node.id) {
            continue;
        }
        // 沿单选项链向下计数
        let mut length: usize = 0;
        let mut cur: Option<&str> = Some(&node.id);
        let mut visited: HashSet<String> = HashSet::new();
        while let Some(cur_id) = cur {
            if !is_chain_node(cur_id) || visited.contains(cur_id) {
                break;
            }
            visited.insert(cur_id.to_string());
            length += 1;
            cur = node_by_id
                .get(cur_id)
                .and_then(|n| n.choices.first())
                .map(|c| c.target_node_id.as_str());
        }
        if length >= threshold {
            heads.push(node.id.clone());
        }
    }
    heads
}
