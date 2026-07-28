//! ═══════════════════════════════════════════════════════════════════════════
//! 趋势存储 - 热点趋势与雷达扫描
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 两类存储：
//! - Trend：热点趋势（关键词、平台、热度分数）
//! - RadarScan：雷达扫描（市场摘要、推荐、平台排名）

use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;

use super::super::types::{Trend, RadarScan, RadarRecommendation, PlatformRankings};
use super::super::connection::Database;
use super::super::connection::db_err;
use super::super::types::{json_decode, json_encode};
use crate::shared::error::AppError;
use crate::infrastructure::db::connection::validate_name;

// ── Trend 辅助函数 ──────────────────────────────────────────────────────────

/// 映射趋势行
fn map_trend_row(row: &rusqlite::Row) -> Result<(String, String, String, f64, String, String), rusqlite::Error> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
}

/// 构建趋势对象
fn build_trend(raw: (String, String, String, f64, String, String)) -> Result<Trend, AppError> {
    let (id, keyword, platform, score, meta_str, scanned_at) = raw;
    Ok(Trend {
        id,
        keyword,
        platform,
        score,
        metadata: json_decode(&meta_str, "metadata")?,
        scanned_at,
    })
}

// ── Trend 操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 创建趋势
    pub fn create_trend(&self, keyword: &str, platform: &str, score: f64, metadata: serde_json::Value) -> Result<Trend, AppError> {
        validate_name(keyword, "Trend keyword")?;
        validate_name(platform, "Trend platform")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let meta_str = json_encode(&metadata, "metadata")?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO trends (id, keyword, platform, score, metadata, scanned_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![&id, keyword, platform, score, &meta_str, &now],
        ).map_err(db_err)?;
        Ok(Trend { id, keyword: keyword.to_string(), platform: platform.to_string(), score, metadata, scanned_at: now })
    }

    /// 列出趋势
    pub fn list_trends(&self, platform: Option<&str>, limit: Option<i64>) -> Result<Vec<Trend>, AppError> {
        let limit = limit.unwrap_or(100).clamp(1, 1000);
        let conn = self.conn()?;
        if let Some(p) = platform {
            let mut stmt = conn.prepare_cached(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends WHERE platform = ? ORDER BY scanned_at DESC LIMIT ?",
            ).map_err(db_err)?;
            let rows = stmt.query_map(params![p, limit], map_trend_row).map_err(db_err)?;
            rows.map(|r| build_trend(r.map_err(db_err)?)).collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends ORDER BY scanned_at DESC LIMIT ?",
            ).map_err(db_err)?;
            let rows = stmt.query_map(params![limit], map_trend_row).map_err(db_err)?;
            rows.map(|r| build_trend(r.map_err(db_err)?)).collect()
        }
    }

    /// 删除趋势
    pub fn delete_trend(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM trends WHERE id = ?", params![id]).map_err(db_err)?;
        Ok(affected > 0)
    }
}

// ── RadarScan 辅助函数 ──────────────────────────────────────────────────────

/// 映射雷达扫描行
fn map_radar_scan_row(row: &rusqlite::Row) -> Result<(String, String, String, String, String), rusqlite::Error> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
}

/// 构建雷达扫描对象
fn build_radar_scan(raw: (String, String, String, String, String)) -> Result<RadarScan, AppError> {
    let (id, market_summary, recs_str, raw_str, created_at) = raw;
    Ok(RadarScan {
        id,
        market_summary,
        recommendations: json_decode(&recs_str, "recommendations")?,
        raw_rankings: json_decode(&raw_str, "raw_rankings")?,
        created_at,
    })
}

// ── RadarScan 操作 ──────────────────────────────────────────────────────────

impl Database {
    /// 创建雷达扫描
    pub fn create_radar_scan(
        &self,
        market_summary: &str,
        recommendations: &[RadarRecommendation],
        raw_rankings: &[PlatformRankings],
    ) -> Result<RadarScan, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let recs_json = json_encode(&recommendations, "recommendations")?;
        let raw_json = json_encode(&raw_rankings, "raw_rankings")?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO radar_scans (id, market_summary, recommendations_json, raw_rankings_json, created_at) VALUES (?, ?, ?, ?, ?)",
            params![&id, market_summary, &recs_json, &raw_json, &now],
        ).map_err(db_err)?;
        Ok(RadarScan {
            id,
            market_summary: market_summary.to_string(),
            recommendations: recommendations.to_vec(),
            raw_rankings: raw_rankings.to_vec(),
            created_at: now,
        })
    }

    /// 列出雷达扫描
    pub fn list_radar_scans(&self, limit: Option<i64>) -> Result<Vec<RadarScan>, AppError> {
        let limit = limit.unwrap_or(50).clamp(1, 500);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, market_summary, recommendations_json, raw_rankings_json, created_at FROM radar_scans ORDER BY created_at DESC LIMIT ?",
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![limit], map_radar_scan_row).map_err(db_err)?;
        rows.map(|r| build_radar_scan(r.map_err(db_err)?)).collect()
    }

    /// 删除雷达扫描
    pub fn delete_radar_scan(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM radar_scans WHERE id = ?", params![id]).map_err(db_err)?;
        Ok(affected > 0)
    }
}