//! ═══════════════════════════════════════════════════════════════════════════
//! Memory 服务 - 业务逻辑层
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 从 commands 中提取的业务逻辑，遵循分层架构原则。

use crate::infrastructure::db::connection::Database;
use crate::shared::error::AppError;
use super::dto::ShortTermMemoryStats;

// ── 短期记忆服务 ────────────────────────────────────────────────────────────

/// 短期记忆服务
pub struct ShortTermMemoryService;

impl ShortTermMemoryService {
    /// 获取短期记忆统计信息
    ///
    /// # 参数
    /// - `db`: 数据库实例
    ///
    /// # 返回值
    /// 返回统计信息（总数、今日数、最近 7 天数）
    pub fn get_stats(db: &Database) -> Result<ShortTermMemoryStats, AppError> {
        // 今日统计
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let today_count = db.list_short_term_by_date(&today)?.len() as u64;

        // 最近 7 天统计（包括今天）
        let start_date = chrono::Utc::now() - chrono::Duration::days(6);
        let start_str = start_date.format("%Y-%m-%d").to_string();
        let last_7 = db.list_short_term_by_date_range(&start_str, &today)?;
        let last_7_days_count = last_7.len() as u64;

        // 总数
        let total = db.count_all_short_term()?;

        Ok(ShortTermMemoryStats {
            total,
            today_count,
            last_7_days_count,
        })
    }

    /// 按日期查询短期记忆
    pub fn list_by_date(db: &Database, date: &str) -> Result<Vec<crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow>, AppError> {
        db.list_short_term_by_date(date).map_err(Into::into)
    }

    /// 按 session_id 查询短期记忆
    pub fn get_for_session(db: &Database, session_id: &str) -> Result<Option<crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow>, AppError> {
        db.get_short_term_for_session(session_id).map_err(Into::into)
    }

    /// 按 book_id 查询短期记忆
    pub fn list_by_book(db: &Database, book_id: &str, limit: i64) -> Result<Vec<crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow>, AppError> {
        // 上限 500 防止大表全扫
        let l = limit.min(500);
        db.list_short_term_by_book(book_id, l).map_err(Into::into)
    }

    /// 按日期范围查询短期记忆
    pub fn list_by_range(db: &Database, start_date: &str, end_date: &str) -> Result<Vec<crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow>, AppError> {
        db.list_short_term_by_date_range(start_date, end_date).map_err(Into::into)
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：由于需要数据库连接，统计测试在集成测试中进行
}