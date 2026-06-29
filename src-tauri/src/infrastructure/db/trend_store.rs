use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::models::{Trend, RadarScan, RadarRecommendation, PlatformRankings};
use super::Database;
use super::connection::db_err;
use super::error::{json_decode, json_encode};
use crate::shared::errors::AppError;

impl Database {
    pub async fn create_trend(&self, keyword: &str, platform: &str, score: f64, metadata: serde_json::Value) -> Result<Trend, AppError> {
        Self::validate_name(keyword, "Trend keyword")?;
        Self::validate_name(platform, "Trend platform")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let meta_str = json_encode(&metadata, "metadata")?;
        sqlx::query(
            "INSERT INTO trends (id, keyword, platform, score, metadata, scanned_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(keyword).bind(platform).bind(score).bind(&meta_str).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(Trend { id, keyword: keyword.to_string(), platform: platform.to_string(), score, metadata, scanned_at: now })
    }

    pub async fn list_trends(&self, platform: Option<&str>, limit: Option<i64>) -> Result<Vec<Trend>, AppError> {
        let limit = limit.unwrap_or(100).max(1).min(1000);
        if let Some(p) = platform {
            let rows = sqlx::query(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends WHERE platform = ? ORDER BY scanned_at DESC LIMIT ?"
            )
            .bind(p).bind(limit)
            .fetch_all(&self.pool).await.map_err(db_err)?;
            rows.iter().map(|row| {
                let meta_str: String = row.get(4);
                Ok(Trend {
                    id: row.get(0), keyword: row.get(1), platform: row.get(2),
                    score: row.get(3), metadata: json_decode(&meta_str, "metadata")?,
                    scanned_at: row.get(5),
                })
            }).collect()
        } else {
            let rows = sqlx::query(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends ORDER BY scanned_at DESC LIMIT ?"
            )
            .bind(limit)
            .fetch_all(&self.pool).await.map_err(db_err)?;
            rows.iter().map(|row| {
                let meta_str: String = row.get(4);
                Ok(Trend {
                    id: row.get(0), keyword: row.get(1), platform: row.get(2),
                    score: row.get(3), metadata: json_decode(&meta_str, "metadata")?,
                    scanned_at: row.get(5),
                })
            }).collect()
        }
    }

    pub async fn delete_trend(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM trends WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}

// ═══════════════════════════════════════════════════════════
// Radar
// ═══════════════════════════════════════════════════════════

impl Database {
    pub async fn create_radar_scan(
        &self,
        market_summary: &str,
        recommendations: &[RadarRecommendation],
        raw_rankings: &[PlatformRankings],
    ) -> Result<RadarScan, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let recs_json = json_encode(&recommendations, "recommendations")?;
        let raw_json = json_encode(&raw_rankings, "raw_rankings")?;
        sqlx::query(
            "INSERT INTO radar_scans (id, market_summary, recommendations_json, raw_rankings_json, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(market_summary).bind(&recs_json).bind(&raw_json).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(RadarScan {
            id,
            market_summary: market_summary.to_string(),
            recommendations: recommendations.to_vec(),
            raw_rankings: raw_rankings.to_vec(),
            created_at: now,
        })
    }

    pub async fn list_radar_scans(&self, limit: Option<i64>) -> Result<Vec<RadarScan>, AppError> {
        let limit = limit.unwrap_or(50).max(1).min(500);
        let rows = sqlx::query(
            "SELECT id, market_summary, recommendations_json, raw_rankings_json, created_at FROM radar_scans ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(|row| {
            let recs_str: String = row.get(2);
            let raw_str: String = row.get(3);
            Ok(RadarScan {
                id: row.get(0), market_summary: row.get(1),
                recommendations: json_decode(&recs_str, "recommendations")?,
                raw_rankings: json_decode(&raw_str, "raw_rankings")?,
                created_at: row.get(4),
            })
        }).collect()
    }

    pub async fn delete_radar_scan(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM radar_scans WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}
