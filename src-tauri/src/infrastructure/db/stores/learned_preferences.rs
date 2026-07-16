// learned_preferences CRUD —— 从交互中自动学习的用户偏好存储。
//
// 置信度模型(对应 migration 注释):
// - occurrence_count < 3 → 0.2(探索期,不合并到 UserProfile)
// - occurrence_count >= 3 → 0.5(确认期)
// - occurrence_count >= 7 → 0.8(稳定期,可合并到 UserProfile)
// - last_seen_at 超过 30 天 → confidence *= 0.5(衰减)
//
// 与 user_profile 表的区别:
// - user_profile 是用户手动配置的静态偏好
// - learned_preferences 是 Agent 自动从对话中学习的偏好
// - 高置信度的 learned_preferences 通过 merge_to_user_profile() 合并

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

const UPSERT_SQL: &str = "\
INSERT INTO learned_preferences (\
    id, preference_key, preference_value, confidence, occurrence_count, \
    learned_from, last_seen_at, created_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
ON CONFLICT(preference_key, preference_value) DO UPDATE SET \
    occurrence_count = occurrence_count + 1, \
    confidence = MIN(1.0, CASE \
        WHEN occurrence_count + 1 >= 7 THEN 0.8 \
        WHEN occurrence_count + 1 >= 3 THEN 0.5 \
        ELSE 0.2 \
    END), \
    learned_from = excluded.learned_from, \
    last_seen_at = excluded.last_seen_at";

const SELECT_COLUMNS: &str = "\
id, preference_key, preference_value, confidence, occurrence_count, \
learned_from, last_seen_at, created_at";

/// 学习到的用户偏好行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPreferenceRow {
    pub id: String,
    pub preference_key: String,
    pub preference_value: String,
    pub confidence: f64,
    pub occurrence_count: u32,
    pub learned_from: Option<String>,
    pub last_seen_at: String,
    pub created_at: String,
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<LearnedPreferenceRow> {
    Ok(LearnedPreferenceRow {
        id: row.get(0)?,
        preference_key: row.get(1)?,
        preference_value: row.get(2)?,
        confidence: row.get(3)?,
        occurrence_count: row.get::<_, i64>(4)? as u32,
        learned_from: row.get(5)?,
        last_seen_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

/// 计算给定 occurrence_count 对应的置信度
///
/// 公式(对齐 migration 注释):
/// - < 3 → 0.2(探索期)
/// - 3..6 → 0.5(确认期)
/// - >= 7 → 0.8(稳定期)
pub fn confidence_for_count(count: u32) -> f64 {
    if count >= 7 {
        0.8
    } else if count >= 3 {
        0.5
    } else {
        0.2
    }
}

/// 简单的字符串哈希(用于生成 ID 后缀,保证唯一性)
///
/// 使用标准库 DefaultHasher(基于 SipHash-1-3 算法,非加密用途)
/// 不引入新依赖,只用于 ID 生成
fn fxhash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

impl Database {
    /// 记录一次偏好观察。
    ///
    /// 如果 (key, value) 已存在,occurrence_count +1,confidence 按公式上调;
    /// 如果不存在,插入新记录,confidence=0.2,count=1。
    ///
    /// learned_from 用于追溯来源(如 "session:abc123")。
    pub fn upsert_learned_preference(
        &self,
        preference_key: &str,
        preference_value: &str,
        learned_from: Option<&str>,
    ) -> Result<(), AppError> {
        let now_dt = chrono::Utc::now();
        let now = now_dt.to_rfc3339();
        // ID 唯一性:timestamp_nanos(在 Windows 上精度为 100ns 量级)
        // + key+value 的 hash,确保同毫秒内不同 (key,value) 对的 ID 不同
        let id = format!(
            "lp-{}-{}",
            now_dt.timestamp_nanos_opt().unwrap_or(0),
            fxhash(&format!("{}:{}", preference_key, preference_value))
        );
        let conn = self.conn()?;
        conn.execute(
            UPSERT_SQL,
            params![
                &id,
                preference_key,
                preference_value,
                0.2_f64,
                1_i64,
                learned_from,
                &now,
                &now,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 列出所有偏好(按 confidence 降序)
    pub fn list_learned_preferences(&self) -> Result<Vec<LearnedPreferenceRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM learned_preferences ORDER BY confidence DESC, last_seen_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按 key 列出偏好(同一 key 可能有多个 value,如 work_hours=09-18 / 20-23)
    pub fn list_learned_preferences_by_key(
        &self,
        key: &str,
    ) -> Result<Vec<LearnedPreferenceRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM learned_preferences WHERE preference_key = ?1 \
             ORDER BY confidence DESC, last_seen_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([key], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 列出高置信度偏好(>= threshold),用于合并到 UserProfile
    ///
    /// 默认 threshold = 0.7(稳定期 + 衰减后仍 >= 0.4 的偏好)
    pub fn list_high_confidence_preferences(
        &self,
        threshold: Option<f64>,
    ) -> Result<Vec<LearnedPreferenceRow>, AppError> {
        let t = threshold.unwrap_or(0.7);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM learned_preferences WHERE confidence >= ?1 \
             ORDER BY confidence DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map(params![t], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 删除指定偏好(用户主动否认)
    pub fn delete_learned_preference(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM learned_preferences WHERE id = ?1",
            params![id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 衰减长期未观察的偏好(用于定时任务,每晚调用一次)
    ///
    /// 规则:last_seen_at 早于 cutoff_iso 的偏好,confidence *= 0.5(下限 0.1)
    /// 返回受影响的行数
    pub fn decay_stale_preferences(&self, cutoff_iso: &str) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE learned_preferences SET confidence = MAX(0.1, confidence * 0.5) \
             WHERE last_seen_at < ?1 AND confidence > 0.1",
            params![cutoff_iso],
        ).map_err(db_err)?;
        Ok(affected as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    #[test]
    fn upsert_new_preference_starts_at_exploration() {
        let db = make_in_memory_db();
        db.upsert_learned_preference("code_style", "concise", Some("session:s1"))
            .unwrap();
        let all = db.list_learned_preferences().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].preference_key, "code_style");
        assert_eq!(all[0].preference_value, "concise");
        assert_eq!(all[0].occurrence_count, 1);
        assert!((all[0].confidence - 0.2).abs() < 0.01);
    }

    #[test]
    fn upsert_same_preference_increments_count_and_confidence() {
        let db = make_in_memory_db();
        // count=1 → confidence=0.2
        db.upsert_learned_preference("work_hours", "09-18", None).unwrap();
        // count=2 → 仍是 0.2(< 3)
        db.upsert_learned_preference("work_hours", "09-18", None).unwrap();
        // count=3 → 0.5
        db.upsert_learned_preference("work_hours", "09-18", None).unwrap();
        let all = db.list_learned_preferences().unwrap();
        assert_eq!(all[0].occurrence_count, 3);
        assert!((all[0].confidence - 0.5).abs() < 0.01);

        // 再加 4 次(count=7)→ 0.8
        for _ in 0..4 {
            db.upsert_learned_preference("work_hours", "09-18", None).unwrap();
        }
        let all = db.list_learned_preferences().unwrap();
        assert_eq!(all[0].occurrence_count, 7);
        assert!((all[0].confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn upsert_different_values_for_same_key_keeps_separate() {
        let db = make_in_memory_db();
        db.upsert_learned_preference("code_style", "concise", None).unwrap();
        db.upsert_learned_preference("code_style", "verbose", None).unwrap();
        let by_key = db.list_learned_preferences_by_key("code_style").unwrap();
        assert_eq!(by_key.len(), 2);
        assert!(by_key.iter().any(|r| r.preference_value == "concise"));
        assert!(by_key.iter().any(|r| r.preference_value == "verbose"));
    }

    #[test]
    fn list_high_confidence_filters() {
        let db = make_in_memory_db();
        // 只观察 1 次,confidence=0.2,不达 0.7
        db.upsert_learned_preference("low_pref", "x", None).unwrap();
        // 观察 7 次,confidence=0.8,达 0.7
        for _ in 0..7 {
            db.upsert_learned_preference("high_pref", "y", None).unwrap();
        }
        let high = db.list_high_confidence_preferences(Some(0.7)).unwrap();
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].preference_key, "high_pref");
    }

    #[test]
    fn decay_halves_confidence_for_stale() {
        let db = make_in_memory_db();
        // 观察 7 次,confidence=0.8
        for _ in 0..7 {
            db.upsert_learned_preference("stale_pref", "v", None).unwrap();
        }
        let before = db.list_learned_preferences().unwrap();
        assert!((before[0].confidence - 0.8).abs() < 0.01);

        // 模拟时间流逝:cutoff 设为未来时间,所有偏好都被视为 stale
        let future = chrono::Utc::now() + chrono::Duration::days(1);
        let affected = db.decay_stale_preferences(&future.to_rfc3339()).unwrap();
        assert_eq!(affected, 1);

        let after = db.list_learned_preferences().unwrap();
        // 0.8 * 0.5 = 0.4
        assert!((after[0].confidence - 0.4).abs() < 0.01);
    }

    #[test]
    fn delete_removes_preference() {
        let db = make_in_memory_db();
        db.upsert_learned_preference("to_delete", "v", None).unwrap();
        let all = db.list_learned_preferences().unwrap();
        assert_eq!(all.len(), 1);

        let deleted = db.delete_learned_preference(&all[0].id).unwrap();
        assert!(deleted);
        assert_eq!(db.list_learned_preferences().unwrap().len(), 0);
    }

    #[test]
    fn confidence_for_count_matches_formula() {
        assert!((confidence_for_count(1) - 0.2).abs() < 0.01);
        assert!((confidence_for_count(2) - 0.2).abs() < 0.01);
        assert!((confidence_for_count(3) - 0.5).abs() < 0.01);
        assert!((confidence_for_count(6) - 0.5).abs() < 0.01);
        assert!((confidence_for_count(7) - 0.8).abs() < 0.01);
        assert!((confidence_for_count(100) - 0.8).abs() < 0.01);
    }
}
