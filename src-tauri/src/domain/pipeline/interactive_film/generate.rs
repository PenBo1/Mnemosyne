//! ═══════════════════════════════════════════════════════════════════════════
//! Interactive Film Generate - StoryGraph 生成
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! generate_story_graph：从故事前提调用 LLM 产出 StoryGraph JSON。
//! 约束：严格 JSON、恰好 1 个 start、>=2 个 branch、>=2 个差异化 ending。
//!
//! 注意：AgentEngine.prompt_once 不支持 temperature 参数，
//! 温度引导（"保持中等创造性"）写入 system prompt 文本。

use std::time::Duration;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::graph_schema::{NodeType, StoryGraph};

/// LLM 调用超时（秒）
const LLM_TIMEOUT_SECS: u64 = 120;

/// 从故事前提一次性生成完整 StoryGraph。
pub async fn generate_story_graph(
    engine: &AgentEngine,
    premise: &str,
    language: &str,
) -> Result<StoryGraph, AppError> {
    let system_prompt = build_generate_prompt(language);
    let user_message = format!("故事前提：\n{}", premise);

    let raw = tokio::time::timeout(
        Duration::from_secs(LLM_TIMEOUT_SECS),
        engine.prompt_once(&system_prompt, &user_message),
    )
    .await
    .map_err(|_| {
        AppError::task_timeout()
    })??;
    tracing::info!(
        response_len = raw.len(),
        "generate_story_graph LLM 响应"
    );

    let json_str = extract_json_block(&raw).ok_or_else(|| {
        AppError::invalid_state("LLM 输出未包含可识别的 JSON 块".to_string())
    })?;
    let graph: StoryGraph = serde_json::from_str(json_str).map_err(|e| {
        AppError::invalid_state(format!("StoryGraph 解析失败: {}", e))
    })?;

    validate_generated_graph(&graph)?;
    Ok(graph)
}

/// 校验生成结果的基础结构约束。
fn validate_generated_graph(graph: &StoryGraph) -> Result<(), AppError> {
    let start_count = graph
        .nodes
        .iter()
        .filter(|n| n.node_type == NodeType::Start)
        .count();
    if start_count != 1 {
        return Err(AppError::invalid_state(format!(
            "生成图必须恰好 1 个 start 节点，实际 {}",
            start_count
        )));
    }
    let branch_count = graph
        .nodes
        .iter()
        .filter(|n| n.node_type == NodeType::Branch)
        .count();
    if branch_count < 2 {
        return Err(AppError::invalid_state(format!(
            "生成图至少需要 2 个 branch 节点，实际 {}",
            branch_count
        )));
    }
    if graph.endings.len() < 2 {
        return Err(AppError::invalid_state(format!(
            "生成图至少需要 2 个 ending，实际 {}",
            graph.endings.len()
        )));
    }
    Ok(())
}

fn build_generate_prompt(language: &str) -> String {
    let lang_hint = match language {
        "en" => "Use English for all text content.",
        _ => "所有正文使用中文。",
    };
    format!(
        r#"你是互动电影 StoryGraph 生成器。根据故事前提，一次性生成完整的 StoryGraph JSON。

结构约束：
- schemaVersion: 1
- projectId: 自定义标识
- title: 故事标题
- worldAnchor: 世界锚点（storyCore/theme/genre/worldRules/durationMinutes）
- characters: 角色列表（id/name/role/motivation/voiceProfile）
- variables: 变量列表（name/type:flag|counter|relationship|item/default/desc）
- nodes: 节点列表，每个节点含 id/title/type/sceneDesc/dialogue/choices/imageSlot/act/position
- endings: 结局列表（id/nodeId/title/type:good|bad|neutral|secret/description）

节点类型（type）：start/normal/branch/merge/ending/explore
选项（choices）：id/text/targetNodeId/condition?/effects[]/weight?:light|heavy|critical
条件（condition）：var/op(>=,<=,>,<,==,!=)/value
效果（effects）：var/op(set|add|sub)/value

硬性要求：
- 恰好 1 个 type=start 节点
- 至少 2 个 type=branch 节点（提供真实分叉）
- 至少 2 个 endings，且 type 两两不同（good/bad/neutral/secret 中至少 2 种）
- 所有 choice 的 targetNodeId 必须指向存在的节点
- 保持中等创造性，避免过度线性

只输出一个完整 JSON 对象，不要任何解释文字、不要 markdown 代码块标记。
{lang_hint}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pipeline::interactive_film::graph_schema::{
        Choice, Ending, EndingType, NodeType, StoryGraph, StoryNode,
    };

    fn graph_with(starts: usize, branches: usize, endings: usize) -> StoryGraph {
        let mut nodes = Vec::new();
        for _ in 0..starts {
            nodes.push(StoryNode {
                id: "start".into(),
                title: "开始".into(),
                node_type: NodeType::Start,
                scene_desc: "".into(),
                dialogue: vec![],
                choices: vec![],
                image_slot: None,
                act: "".into(),
                position: None,
            });
        }
        for i in 0..branches {
            nodes.push(StoryNode {
                id: format!("b{}", i),
                title: format!("分支{}", i),
                node_type: NodeType::Branch,
                scene_desc: "".into(),
                dialogue: vec![],
                choices: vec![Choice {
                    id: format!("c{}", i),
                    text: "选".into(),
                    target_node_id: "end".into(),
                    condition: None,
                    effects: vec![],
                    weight: None,
                }],
                image_slot: None,
                act: "".into(),
                position: None,
            });
        }
        nodes.push(StoryNode {
            id: "end".into(),
            title: "结局节点".into(),
            node_type: NodeType::Ending,
            scene_desc: "".into(),
            dialogue: vec![],
            choices: vec![],
            image_slot: None,
            act: "".into(),
            position: None,
        });
        let mut ends = Vec::new();
        for i in 0..endings {
            let t = if i % 2 == 0 { EndingType::Good } else { EndingType::Bad };
            ends.push(Ending {
                id: format!("e{}", i),
                node_id: "end".into(),
                title: format!("结局{}", i),
                ending_type: t,
                description: "".into(),
            });
        }
        StoryGraph {
            schema_version: 1,
            project_id: "p".into(),
            title: "t".into(),
            world_anchor: None,
            characters: vec![],
            variables: vec![],
            nodes,
            endings: ends,
        }
    }

    #[test]
    fn validate_ok() {
        let g = graph_with(1, 2, 2);
        assert!(validate_generated_graph(&g).is_ok());
    }

    #[test]
    fn validate_rejects_zero_start() {
        let g = graph_with(0, 2, 2);
        assert!(validate_generated_graph(&g).is_err());
    }

    #[test]
    fn validate_rejects_one_ending() {
        let g = graph_with(1, 2, 1);
        assert!(validate_generated_graph(&g).is_err());
    }
}
