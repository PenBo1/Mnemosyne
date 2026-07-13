// 审计事件持久化:audit_events 表的读写方法。

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

/// 一行审计事件(前端展示用)。payload 为完整 SecurityEvent JSON。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventRow {
    pub id: String,
    pub event_type: String,
    pub operation: Option<String>,
    pub workspace_id: Option<String>,
    pub is_denied: bool,
    pub is_security_related: bool,
    pub payload: serde_json::Value,
    pub recorded_at: String,
}

/// 审计事件聚合统计(供仪表盘 Violations 卡片)。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventStats {
    pub total: i64,
    pub denied: i64,
    pub security_related: i64,
    pub by_type: Vec<AuditTypeCount>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditTypeCount {
    pub event_type: String,
    pub count: i64,
}

/// 插入审计事件。payload 为完整 SecurityEvent 序列化后的 JSON 字符串。
pub fn insert_audit_event(
    db: &Database,
    id: &str,
    event_type: &str,
    operation: Option<&str>,
    workspace_id: Option<&str>,
    is_denied: bool,
    is_security_related: bool,
    payload: &str,
    recorded_at: &str,
) -> Result<(), AppError> {
    let conn = db.conn()?;
    conn.execute(
        "INSERT INTO audit_events (id, event_type, operation, workspace_id, is_denied, is_security_related, payload, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            id,
            event_type,
            operation,
            workspace_id,
            is_denied as i64,
            is_security_related as i64,
            payload,
            recorded_at,
        ],
    )
    .map_err(db_err)?;
    Ok(())
}

impl Database {
    /// 查询审计事件(按 recorded_at 倒序)。limit 上限 1000。
    pub fn query_audit_events(&self, limit: i64) -> Result<Vec<AuditEventRow>, AppError> {
        let limit = limit.clamp(1, 1000);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, event_type, operation, workspace_id, is_denied, is_security_related, payload, recorded_at
             FROM audit_events
             ORDER BY recorded_at DESC
             LIMIT ?1",
        ).map_err(db_err)?;
        let rows = stmt.query_map([limit], |row| {
            let payload_str: String = row.get(6)?;
            let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
            Ok(AuditEventRow {
                id: row.get(0)?,
                event_type: row.get(1)?,
                operation: row.get(2)?,
                workspace_id: row.get(3)?,
                is_denied: row.get::<_, i64>(4)? != 0,
                is_security_related: row.get::<_, i64>(5)? != 0,
                payload,
                recorded_at: row.get(7)?,
            })
        }).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    /// 审计事件聚合统计:总数 / 拒绝数 / 安全相关数 / 按类型分组。
    pub fn audit_event_stats(&self) -> Result<AuditEventStats, AppError> {
        let conn = self.conn()?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .map_err(db_err)?;
        let denied: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events WHERE is_denied = 1", [], |row| row.get(0))
            .map_err(db_err)?;
        let security_related: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events WHERE is_security_related = 1", [], |row| row.get(0))
            .map_err(db_err)?;

        let mut stmt = conn.prepare_cached(
            "SELECT event_type, COUNT(*) FROM audit_events GROUP BY event_type ORDER BY COUNT(*) DESC",
        ).map_err(db_err)?;
        let rows = stmt.query_map([], |row| {
            Ok(AuditTypeCount {
                event_type: row.get(0)?,
                count: row.get(1)?,
            })
        }).map_err(db_err)?;
        let by_type = rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        Ok(AuditEventStats { total, denied, security_related, by_type })
    }
}
