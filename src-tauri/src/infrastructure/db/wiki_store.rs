use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::Database;
use super::connection::db_err;
use super::error::{json_decode, json_encode};
use crate::shared::errors::AppError;
use crate::features::wiki::{
    WikiEntry, CreateWikiEntryRequest, UpdateWikiEntryRequest,
    CreateWikiLinkRequest, WikiEntityLink,
    WikiGraphView, WikiGraphNode, WikiGraphEdge,
};

impl Database {
    fn map_wiki_entry(row: &sqlx::sqlite::SqliteRow) -> Result<WikiEntry, AppError> {
        let category_str: String = row.get(4);
        let source_type_str: String = row.get(5);
        let tags_json: String = row.get(7);
        Ok(WikiEntry {
            id: row.get(0),
            novel_id: row.get(1),
            title: row.get(2),
            content: row.get(3),
            category: category_str.parse().unwrap_or(crate::features::wiki::WikiCategory::General),
            source_type: source_type_str.parse().unwrap_or(crate::features::wiki::WikiSourceType::Manual),
            source_chapter: row.try_get::<Option<i64>, usize>(6).unwrap_or(None).map(|n| n as u32),
            tags: json_decode(&tags_json, "tags")?,
            importance: row.get::<i64, usize>(8) as u32,
            word_count: row.get::<i64, usize>(9) as u32,
            created_at: row.get(10),
            updated_at: row.get(11),
        })
    }

    pub async fn list_wiki_entries(
        &self,
        novel_id: &str,
        category: Option<&crate::features::wiki::WikiCategory>,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let rows = if let Some(cat) = category {
            sqlx::query(
                "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE novel_id = ? AND category = ? ORDER BY importance DESC, updated_at DESC"
            )
            .bind(novel_id).bind(cat.to_string())
            .fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query(
                "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE novel_id = ? ORDER BY importance DESC, updated_at DESC"
            )
            .bind(novel_id)
            .fetch_all(&self.pool).await.map_err(db_err)?
        };
        rows.iter().map(Self::map_wiki_entry).collect()
    }

    pub async fn get_wiki_entry(&self, entry_id: &str) -> Result<Option<WikiEntry>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE id = ?"
        )
        .bind(entry_id)
        .fetch_optional(&self.pool).await.map_err(db_err)?;
        match row_opt {
            None => Ok(None),
            Some(row) => Ok(Some(Self::map_wiki_entry(&row)?)),
        }
    }

    pub async fn create_wiki_entry(&self, req: &CreateWikiEntryRequest) -> Result<WikiEntry, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = json_encode(&req.tags, "tags")?;
        let importance = req.importance.unwrap_or(0);
        let word_count = count_words(&req.content);
        let source_chapter_i64 = req.source_chapter.map(|n| n as i64);

        sqlx::query(
            "INSERT INTO wiki_entries (id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'manual', ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.novel_id).bind(&req.title).bind(&req.content)
        .bind(req.category.to_string()).bind(source_chapter_i64).bind(&tags_json)
        .bind(importance as i64).bind(word_count as i64).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(WikiEntry {
            id, novel_id: req.novel_id.clone(), title: req.title.clone(), content: req.content.clone(),
            category: req.category.clone(), source_type: crate::features::wiki::WikiSourceType::Manual,
            source_chapter: req.source_chapter, tags: req.tags.clone(), importance,
            word_count, created_at: now.clone(), updated_at: now,
        })
    }

    pub async fn update_wiki_entry(
        &self,
        entry_id: &str,
        req: &UpdateWikiEntryRequest,
    ) -> Result<WikiEntry, AppError> {
        let existing = self.get_wiki_entry(entry_id).await?.ok_or_else(|| AppError::not_found("Wiki entry not found"))?;
        let now = Utc::now().to_rfc3339();

        let title = req.title.clone().unwrap_or(existing.title);
        let content = req.content.clone().unwrap_or(existing.content);
        let category = req.category.clone().unwrap_or(existing.category);
        let tags = req.tags.clone().unwrap_or(existing.tags);
        let importance = req.importance.unwrap_or(existing.importance);
        let word_count = count_words(&content);
        let tags_json = json_encode(&tags, "tags")?;

        sqlx::query(
            "UPDATE wiki_entries SET title = ?, content = ?, category = ?, tags = ?, importance = ?, word_count = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&title).bind(&content).bind(category.to_string()).bind(&tags_json)
        .bind(importance as i64).bind(word_count as i64).bind(&now).bind(entry_id)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(WikiEntry {
            id: existing.id, novel_id: existing.novel_id, title, content, category,
            source_type: existing.source_type, source_chapter: existing.source_chapter,
            tags, importance, word_count, created_at: existing.created_at, updated_at: now,
        })
    }

    pub async fn delete_wiki_entry(&self, entry_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM wiki_entries WHERE id = ?")
            .bind(entry_id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    /// FTS5 全文搜索 Wiki 条目。
    pub async fn search_wiki_entries(
        &self,
        novel_id: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let limit_val = limit.unwrap_or(20) as i64;
        // 用 FTS5 JOIN 主表，避免 LIKE 全表扫描
        let rows = sqlx::query(
            "SELECT e.id, e.novel_id, e.title, e.content, e.category, e.source_type, e.source_chapter, e.tags, e.importance, e.word_count, e.created_at, e.updated_at \
             FROM wiki_entries e \
             JOIN wiki_entries_fts f ON f.rowid = e.rowid \
             WHERE e.novel_id = ? AND wiki_entries_fts MATCH ? \
             ORDER BY e.importance DESC, e.updated_at DESC LIMIT ?"
        )
        .bind(novel_id).bind(query).bind(limit_val)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(Self::map_wiki_entry).collect()
    }

    pub async fn get_wiki_context_for_chapter(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE novel_id = ? AND (source_chapter = ? OR importance >= 5) ORDER BY importance DESC, updated_at DESC"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(Self::map_wiki_entry).collect()
    }

    pub async fn get_wiki_graph_view(
        &self,
        novel_id: &str,
        filter_category: Option<&crate::features::wiki::WikiCategory>,
        min_importance: Option<u32>,
    ) -> Result<WikiGraphView, AppError> {
        let min_imp = min_importance.unwrap_or(0);

        let nodes: Vec<WikiGraphNode> = if let Some(cat) = filter_category {
            sqlx::query(
                "SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ? AND category = ? AND importance >= ?"
            )
            .bind(novel_id).bind(cat.to_string()).bind(min_imp as i64)
            .map(|row: sqlx::sqlite::SqliteRow| WikiGraphNode {
                id: row.get(0), title: row.get(1), category: row.get(2),
                importance: row.get::<i64, usize>(3) as u32,
            })
            .fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query(
                "SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ? AND importance >= ?"
            )
            .bind(novel_id).bind(min_imp as i64)
            .map(|row: sqlx::sqlite::SqliteRow| WikiGraphNode {
                id: row.get(0), title: row.get(1), category: row.get(2),
                importance: row.get::<i64, usize>(3) as u32,
            })
            .fetch_all(&self.pool).await.map_err(db_err)?
        };

        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

        let all_edges: Vec<WikiGraphEdge> = sqlx::query(
            "SELECT source_entry_id, target_entry_id, relation_type, weight FROM wiki_entity_links WHERE novel_id = ?"
        )
        .bind(novel_id)
        .map(|row: sqlx::sqlite::SqliteRow| WikiGraphEdge {
            source: row.get(0), target: row.get(1), relation: row.get(2),
            weight: row.get::<i64, usize>(3) as u32,
        })
        .fetch_all(&self.pool).await.map_err(db_err)?;

        let edges: Vec<_> = all_edges.into_iter()
            .filter(|e| node_ids.contains(&e.source) && node_ids.contains(&e.target))
            .collect();

        Ok(WikiGraphView { nodes, edges })
    }
}

// ═══════════════════════════════════════════════════════════
// Wiki Entity Links
// ═══════════════════════════════════════════════════════════

impl Database {
    pub async fn create_wiki_link(&self, req: &CreateWikiLinkRequest) -> Result<WikiEntityLink, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let weight = req.weight.unwrap_or(1);
        let source_chapter_i64 = req.source_chapter.map(|n| n as i64);

        sqlx::query(
            "INSERT INTO wiki_entity_links (id, novel_id, source_entry_id, target_entry_id, relation_type, relation_desc, weight, source_chapter, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.novel_id).bind(&req.source_entry_id).bind(&req.target_entry_id)
        .bind(&req.relation_type).bind(&req.relation_desc).bind(weight as i64)
        .bind(source_chapter_i64).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(WikiEntityLink {
            id, novel_id: req.novel_id.clone(), source_entry_id: req.source_entry_id.clone(),
            target_entry_id: req.target_entry_id.clone(), relation_type: req.relation_type.clone(),
            relation_desc: req.relation_desc.clone(), weight, source_chapter: req.source_chapter,
            created_at: now,
        })
    }

    pub async fn delete_wiki_link(&self, link_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM wiki_entity_links WHERE id = ?")
            .bind(link_id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}

/// 字数统计委托给共享实现。
fn count_words(content: &str) -> u32 {
    crate::infrastructure::utils::text_utils::count_words(content)
}
