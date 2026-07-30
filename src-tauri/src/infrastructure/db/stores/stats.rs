//! ═══════════════════════════════════════════════════════════════════════════
//! 统计存储 - 全局统计与活动数据
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供三类统计：
//! - get_stats: 全局计数（prompt/novel/trend 数量）
//! - get_daily_activity: 每日会话活动
//! - get_ai_stats: AI 指标聚合（LLM 调用、token、模型用量）
//! - get_usage_stats: 使用统计（活跃天数、连续活跃、热力图）

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;
use rusqlite::OptionalExtension;

impl Database {
    /// 获取全局统计
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

    /// 获取每日活动
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

    /// 获取 AI 统计
    ///
    /// 从 messages 表统计 token 总量、LLM 调用数、模型用量、工具调用数。
    pub fn get_ai_stats(&self) -> Result<serde_json::Value, AppError> {
        let conn = self.conn()?;

        // LLM 调用数
        let llm_calls: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE role = 'assistant' AND model IS NOT NULL AND model != ''",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // Token 总量
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

        // 工具调用数
        let tool_calls: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(json_array_length(tool_calls)), 0) FROM messages WHERE tool_calls IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // 模型用量分组
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

    /// 获取使用统计
    ///
    /// 返回活跃天数、连续活跃天数、热力图数据、按天 token 趋势。
    pub fn get_usage_stats(&self, days: i64) -> Result<serde_json::Value, AppError> {
        let conn = self.conn()?;
        let days_clause = format!("DATE('now', '-{} days')", days);

        // 会话数量
        let session_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM sessions WHERE created_at >= {}", days_clause),
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // 消息数量
        let message_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM messages WHERE created_at >= {}", days_clause),
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // Token 总量
        let total_tokens: i64 = conn
            .query_row(
                &format!(
                    "SELECT COALESCE(SUM(input_tokens + output_tokens), 0) FROM messages 
                     WHERE role = 'assistant' AND created_at >= {}",
                    days_clause
                ),
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // 活跃天数
        let active_days: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(DISTINCT DATE(created_at)) FROM sessions WHERE created_at >= {}",
                    days_clause
                ),
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        // 当前连续活跃天数（优化：直接获取最近活跃日期，在 Rust 端计算连续天数）
        let mut active_dates_stmt = conn.prepare_cached(
            "SELECT DISTINCT DATE(created_at) as date
             FROM sessions
             WHERE created_at >= DATE('now', '-30 days')
             ORDER BY date DESC"
        ).map_err(db_err)?;
        let active_dates_rows = active_dates_stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(db_err)?;
        let active_dates: Vec<String> = active_dates_rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        // 计算连续活跃天数（从今天或昨天开始往前数）
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (chrono::Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
        let mut current_streak: i64 = 0;

        // 检查今天或昨天是否有活动
        if active_dates.contains(&today) || active_dates.contains(&yesterday) {
            let start_date = if active_dates.contains(&today) {
                chrono::Local::now().date_naive()
            } else {
                (chrono::Local::now() - chrono::Duration::days(1)).date_naive()
            };

            // 遍历计算连续天数
            let mut check_date = start_date;
            let mut streak_count = 0;
            loop {
                let date_str = check_date.format("%Y-%m-%d").to_string();
                if active_dates.contains(&date_str) {
                    streak_count += 1;
                    check_date -= chrono::Duration::days(1);
                } else {
                    break;
                }
                // 最多计算 30 天
                if streak_count >= 30 {
                    break;
                }
            }
            current_streak = streak_count;
        }

        // 热力图数据
        let mut heatmap_stmt = conn.prepare_cached(&format!(
            "SELECT DATE(s.created_at) as date, COUNT(*) as count
             FROM sessions s
             WHERE s.created_at >= {}
             GROUP BY DATE(s.created_at)
             ORDER BY date",
            days_clause
        )).map_err(db_err)?;
        let heatmap_rows = heatmap_stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "date": row.get::<_, String>(0)?,
                    "count": row.get::<_, i64>(1)?,
                }))
            })
            .map_err(db_err)?;
        let heatmap: Vec<serde_json::Value> = heatmap_rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        // 按天 Token 趋势
        let mut trend_stmt = conn.prepare_cached(&format!(
            "SELECT DATE(created_at) as date, 
                    COALESCE(SUM(input_tokens + output_tokens), 0) as tokens
             FROM messages
             WHERE role = 'assistant' AND created_at >= {}
             GROUP BY DATE(created_at)
             ORDER BY date",
            days_clause
        )).map_err(db_err)?;
        let trend_rows = trend_stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "date": row.get::<_, String>(0)?,
                    "tokens": row.get::<_, i64>(1)?,
                }))
            })
            .map_err(db_err)?;
        let token_trend: Vec<serde_json::Value> = trend_rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        // 最常用模型
        let most_used_model: serde_json::Value = conn
            .query_row(
                &format!(
                    "SELECT model, COUNT(*) as calls, SUM(input_tokens + output_tokens) as tokens
                     FROM messages
                     WHERE role = 'assistant' AND model IS NOT NULL AND model != '' AND created_at >= {}
                     GROUP BY model
                     ORDER BY calls DESC
                     LIMIT 1",
                    days_clause
                ),
                [],
                |row| {
                    let model: String = row.get(0)?;
                    let calls: i64 = row.get(1)?;
                    let tokens: i64 = row.get(2)?;
                    Ok(serde_json::json!({
                        "model": model,
                        "calls": calls,
                        "tokens": tokens,
                    }))
                },
            )
            .optional()
            .map_err(db_err)?
            .unwrap_or(serde_json::json!({ "model": null, "calls": 0, "tokens": 0 }));

        // 模型占比计算
        let model_usage_with_ratio: Vec<serde_json::Value> = {
            let total_model_tokens: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COALESCE(SUM(input_tokens + output_tokens), 0) FROM messages 
                         WHERE role = 'assistant' AND model IS NOT NULL AND model != '' AND created_at >= {}",
                        days_clause
                    ),
                    [],
                    |row| row.get(0),
                )
                .map_err(db_err)?;

            let mut stmt = conn.prepare_cached(&format!(
                "SELECT provider, model, COUNT(*) as calls, SUM(input_tokens) as input, SUM(output_tokens) as output
                 FROM messages
                 WHERE role = 'assistant' AND model IS NOT NULL AND model != '' AND created_at >= {}
                 GROUP BY provider, model
                 ORDER BY calls DESC",
                days_clause
            )).map_err(db_err)?;
            let rows = stmt.query_map([], |row| {
                let provider: Option<String> = row.get(0)?;
                let model: String = row.get(1)?;
                let calls: i64 = row.get(2)?;
                let input: i64 = row.get(3)?;
                let output: i64 = row.get(4)?;
                let total = input + output;
                let ratio = if total_model_tokens > 0 {
                    (total as f64 / total_model_tokens as f64 * 100.0).round() as i64
                } else {
                    0
                };
                Ok(serde_json::json!({
                    "provider": provider,
                    "model": model,
                    "calls": calls,
                    "inputTokens": input,
                    "outputTokens": output,
                    "totalTokens": total,
                    "ratio": ratio,
                }))
            }).map_err(db_err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?
        };

        Ok(serde_json::json!({
            "activeDays": active_days,
            "currentStreak": current_streak,
            "heatmap": heatmap,
            "tokenTrend": token_trend,
            "sessionCount": session_count,
            "messageCount": message_count,
            "totalTokens": total_tokens,
            "mostUsedModel": most_used_model,
            "modelUsage": model_usage_with_ratio,
        }))
    }
}