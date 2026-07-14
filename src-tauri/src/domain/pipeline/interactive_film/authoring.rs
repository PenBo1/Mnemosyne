// 互动电影创作辅助。
//
// - build_fill_node_delta_from_llm_text / build_structure_delta_from_llm_text:
//   把 LLM 输出（JSON）转换成 StoryGraphDelta
// - AuthoringState: 创作阶段状态（current_phase / rev / phase_visits）
// - apply_graph_delta: apply_story_graph_delta 的薄封装
// - Delta builder 工厂：构造常见结构性 delta

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::delta::{apply_story_graph_delta, StoryGraphDelta, UpsertRemove};
use super::graph_schema::{
    Character, Ending, StoryGraph, StoryNode, Variable, WorldAnchor,
};

/// 把 LLM 输出转换成 fill-node delta（用于"填充节点内容"场景）。
///
/// 期望 LLM 输出包含一个 StoryNode 或 StoryNode 数组的 JSON。
pub fn build_fill_node_delta_from_llm_text(text: &str) -> Result<StoryGraphDelta, AppError> {
    let json_str = extract_json_block(text).ok_or_else(|| {
        AppError::invalid_state("LLM 输出未包含可识别的 JSON 块".to_string())
    })?;
    // 兼容单节点或节点数组
    let nodes: Vec<StoryNode> = if json_str.trim_start().starts_with('[') {
        serde_json::from_str(json_str)?
    } else {
        vec![serde_json::from_str::<StoryNode>(json_str)?]
    };
    Ok(StoryGraphDelta {
        nodes: Some(UpsertRemove {
            upsert: nodes,
            remove: Vec::new(),
        }),
        ..Default::default()
    })
}

/// 把 LLM 输出转换成 structure delta（用于"结构调整"场景）。
///
/// 期望 LLM 输出本身就是 StoryGraphDelta JSON。
pub fn build_structure_delta_from_llm_text(text: &str) -> Result<StoryGraphDelta, AppError> {
    let json_str = extract_json_block(text).ok_or_else(|| {
        AppError::invalid_state("LLM 输出未包含可识别的 JSON 块".to_string())
    })?;
    let delta: StoryGraphDelta = serde_json::from_str(json_str)?;
    Ok(delta)
}

/// 创作状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringState {
    #[serde(default = "default_phase")]
    pub current_phase: String,
    #[serde(default)]
    pub rev: u32,
    #[serde(default)]
    pub phase_visits: serde_json::Value,
}

fn default_phase() -> String {
    "draft".to_string()
}

impl Default for AuthoringState {
    fn default() -> Self {
        Self {
            current_phase: default_phase(),
            rev: 0,
            phase_visits: serde_json::Value::Object(Default::default()),
        }
    }
}

pub fn load_authoring_state(path: &Path) -> Result<AuthoringState, AppError> {
    if !path.exists() {
        return Ok(AuthoringState::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| {
        AppError::file_read_error(format!("{}: {}", path.display(), e))
    })?;
    let state: AuthoringState = serde_json::from_str(&raw)?;
    Ok(state)
}

pub fn save_authoring_state(path: &Path, state: &AuthoringState) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::internal(format!("Failed to create authoring state dir: {}", e))
        })?;
    }
    let json = serde_json::to_string_pretty(state)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json).map_err(|e| {
        AppError::file_write_error(format!("{}: {}", tmp.display(), e))
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::file_write_error(format!("{}: {}", path.display(), e))
    })?;
    Ok(())
}

/// 应用 delta 到 graph（apply_story_graph_delta 的薄封装）
pub fn apply_graph_delta(
    graph: &mut StoryGraph,
    delta: &StoryGraphDelta,
) -> Result<(), AppError> {
    apply_story_graph_delta(graph, delta)
}

// ── Delta builder 工厂 ────────────────────────────────

/// 设置/替换世界锚点
pub fn build_world_anchor_delta(anchor: WorldAnchor) -> StoryGraphDelta {
    StoryGraphDelta {
        world_anchor: Some(anchor),
        ..Default::default()
    }
}

/// 新增/更新变量
pub fn build_add_variable_delta(variable: Variable) -> StoryGraphDelta {
    StoryGraphDelta {
        variables: Some(UpsertRemove {
            upsert: vec![variable],
            remove: Vec::new(),
        }),
        ..Default::default()
    }
}

/// 定义/更新结局
pub fn build_define_ending_delta(ending: Ending) -> StoryGraphDelta {
    StoryGraphDelta {
        endings: Some(UpsertRemove {
            upsert: vec![ending],
            remove: Vec::new(),
        }),
        ..Default::default()
    }
}

/// 移除节点（同时会留下指向它的悬空 choice，需后续清理）
pub fn build_remove_node_delta(node_id: impl Into<String>) -> StoryGraphDelta {
    StoryGraphDelta {
        nodes: Some(UpsertRemove {
            upsert: Vec::new(),
            remove: vec![node_id.into()],
        }),
        ..Default::default()
    }
}

/// 连接选项：upsert 一个已包含新 choice 的节点。
///
/// 注意：apply_story_graph_delta 对 nodes 是整节点替换语义，
/// 因此调用方需提供包含新 choice 的完整 StoryNode。
pub fn build_connect_choice_delta(node: StoryNode) -> StoryGraphDelta {
    StoryGraphDelta {
        nodes: Some(UpsertRemove {
            upsert: vec![node],
            remove: Vec::new(),
        }),
        ..Default::default()
    }
}

/// 新增/更新角色
pub fn build_upsert_characters_delta(characters: Vec<Character>) -> StoryGraphDelta {
    StoryGraphDelta {
        characters: Some(UpsertRemove {
            upsert: characters,
            remove: Vec::new(),
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_node_delta_from_single_node() {
        let text = r#"{"id":"n1","title":"T","type":"normal","sceneDesc":"s","dialogue":[],"choices":[],"act":""}"#;
        let delta = build_fill_node_delta_from_llm_text(text).unwrap();
        assert_eq!(delta.nodes.unwrap().upsert.len(), 1);
    }

    #[test]
    fn fill_node_delta_from_array() {
        let text = r#"[{"id":"n1","title":"T","type":"normal","sceneDesc":"","dialogue":[],"choices":[],"act":""}]"#;
        let delta = build_fill_node_delta_from_llm_text(text).unwrap();
        assert_eq!(delta.nodes.unwrap().upsert.len(), 1);
    }

    #[test]
    fn authoring_state_default_and_roundtrip() {
        let dir = std::env::temp_dir().join("mnemosyne_authoring_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("state.json");
        // 不存在时返回默认
        let state = load_authoring_state(&path).unwrap();
        assert_eq!(state.current_phase, "draft");
        // 保存后重新加载
        let mut state2 = state;
        state2.rev = 5;
        save_authoring_state(&path, &state2).unwrap();
        let loaded = load_authoring_state(&path).unwrap();
        assert_eq!(loaded.rev, 5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builders_produce_expected_shapes() {
        let v = Variable {
            name: "v".into(),
            var_type: super::super::graph_schema::VariableType::Counter,
            default: serde_json::json!(0),
            desc: None,
        };
        let d = build_add_variable_delta(v);
        assert!(d.variables.is_some());

        let d2 = build_remove_node_delta("n1");
        assert_eq!(d2.nodes.unwrap().remove, vec!["n1".to_string()]);
    }
}
