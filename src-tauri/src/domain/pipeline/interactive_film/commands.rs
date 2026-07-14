// 互动电影 IPC 命令。
//
// 命令列表（前端使用 camelCase 调用）：
// - film_generate_graph: 从前提生成 StoryGraph
// - film_export_html: 导出为可玩 HTML
// - film_export_ink: 导出为 Ink 脚本
// - film_apply_delta: 应用 StoryGraphDelta 到图
//
// 约定：命令仅做参数提取 + 校验 + 委派，不含业务逻辑。

use crate::core::agent::commands::AgentState;
use crate::domain::pipeline::interactive_film::authoring::apply_graph_delta;
use crate::domain::pipeline::interactive_film::delta::StoryGraphDelta;
use crate::domain::pipeline::interactive_film::export_html::build_playable_html;
use crate::domain::pipeline::interactive_film::export_ink::export_ink;
use crate::domain::pipeline::interactive_film::generate::generate_story_graph;
use crate::domain::pipeline::interactive_film::graph_schema::StoryGraph;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;

/// 从故事前提生成完整 StoryGraph。
#[tauri::command]
pub async fn film_generate_graph(
    agent_state: State<'_, AgentState>,
    premise: String,
    language: Option<String>,
) -> Result<IpcResponse<StoryGraph>, AppError> {
    if premise.trim().is_empty() {
        return Err(AppError::invalid_input("premise 不能为空"));
    }
    let lang = language.unwrap_or_else(|| "zh".to_string());
    let graph = generate_story_graph(&agent_state.engine, &premise, &lang).await?;
    tracing::info!(
        nodes = graph.nodes.len(),
        endings = graph.endings.len(),
        "film_generate_graph 完成"
    );
    Ok(IpcResponse::ok(graph))
}

/// 导出为单文件可玩 HTML。
#[tauri::command]
pub async fn film_export_html(
    graph_json: String,
) -> Result<IpcResponse<String>, AppError> {
    let graph: StoryGraph = serde_json::from_str(&graph_json)
        .map_err(|e| AppError::invalid_input(format!("StoryGraph 解析失败: {}", e)))?;
    let html = build_playable_html(&graph)?;
    Ok(IpcResponse::ok(html))
}

/// 导出为 Ink 脚本。
#[tauri::command]
pub async fn film_export_ink(
    graph_json: String,
) -> Result<IpcResponse<String>, AppError> {
    let graph: StoryGraph = serde_json::from_str(&graph_json)
        .map_err(|e| AppError::invalid_input(format!("StoryGraph 解析失败: {}", e)))?;
    let ink = export_ink(&graph)?;
    Ok(IpcResponse::ok(ink))
}

/// 应用 StoryGraphDelta 到图，返回更新后的图。
#[tauri::command]
pub async fn film_apply_delta(
    graph_json: String,
    delta_json: String,
) -> Result<IpcResponse<StoryGraph>, AppError> {
    let mut graph: StoryGraph = serde_json::from_str(&graph_json)
        .map_err(|e| AppError::invalid_input(format!("StoryGraph 解析失败: {}", e)))?;
    let delta: StoryGraphDelta = serde_json::from_str(&delta_json)
        .map_err(|e| AppError::invalid_input(format!("StoryGraphDelta 解析失败: {}", e)))?;
    apply_graph_delta(&mut graph, &delta)?;
    Ok(IpcResponse::ok(graph))
}
