
use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

impl Database {
    pub fn get_stats(&self) -> Result<serde_json::Value, AppError> {
        let conn = self.conn()?;
        let prompt_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |row| row.get::<_, i64>(0))
            .map_err(db_err)?;
        let novel_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM novels", [], |row| row.get::<_, i64>(0))
            .map_err(db_err)?;
        let trend_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trends", [], |row| row.get::<_, i64>(0))
            .map_err(db_err)?;
        let total_words: i64 = conn
            .query_row("SELECT COALESCE(SUM(word_count), 0) FROM novels", [], |row| row.get::<_, i64>(0))
            .map_err(db_err)?;
        Ok(serde_json::json!({ "promptCount": prompt_count, "novelCount": novel_count, "trendCount": trend_count, "totalWords": total_words }))
    }

    pub fn get_daily_activity(&self) -> Result<serde_json::Value, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT DATE(created_at) as date, COUNT(*) FROM sessions WHERE created_at >= DATE('now', '-1 year') GROUP BY DATE(created_at) ORDER BY date"
        ).map_err(db_err)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).map_err(db_err)?;
        let chat_activity: Vec<(String, i64)> = rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        let chat_json: Vec<serde_json::Value> = chat_activity.into_iter()
            .map(|(date, count)| serde_json::json!({ "date": date, "count": count }))
            .collect();

        Ok(serde_json::json!({
            "chatActivity": chat_json,
            "novelActivity": []
        }))
    }

    /// AI 指标聚合:从 messages 表统计 token 总量、LLM 调用数、模型用量、工具调用数。
    ///
    /// 数据源:assistant 角色消息(由 AgentEngine 流式调用后写入,携带 model/
    /// provider/input_tokens/output_tokens/tool_calls 等字段)。
    pub fn get_ai_stats(&self) -> Result<serde_json::Value, AppError> {
        let conn = self.conn()?;

        // LLM 调用数 = assistant 消息中 model 非空的数量
        let llm_calls: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE role = 'assistant' AND model IS NOT NULL AND model != ''",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // token 总量 = sum(input_tokens + output_tokens) over assistant messages
        let total_tokens: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(input_tokens + output_tokens), 0) FROM messages WHERE role = 'assistant'",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        let input_tokens: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(input_tokens), 0) FROM messages WHERE role = 'assistant'",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        let output_tokens: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(output_tokens), 0) FROM messages WHERE role = 'assistant'",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // 工具调用数 = sum(json_array_length(tool_calls)) over messages with tool_calls
        // SQLite JSON1 提供 json_array_length;tool_calls 为 NULL 时记 0
        let tool_calls: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(json_array_length(tool_calls)), 0) FROM messages WHERE tool_calls IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // 模型用量分组:按 provider+model 聚合调用数与 token
        let mut stmt = conn.prepare_cached(
            "SELECT provider, model, COUNT(*) as calls, SUM(input_tokens) as input, SUM(output_tokens) as output
             FROM messages
             WHERE role = 'assistant' AND model IS NOT NULL AND model != ''
             GROUP BY provider, model
             ORDER BY calls DESC",
        ).map_err(db_err)?;
        let rows = stmt.query_map([], |row| {
            let provider: Option<String> = row.get(0)?;
            let model: String = row.get(1)?;
            let calls: i64 = row.get(2)?;
            let input: i64 = row.get(3)?;
            let output: i64 = row.get(4)?;
            Ok(serde_json::json!({
                "provider": provider,
                "model": model,
                "calls": calls,
                "inputTokens": input,
                "outputTokens": output,
                "totalTokens": input + output,
            }))
        }).map_err(db_err)?;
        let model_usage: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        Ok(serde_json::json!({
            "llmCalls": llm_calls,
            "totalTokens": total_tokens,
            "inputTokens": input_tokens,
            "outputTokens": output_tokens,
            "toolCalls": tool_calls,
            "modelUsage": model_usage,
        }))
    }
}