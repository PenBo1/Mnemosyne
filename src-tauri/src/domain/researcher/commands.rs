// 研究员 IPC 命令(前端 camelCase 调用):
// - researcher_run: 基于 LLM 生成结构化研究报告

use crate::core::agent::commands::AgentState;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;

use super::report::run_research_report;
use super::types::{ResearchInput, ResearchReport};

#[tauri::command]
pub async fn researcher_run(
    agent_state: State<'_, AgentState>,
    input: ResearchInput,
) -> Result<IpcResponse<ResearchReport>, AppError> {
    let report = run_research_report(&agent_state.engine, &input).await?;
    tracing::info!(
        query = %report.query,
        depth = %report.depth,
        claims = report.claims.len(),
        "Research report generated"
    );
    Ok(IpcResponse::ok(report))
}
