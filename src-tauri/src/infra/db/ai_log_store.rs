use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::Database;
use crate::errors::AppError;

const AI_LOGS_SCHEMA: &str = include_str!("sql/ai_logs_schema.sql");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCall {
    pub id: String,
    pub session_id: String,
    pub agent_role: String,
    pub model: String,
    pub provider: String,
    pub system_prompt: Option<String>,
    pub messages_json: String,
    pub tools_json: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub response_content: Option<String>,
    pub response_tool_calls: Option<String>,
    pub finish_reason: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub latency_ms: Option<u64>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: String,
    pub session_id: String,
    pub llm_call_id: Option<String>,
    pub tool_name: String,
    pub arguments_json: String,
    pub result_content: Option<String>,
    pub is_error: bool,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub sandbox_allowed: bool,
    pub sandbox_violation: Option<String>,
    pub pve_blocked: bool,
    pub metadata: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentThinking {
    pub id: String,
    pub session_id: String,
    pub llm_call_id: Option<String>,
    pub thinking_content: String,
    pub thinking_level: Option<String>,
    pub thinking_tokens: u32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageLog {
    pub id: String,
    pub pipeline_run_id: String,
    pub novel_id: String,
    pub chapter_number: u32,
    pub stage_name: String,
    pub agent_role: String,
    pub model: Option<String>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub audit_score: Option<f64>,
    pub gate_passed: Option<bool>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub recovery_strategy: Option<String>,
    pub metadata: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxViolation {
    pub id: String,
    pub session_id: Option<String>,
    pub violation_type: String,
    pub resource: String,
    pub action: String,
    pub rule_matched: Option<String>,
    pub tool_name: Option<String>,
    pub arguments_json: Option<String>,
    pub detected_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperation {
    pub id: String,
    pub session_id: Option<String>,
    pub book_id: String,
    pub operation: String,
    pub entry_type: Option<String>,
    pub chapter: Option<u32>,
    pub content_preview: Option<String>,
    pub search_query: Option<String>,
    pub search_results_count: Option<u32>,
    pub performed_at: String,
    pub created_at: String,
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::internal(format!("Database error: {}", e))
}

impl Database {
    pub fn init_ai_logs(&self) -> Result<(), AppError> {
        let pool = self.pool.clone();
        let schema = AI_LOGS_SCHEMA.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                sqlx::raw_sql(&schema).execute(&pool).await.map_err(db_err)?;
                Ok::<(), AppError>(())
            })
        })
    }

    pub async fn log_llm_call_start(
        &self,
        session_id: &str,
        agent_role: &str,
        model: &str,
        provider: &str,
        system_prompt: Option<&str>,
        messages_json: &str,
        tools_json: Option<&str>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO llm_calls (id, session_id, agent_role, model, provider, system_prompt, messages_json, tools_json, temperature, max_tokens, input_tokens, output_tokens, cache_read_tokens, started_at, status, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, ?, 'running', '{}', ?)"
        )
        .bind(&id).bind(session_id).bind(agent_role).bind(model).bind(provider)
        .bind(system_prompt).bind(messages_json).bind(tools_json)
        .bind(temperature).bind(max_tokens).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }

    pub async fn log_llm_call_complete(
        &self,
        id: &str,
        response_content: Option<&str>,
        response_tool_calls: Option<&str>,
        finish_reason: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
        latency_ms: u64,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE llm_calls SET response_content = ?, response_tool_calls = ?, finish_reason = ?, input_tokens = ?, output_tokens = ?, completed_at = ?, latency_ms = ?, status = 'completed' WHERE id = ?"
        )
        .bind(response_content).bind(response_tool_calls).bind(finish_reason)
        .bind(input_tokens).bind(output_tokens).bind(&now).bind(latency_ms as i64).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    pub async fn log_llm_call_error(&self, id: &str, error_message: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE llm_calls SET error_message = ?, completed_at = ?, status = 'failed' WHERE id = ?"
        )
        .bind(error_message).bind(&now).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    pub async fn get_llm_calls(&self, session_id: &str, limit: u32) -> Result<Vec<LlmCall>, AppError> {
        sqlx::query(
            "SELECT id, session_id, agent_role, model, provider, system_prompt, messages_json, tools_json, temperature, max_tokens, response_content, response_tool_calls, finish_reason, input_tokens, output_tokens, cache_read_tokens, started_at, completed_at, latency_ms, status, error_message, metadata, created_at FROM llm_calls WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(session_id).bind(limit)
        .map(|row: sqlx::sqlite::SqliteRow| LlmCall {
            id: row.get(0usize), session_id: row.get(1usize), agent_role: row.get(2usize),
            model: row.get(3usize), provider: row.get(4usize), system_prompt: row.get(5usize),
            messages_json: row.get(6usize), tools_json: row.get(7usize), temperature: row.get(8usize),
            max_tokens: row.get(9usize), response_content: row.get(10usize),
            response_tool_calls: row.get(11usize), finish_reason: row.get(12usize),
            input_tokens: row.get(13usize), output_tokens: row.get(14usize),
            cache_read_tokens: row.get(15usize), started_at: row.get(16usize),
            completed_at: row.get(17usize), latency_ms: row.get(18usize), status: row.get(19usize),
            error_message: row.get(20usize), metadata: row.get(21usize), created_at: row.get(22usize),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn log_tool_execution_start(
        &self,
        session_id: &str,
        llm_call_id: Option<&str>,
        tool_name: &str,
        arguments_json: &str,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO tool_executions (id, session_id, llm_call_id, tool_name, arguments_json, sandbox_allowed, pve_blocked, metadata, created_at) VALUES (?, ?, ?, ?, ?, 1, 0, '{}', ?)"
        )
        .bind(&id).bind(session_id).bind(llm_call_id).bind(tool_name).bind(arguments_json).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }

    pub async fn log_tool_execution_complete(
        &self,
        id: &str,
        result_content: Option<&str>,
        is_error: bool,
        error_message: Option<&str>,
        duration_ms: u64,
        sandbox_allowed: bool,
        sandbox_violation: Option<&str>,
        pve_blocked: bool,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tool_executions SET result_content = ?, is_error = ?, error_message = ?, completed_at = ?, duration_ms = ?, sandbox_allowed = ?, sandbox_violation = ?, pve_blocked = ? WHERE id = ?"
        )
        .bind(result_content).bind(is_error as i32).bind(error_message).bind(&now)
        .bind(duration_ms as i64).bind(sandbox_allowed as i32).bind(sandbox_violation)
        .bind(pve_blocked as i32).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    pub async fn get_tool_executions(&self, session_id: &str, limit: u32) -> Result<Vec<ToolExecution>, AppError> {
        sqlx::query(
            "SELECT id, session_id, llm_call_id, tool_name, arguments_json, result_content, is_error, error_message, started_at, completed_at, duration_ms, sandbox_allowed, sandbox_violation, pve_blocked, metadata, created_at FROM tool_executions WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(session_id).bind(limit)
        .map(|row: sqlx::sqlite::SqliteRow| ToolExecution {
            id: row.get(0usize), session_id: row.get(1usize), llm_call_id: row.get(2usize),
            tool_name: row.get(3usize), arguments_json: row.get(4usize), result_content: row.get(5usize),
            is_error: row.get::<i32, usize>(6) != 0, error_message: row.get(7usize),
            started_at: row.get(8usize), completed_at: row.get(9usize), duration_ms: row.get(10usize),
            sandbox_allowed: row.get::<i32, usize>(11) != 0, sandbox_violation: row.get(12usize),
            pve_blocked: row.get::<i32, usize>(13) != 0, metadata: row.get(14usize), created_at: row.get(15usize),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn log_thinking(
        &self,
        session_id: &str,
        llm_call_id: Option<&str>,
        thinking_content: &str,
        thinking_level: Option<&str>,
        thinking_tokens: u32,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO agent_thinking (id, session_id, llm_call_id, thinking_content, thinking_level, thinking_tokens, started_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(session_id).bind(llm_call_id).bind(thinking_content)
        .bind(thinking_level).bind(thinking_tokens).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }

    pub async fn log_pipeline_stage_start(
        &self,
        pipeline_run_id: &str,
        novel_id: &str,
        chapter_number: u32,
        stage_name: &str,
        agent_role: &str,
        model: Option<&str>,
        input_summary: Option<&str>,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO pipeline_stage_logs (id, pipeline_run_id, novel_id, chapter_number, stage_name, agent_role, model, status, started_at, input_summary, input_tokens, output_tokens, retry_count, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, 'running', ?, ?, 0, 0, 0, '{}', ?)"
        )
        .bind(&id).bind(pipeline_run_id).bind(novel_id).bind(chapter_number as i64)
        .bind(stage_name).bind(agent_role).bind(model).bind(&now).bind(input_summary).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }

    pub async fn log_pipeline_stage_complete(
        &self,
        id: &str,
        output_summary: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
        audit_score: Option<f64>,
        gate_passed: Option<bool>,
        duration_ms: u64,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE pipeline_stage_logs SET output_summary = ?, input_tokens = ?, output_tokens = ?, audit_score = ?, gate_passed = ?, completed_at = ?, duration_ms = ?, status = 'completed' WHERE id = ?"
        )
        .bind(output_summary).bind(input_tokens).bind(output_tokens).bind(audit_score)
        .bind(gate_passed.map(|v| v as i32)).bind(&now).bind(duration_ms as i64).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    pub async fn log_pipeline_stage_error(
        &self,
        id: &str,
        error_message: &str,
        retry_count: u32,
        recovery_strategy: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE pipeline_stage_logs SET error_message = ?, retry_count = ?, recovery_strategy = ?, completed_at = ?, status = 'failed' WHERE id = ?"
        )
        .bind(error_message).bind(retry_count).bind(recovery_strategy).bind(&now).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    pub async fn log_sandbox_violation(
        &self,
        session_id: Option<&str>,
        violation_type: &str,
        resource: &str,
        action: &str,
        rule_matched: Option<&str>,
        tool_name: Option<&str>,
        arguments_json: Option<&str>,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO sandbox_violations (id, session_id, violation_type, resource, action, rule_matched, tool_name, arguments_json, detected_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(session_id).bind(violation_type).bind(resource).bind(action)
        .bind(rule_matched).bind(tool_name).bind(arguments_json).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }

    pub async fn log_memory_operation(
        &self,
        session_id: Option<&str>,
        book_id: &str,
        operation: &str,
        entry_type: Option<&str>,
        chapter: Option<u32>,
        content_preview: Option<&str>,
        search_query: Option<&str>,
        search_results_count: Option<u32>,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO memory_operations (id, session_id, book_id, operation, entry_type, chapter, content_preview, search_query, search_results_count, performed_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(session_id).bind(book_id).bind(operation).bind(entry_type)
        .bind(chapter.map(|c| c as i64)).bind(content_preview).bind(search_query)
        .bind(search_results_count.map(|c| c as i64)).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(id)
    }

    pub async fn get_session_token_usage(&self, session_id: &str) -> Result<(u32, u32), AppError> {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(input_tokens), 0) as inp, COALESCE(SUM(output_tokens), 0) as out FROM llm_calls WHERE session_id = ?"
        )
        .bind(session_id)
        .fetch_one(&self.pool).await.map_err(db_err)?;
        Ok((row.get(0usize), row.get(1usize)))
    }

    pub async fn get_session_tool_stats(&self, session_id: &str) -> Result<serde_json::Value, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_executions WHERE session_id = ?")
            .bind(session_id).fetch_one(&self.pool).await.map_err(db_err)?;

        let errors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_executions WHERE session_id = ? AND is_error = 1")
            .bind(session_id).fetch_one(&self.pool).await.map_err(db_err)?;

        let sandbox_blocked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_executions WHERE session_id = ? AND sandbox_allowed = 0")
            .bind(session_id).fetch_one(&self.pool).await.map_err(db_err)?;

        Ok(serde_json::json!({
            "total_calls": total,
            "errors": errors,
            "sandbox_blocked": sandbox_blocked,
            "success_rate": if total > 0 { (total - errors) as f64 / total as f64 } else { 1.0 },
        }))
    }

    pub async fn get_model_usage_stats(&self, session_id: &str) -> Result<Vec<serde_json::Value>, AppError> {
        sqlx::query(
            "SELECT model, COUNT(*) as calls, SUM(input_tokens) as input, SUM(output_tokens) as output, AVG(latency_ms) as avg_latency FROM llm_calls WHERE session_id = ? GROUP BY model ORDER BY calls DESC"
        )
        .bind(session_id)
        .map(|row: sqlx::sqlite::SqliteRow| {
            serde_json::json!({
                "model": row.get::<String, usize>(0),
                "calls": row.get::<i64, usize>(1),
                "input_tokens": row.get::<i64, usize>(2),
                "output_tokens": row.get::<i64, usize>(3),
                "avg_latency_ms": row.try_get::<Option<f64>, usize>(4).unwrap_or(None),
            })
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_sandbox_violations(&self, session_id: &str, limit: u32) -> Result<Vec<SandboxViolation>, AppError> {
        sqlx::query(
            "SELECT id, session_id, violation_type, resource, action, rule_matched, tool_name, arguments_json, detected_at, created_at FROM sandbox_violations WHERE session_id = ? ORDER BY detected_at DESC LIMIT ?"
        )
        .bind(session_id).bind(limit)
        .map(|row: sqlx::sqlite::SqliteRow| SandboxViolation {
            id: row.get(0usize), session_id: row.get(1usize), violation_type: row.get(2usize),
            resource: row.get(3usize), action: row.get(4usize), rule_matched: row.get(5usize),
            tool_name: row.get(6usize), arguments_json: row.get(7usize),
            detected_at: row.get(8usize), created_at: row.get(9usize),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }
}
