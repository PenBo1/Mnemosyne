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

/// 审计事件过滤查询参数(对齐前端 AuditFilter)。
///
/// 所有字段都是 Option —— None 表示不应用该过滤。
/// 时间字段使用 RFC3339 字符串比较(SQL TEXT 比较 ISO8601 排序正确)。
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventFilter {
    pub workspace_id: Option<String>,
    pub operation: Option<String>,
    pub event_type: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub only_denied: Option<bool>,
    pub only_security: Option<bool>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

/// 直方图桶(按时间聚合)。bucket 为桶起始时间(RFC3339),count 为该桶事件数。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditHistogramBucket {
    pub bucket: String,
    pub count: i64,
    pub denied: i64,
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

    /// 按过滤条件查询审计事件。
    ///
    /// 动态拼装 WHERE 子句 + 参数绑定。
    /// 所有字符串字段使用 LIKE 模糊匹配(operation 便于按前缀查 fs_/git_ 等),
    /// except workspace_id 和 event_type 用精确匹配。
    /// limit 默认 50,上限 1000;offset 默认 0。
    pub fn query_audit_events_filtered(
        &self,
        filter: &AuditEventFilter,
    ) -> Result<Vec<AuditEventRow>, AppError> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut sql = String::from(
            "SELECT id, event_type, operation, workspace_id, is_denied, is_security_related, payload, recorded_at
             FROM audit_events WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;

        if let Some(ws) = &filter.workspace_id {
            sql.push_str(&format!(" AND workspace_id = ?{idx}"));
            params.push(Box::new(ws.clone()));
            idx += 1;
        }
        if let Some(op) = &filter.operation {
            sql.push_str(&format!(" AND operation LIKE ?{idx}"));
            params.push(Box::new(format!("%{op}%")));
            idx += 1;
        }
        if let Some(et) = &filter.event_type {
            sql.push_str(&format!(" AND event_type = ?{idx}"));
            params.push(Box::new(et.clone()));
            idx += 1;
        }
        if let Some(since) = &filter.since {
            sql.push_str(&format!(" AND recorded_at >= ?{idx}"));
            params.push(Box::new(since.clone()));
            idx += 1;
        }
        if let Some(until) = &filter.until {
            sql.push_str(&format!(" AND recorded_at <= ?{idx}"));
            params.push(Box::new(until.clone()));
            idx += 1;
        }
        if filter.only_denied == Some(true) {
            sql.push_str(" AND is_denied = 1");
        }
        if filter.only_security == Some(true) {
            sql.push_str(" AND is_security_related = 1");
        }

        sql.push_str(&format!(
            " ORDER BY recorded_at DESC LIMIT ?{idx} OFFSET ?{}",
            idx + 1
        ));
        params.push(Box::new(limit));
        params.push(Box::new(offset));

        let conn = self.conn()?;
        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let payload_str: String = row.get(6)?;
            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
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

    /// 按时间桶聚合统计(用于直方图可视化)。
    ///
    /// `granularity` 支持 "hour" / "day" / "month"(对应 SQLite strftime 格式)。
    /// `since`/`until` 为可选 RFC3339 字符串,空表示不限。
    pub fn audit_event_histogram(
        &self,
        granularity: &str,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<AuditHistogramBucket>, AppError> {
        let fmt = match granularity {
            "hour" => "%Y-%m-%dT%H:00:00",
            "day" => "%Y-%m-%d",
            "month" => "%Y-%m",
            _ => return Err(AppError::bad_request(format!(
                "Invalid granularity '{}', expected one of: hour/day/month",
                granularity
            ))),
        };

        let mut sql = format!(
            "SELECT strftime('{fmt}', recorded_at) AS bucket, \
                    COUNT(*) AS cnt, \
                    SUM(CASE WHEN is_denied = 1 THEN 1 ELSE 0 END) AS denied_cnt \
             FROM audit_events WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;
        if let Some(s) = since {
            sql.push_str(&format!(" AND recorded_at >= ?{idx}"));
            params.push(Box::new(s.to_string()));
            idx += 1;
        }
        if let Some(u) = until {
            sql.push_str(&format!(" AND recorded_at <= ?{idx}"));
            params.push(Box::new(u.to_string()));
        }
        sql.push_str(" GROUP BY bucket ORDER BY bucket ASC");

        let conn = self.conn()?;
        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(AuditHistogramBucket {
                bucket: row.get(0)?,
                count: row.get(1)?,
                denied: row.get::<_, i64>(2).unwrap_or(0),
            })
        }).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }
}
