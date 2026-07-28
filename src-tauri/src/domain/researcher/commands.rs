//! ═══════════════════════════════════════════════════════════════════════════
//! 研究员命令 - IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! - researcher_run: 基于 LLM 生成结构化研究报告

use crate::core::agent::commands::AgentState;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;
use std::time::Instant;

use super::report::run_research_report;
use super::types::{ResearchInput, ResearchReport};

#[tauri::command]
pub async fn researcher_run(
    agent_state: State<'_, AgentState>,
    input: ResearchInput,
) -> Result<IpcResponse<ResearchReport>, AppError> {
    let start = Instant::now();
    tracing::info!(
        query = %input.query,
        depth = ?input.depth,
        "researcher_run: enter"
    );
    
    let report = run_research_report(&agent_state.engine, &input).await.map_err(|e| {
        tracing::error!(query = %input.query, error = %e, "researcher_run: Failed to run research report");
        e
    })?;
    
    tracing::info!(
        query = %report.query,
        depth = %report.depth,
        claims = report.claims.len(),
        duration_ms = start.elapsed().as_millis(),
        "researcher_run: exit"
    );
    Ok(IpcResponse::ok(report))
}