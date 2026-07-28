//! ═══════════════════════════════════════════════════════════════════════════
//! Interactive Film Commands - IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 命令列表（前端使用 camelCase 调用）：
//! - film_generate_graph: 从前提生成 StoryGraph
//! - film_export_html: 导出为可玩 HTML
//! - film_export_ink: 导出为 Ink 脚本
//! - film_apply_delta: 应用 StoryGraphDelta 到图
//!
//! 约定：命令仅做参数提取 + 校验 + 委派，不含业务逻辑。

use crate::core::agent::commands::AgentState;
use crate::domain::pipeline::interactive_film::authoring::apply_graph_delta;
use crate::domain::pipeline::interactive_film::delta::StoryGraphDelta;
use crate::domain::pipeline::interactive_film::export_html::build_playable_html;
use crate::domain::pipeline::interactive_film::export_ink::export_ink;
use crate::domain::pipeline::interactive_film::generate::generate_story_graph;
use crate::domain::pipeline::interactive_film::graph_schema::StoryGraph;
use crate::shared::error::{AppError, IpcResponse};
use std::time::Instant;
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
    graph: StoryGraph,
) -> Result<IpcResponse<String>, AppError> {
    let start = Instant::now();
    tracing::info!(nodes = graph.nodes.len(), "film_export_html: enter");
    
    let html = build_playable_html(&graph)?;
    tracing::info!(
        html_len = html.len(),
        duration_ms = start.elapsed().as_millis(),
        "film_export_html: exit"
    );
    Ok(IpcResponse::ok(html))
}

/// 导出为 Ink 脚本。
#[tauri::command]
pub async fn film_export_ink(
    graph: StoryGraph,
) -> Result<IpcResponse<String>, AppError> {
    let ink = export_ink(&graph)?;
    Ok(IpcResponse::ok(ink))
}

/// 应用 StoryGraphDelta 到图，返回更新后的图。
#[tauri::command]
pub async fn film_apply_delta(
    mut graph: StoryGraph,
    delta: StoryGraphDelta,
) -> Result<IpcResponse<StoryGraph>, AppError> {
    apply_graph_delta(&mut graph, &delta)?;
    Ok(IpcResponse::ok(graph))
}
