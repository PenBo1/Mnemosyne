use super::Database;
use super::connection::db_err;
use crate::shared::errors::AppError;

impl Database {
    pub async fn get_stats(&self) -> Result<serde_json::Value, AppError> {
        let prompt_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prompts")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        let novel_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM novels")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        let trend_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trends")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        let total_words: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(word_count), 0) FROM novels")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        Ok(serde_json::json!({ "promptCount": prompt_count, "novelCount": novel_count, "trendCount": trend_count, "totalWords": total_words }))
    }

    pub async fn get_daily_activity(&self) -> Result<serde_json::Value, AppError> {
        let chat_activity: Vec<(String, i64)> = sqlx::query_as(
            "SELECT DATE(created_at) as date, COUNT(*) FROM sessions WHERE created_at >= DATE('now', '-1 year') GROUP BY DATE(created_at) ORDER BY date"
        )
        .fetch_all(&self.pool).await.map_err(db_err)?;

        let chat_json: Vec<serde_json::Value> = chat_activity.into_iter()
            .map(|(date, count)| serde_json::json!({ "date": date, "count": count }))
            .collect();

        Ok(serde_json::json!({
            "chatActivity": chat_json,
            "novelActivity": []
        }))
    }
}
