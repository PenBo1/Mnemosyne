use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::Database;
use crate::errors::AppError;

const AI_LOGS_SCHEMA: &str = include_str!("sql/ai_logs_schema.sql");

// ── Types ──────────────────────────────────────────────────

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

// ── Store ──────────────────────────────────────────────────

impl Database {
    /// Initialize AI logs schema
    pub fn init_ai_logs(&self) -> Result<(), AppError> {
        self.conn.execute_batch(AI_LOGS_SCHEMA)
            .map_err(|e| AppError::internal(format!("Failed to init AI logs schema: {}", e)))?;
        Ok(())
    }

    // ── LLM Calls ──────────────────────────────────────────

    pub fn log_llm_call_start(
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
        self.conn.execute(
            "INSERT INTO llm_calls (id, session_id, agent_role, model, provider, system_prompt, messages_json, tools_json, temperature, max_tokens, input_tokens, output_tokens, cache_read_tokens, started_at, status, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, 0, ?11, 'running', '{}', ?11)",
            params![id, session_id, agent_role, model, provider, system_prompt, messages_json, tools_json, temperature, max_tokens, now],
        ).map_err(|e| AppError::internal(format!("Failed to log LLM call: {}", e)))?;
        Ok(id)
    }

    pub fn log_llm_call_complete(
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
        self.conn.execute(
            "UPDATE llm_calls SET response_content = ?1, response_tool_calls = ?2, finish_reason = ?3, input_tokens = ?4, output_tokens = ?5, completed_at = ?6, latency_ms = ?7, status = 'completed' WHERE id = ?8",
            params![response_content, response_tool_calls, finish_reason, input_tokens, output_tokens, now, latency_ms, id],
        ).map_err(|e| AppError::internal(format!("Failed to complete LLM call: {}", e)))?;
        Ok(())
    }

    pub fn log_llm_call_error(
        &self,
        id: &str,
        error_message: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE llm_calls SET error_message = ?1, completed_at = ?2, status = 'failed' WHERE id = ?3",
            params![error_message, now, id],
        ).map_err(|e| AppError::internal(format!("Failed to log LLM error: {}", e)))?;
        Ok(())
    }

    pub fn get_llm_calls(&self, session_id: &str, limit: u32) -> Result<Vec<LlmCall>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, agent_role, model, provider, system_prompt, messages_json, tools_json, temperature, max_tokens, response_content, response_tool_calls, finish_reason, input_tokens, output_tokens, cache_read_tokens, started_at, completed_at, latency_ms, status, error_message, metadata, created_at FROM llm_calls WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map(params![session_id, limit], |row| {
            Ok(LlmCall {
                id: row.get(0)?,
                session_id: row.get(1)?,
                agent_role: row.get(2)?,
                model: row.get(3)?,
                provider: row.get(4)?,
                system_prompt: row.get(5)?,
                messages_json: row.get(6)?,
                tools_json: row.get(7)?,
                temperature: row.get(8)?,
                max_tokens: row.get(9)?,
                response_content: row.get(10)?,
                response_tool_calls: row.get(11)?,
                finish_reason: row.get(12)?,
                input_tokens: row.get(13)?,
                output_tokens: row.get(14)?,
                cache_read_tokens: row.get(15)?,
                started_at: row.get(16)?,
                completed_at: row.get(17)?,
                latency_ms: row.get(18)?,
                status: row.get(19)?,
                error_message: row.get(20)?,
                metadata: row.get(21)?,
                created_at: row.get(22)?,
            })
        }).map_err(|e| AppError::internal(format!("Failed to query LLM calls: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect LLM calls: {}", e)))
    }

    // ── Tool Executions ─────────────────────────────────────

    pub fn log_tool_execution_start(
        &self,
        session_id: &str,
        llm_call_id: Option<&str>,
        tool_name: &str,
        arguments_json: &str,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO tool_executions (id, session_id, llm_call_id, tool_name, arguments_json, sandbox_allowed, pve_blocked, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, 0, '{}', ?6)",
            params![id, session_id, llm_call_id, tool_name, arguments_json, now],
        ).map_err(|e| AppError::internal(format!("Failed to log tool execution: {}", e)))?;
        Ok(id)
    }

    pub fn log_tool_execution_complete(
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
        self.conn.execute(
            "UPDATE tool_executions SET result_content = ?1, is_error = ?2, error_message = ?3, completed_at = ?4, duration_ms = ?5, sandbox_allowed = ?6, sandbox_violation = ?7, pve_blocked = ?8 WHERE id = ?9",
            params![result_content, is_error as i32, error_message, now, duration_ms, sandbox_allowed as i32, sandbox_violation, pve_blocked as i32, id],
        ).map_err(|e| AppError::internal(format!("Failed to complete tool execution: {}", e)))?;
        Ok(())
    }

    pub fn get_tool_executions(&self, session_id: &str, limit: u32) -> Result<Vec<ToolExecution>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, llm_call_id, tool_name, arguments_json, result_content, is_error, error_message, started_at, completed_at, duration_ms, sandbox_allowed, sandbox_violation, pve_blocked, metadata, created_at FROM tool_executions WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map(params![session_id, limit], |row| {
            Ok(ToolExecution {
                id: row.get(0)?,
                session_id: row.get(1)?,
                llm_call_id: row.get(2)?,
                tool_name: row.get(3)?,
                arguments_json: row.get(4)?,
                result_content: row.get(5)?,
                is_error: row.get::<_, i32>(6)? != 0,
                error_message: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
                duration_ms: row.get(10)?,
                sandbox_allowed: row.get::<_, i32>(11)? != 0,
                sandbox_violation: row.get(12)?,
                pve_blocked: row.get::<_, i32>(13)? != 0,
                metadata: row.get(14)?,
                created_at: row.get(15)?,
            })
        }).map_err(|e| AppError::internal(format!("Failed to query tool executions: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect tool executions: {}", e)))
    }

    // ── Agent Thinking ──────────────────────────────────────

    pub fn log_thinking(
        &self,
        session_id: &str,
        llm_call_id: Option<&str>,
        thinking_content: &str,
        thinking_level: Option<&str>,
        thinking_tokens: u32,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO agent_thinking (id, session_id, llm_call_id, thinking_content, thinking_level, thinking_tokens, started_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, session_id, llm_call_id, thinking_content, thinking_level, thinking_tokens, now],
        ).map_err(|e| AppError::internal(format!("Failed to log thinking: {}", e)))?;
        Ok(id)
    }

    // ── Pipeline Stage Logs ─────────────────────────────────

    pub fn log_pipeline_stage_start(
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
        self.conn.execute(
            "INSERT INTO pipeline_stage_logs (id, pipeline_run_id, novel_id, chapter_number, stage_name, agent_role, model, status, started_at, input_summary, input_tokens, output_tokens, retry_count, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running', ?8, ?9, 0, 0, 0, '{}', ?8)",
            params![id, pipeline_run_id, novel_id, chapter_number, stage_name, agent_role, model, now, input_summary],
        ).map_err(|e| AppError::internal(format!("Failed to log pipeline stage: {}", e)))?;
        Ok(id)
    }

    pub fn log_pipeline_stage_complete(
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
        self.conn.execute(
            "UPDATE pipeline_stage_logs SET output_summary = ?1, input_tokens = ?2, output_tokens = ?3, audit_score = ?4, gate_passed = ?5, completed_at = ?6, duration_ms = ?7, status = 'completed' WHERE id = ?8",
            params![output_summary, input_tokens, output_tokens, audit_score, gate_passed.map(|v| v as i32), now, duration_ms, id],
        ).map_err(|e| AppError::internal(format!("Failed to complete pipeline stage: {}", e)))?;
        Ok(())
    }

    pub fn log_pipeline_stage_error(
        &self,
        id: &str,
        error_message: &str,
        retry_count: u32,
        recovery_strategy: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE pipeline_stage_logs SET error_message = ?1, retry_count = ?2, recovery_strategy = ?3, completed_at = ?4, status = 'failed' WHERE id = ?5",
            params![error_message, retry_count, recovery_strategy, now, id],
        ).map_err(|e| AppError::internal(format!("Failed to log pipeline stage error: {}", e)))?;
        Ok(())
    }

    // ── Sandbox Violations ──────────────────────────────────

    pub fn log_sandbox_violation(
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
        self.conn.execute(
            "INSERT INTO sandbox_violations (id, session_id, violation_type, resource, action, rule_matched, tool_name, arguments_json, detected_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![id, session_id, violation_type, resource, action, rule_matched, tool_name, arguments_json, now],
        ).map_err(|e| AppError::internal(format!("Failed to log sandbox violation: {}", e)))?;
        Ok(id)
    }

    // ── Memory Operations ───────────────────────────────────

    pub fn log_memory_operation(
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
        self.conn.execute(
            "INSERT INTO memory_operations (id, session_id, book_id, operation, entry_type, chapter, content_preview, search_query, search_results_count, performed_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![id, session_id, book_id, operation, entry_type, chapter, content_preview, search_query, search_results_count, now],
        ).map_err(|e| AppError::internal(format!("Failed to log memory operation: {}", e)))?;
        Ok(id)
    }

    // ── Analytics ───────────────────────────────────────────

    pub fn get_session_token_usage(&self, session_id: &str) -> Result<(u32, u32), AppError> {
        let result = self.conn.query_row(
            "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM llm_calls WHERE session_id = ?1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| AppError::internal(format!("Failed to get token usage: {}", e)))?;
        Ok(result)
    }

    pub fn get_session_tool_stats(&self, session_id: &str) -> Result<serde_json::Value, AppError> {
        let total: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_executions WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        ).map_err(|e| AppError::internal(format!("Failed to count tools: {}", e)))?;

        let errors: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_executions WHERE session_id = ?1 AND is_error = 1",
            params![session_id],
            |row| row.get(0),
        ).map_err(|e| AppError::internal(format!("Failed to count tool errors: {}", e)))?;

        let sandbox_blocked: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_executions WHERE session_id = ?1 AND sandbox_allowed = 0",
            params![session_id],
            |row| row.get(0),
        ).map_err(|e| AppError::internal(format!("Failed to count sandbox blocks: {}", e)))?;

        Ok(serde_json::json!({
            "total_calls": total,
            "errors": errors,
            "sandbox_blocked": sandbox_blocked,
            "success_rate": if total > 0 { (total - errors) as f64 / total as f64 } else { 1.0 },
        }))
    }

    pub fn get_model_usage_stats(&self, session_id: &str) -> Result<Vec<serde_json::Value>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT model, COUNT(*) as calls, SUM(input_tokens) as input, SUM(output_tokens) as output, AVG(latency_ms) as avg_latency FROM llm_calls WHERE session_id = ?1 GROUP BY model ORDER BY calls DESC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(serde_json::json!({
                "model": row.get::<_, String>(0)?,
                "calls": row.get::<_, i64>(1)?,
                "input_tokens": row.get::<_, i64>(2)?,
                "output_tokens": row.get::<_, i64>(3)?,
                "avg_latency_ms": row.get::<_, Option<f64>>(4)?,
            }))
        }).map_err(|e| AppError::internal(format!("Failed to query model stats: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect model stats: {}", e)))
    }
}
