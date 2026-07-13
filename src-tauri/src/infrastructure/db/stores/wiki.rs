
use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;

use super::super::connection::Database;
use super::super::connection::db_err;
use super::super::types::{json_decode, json_encode};
use crate::shared::error::AppError;
use crate::shared::wiki::models::{
    WikiEntry, WikiGraphView, WikiGraphNode, WikiGraphEdge, WikiEntityLink,
    CreateWikiEntryRequest, UpdateWikiEntryRequest, CreateWikiLinkRequest,
};

fn map_wiki_entry_row(row: &rusqlite::Row) -> rusqlite::Result<(String, String, String, String, String, String, Option<i64>, String, i64, i64, String, String)> {
    Ok((
        row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
        row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?,
        row.get(10)?, row.get(11)?,
    ))
}

fn build_wiki_entry(
    raw: (String, String, String, String, String, String, Option<i64>, String, i64, i64, String, String),
) -> Result<WikiEntry, AppError> {
    let (id, novel_id, title, content, category_str, source_type_str, source_chapter, tags_json, importance, word_count, created_at, updated_at) = raw;
    Ok(WikiEntry {
        id,
        novel_id,
        title,
        content,
        category: category_str.parse().unwrap_or(crate::shared::wiki::types::WikiCategory::General),
        source_type: source_type_str.parse().unwrap_or(crate::shared::wiki::types::WikiSourceType::Manual),
        source_chapter: source_chapter.map(|n| n as u32),
        tags: json_decode(&tags_json, "tags")?,
        importance: importance as u32,
        word_count: word_count as u32,
        created_at,
        updated_at,
    })
}

impl Database {
    pub fn list_wiki_entries(
        &self,
        novel_id: &str,
        category: Option<&crate::shared::wiki::types::WikiCategory>,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(if category.is_some() {
            "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE novel_id = ? AND category = ? ORDER BY importance DESC, updated_at DESC"
        } else {
            "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE novel_id = ? ORDER BY importance DESC, updated_at DESC"
        }).map_err(db_err)?;
        let rows = if let Some(cat) = category {
            let cat_str = cat.to_string();
            stmt.query_map(params![novel_id, &cat_str], map_wiki_entry_row).map_err(db_err)?
        } else {
            stmt.query_map([novel_id], map_wiki_entry_row).map_err(db_err)?
        };
        rows.map(|r| build_wiki_entry(r.map_err(db_err)?)).collect()
    }

    pub fn get_wiki_entry(&self, entry_id: &str) -> Result<Option<WikiEntry>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE id = ?",
            params![entry_id],
            map_wiki_entry_row,
        );
        match result {
            Ok(raw) => Ok(Some(build_wiki_entry(raw)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn create_wiki_entry(&self, req: &CreateWikiEntryRequest) -> Result<WikiEntry, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = json_encode(&req.tags, "tags")?;
        let importance = req.importance.unwrap_or(0);
        let word_count = count_words(&req.content);
        let source_chapter_i64 = req.source_chapter.map(|n| n as i64);

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO wiki_entries (id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'manual', ?, ?, ?, ?, ?, ?)",
            params![
                &id, &req.novel_id, &req.title, &req.content,
                req.category.to_string(), source_chapter_i64, &tags_json,
                importance as i64, word_count as i64, &now, &now,
            ],
        ).map_err(db_err)?;

        Ok(WikiEntry {
            id, novel_id: req.novel_id.clone(), title: req.title.clone(), content: req.content.clone(),
            category: req.category.clone(), source_type: crate::shared::wiki::types::WikiSourceType::Manual,
            source_chapter: req.source_chapter, tags: req.tags.clone(), importance,
            word_count, created_at: now.clone(), updated_at: now,
        })
    }

    pub fn update_wiki_entry(
        &self,
        entry_id: &str,
        req: &UpdateWikiEntryRequest,
    ) -> Result<WikiEntry, AppError> {
        let existing = self.get_wiki_entry(entry_id)?
            .ok_or_else(|| AppError::not_found("Wiki entry not found"))?;
        let now = Utc::now().to_rfc3339();

        let title = req.title.clone().unwrap_or(existing.title);
        let content = req.content.clone().unwrap_or(existing.content);
        let category = req.category.clone().unwrap_or(existing.category);
        let tags = req.tags.clone().unwrap_or(existing.tags);
        let importance = req.importance.unwrap_or(existing.importance);
        let word_count = count_words(&content);
        let tags_json = json_encode(&tags, "tags")?;

        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE wiki_entries SET title = ?, content = ?, category = ?, tags = ?, importance = ?, word_count = ?, updated_at = ? WHERE id = ?",
                params![
                    &title, &content, category.to_string(), &tags_json,
                    importance as i64, word_count as i64, &now, entry_id,
                ],
            ).map_err(db_err)?;
        }

        Ok(WikiEntry {
            id: existing.id, novel_id: existing.novel_id, title, content, category,
            source_type: existing.source_type, source_chapter: existing.source_chapter,
            tags, importance, word_count, created_at: existing.created_at, updated_at: now,
        })
    }

    pub fn delete_wiki_entry(&self, entry_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM wiki_entries WHERE id = ?", params![entry_id]).map_err(db_err)?;
        Ok(affected > 0)
    }

    pub fn search_wiki_entries(
        &self,
        novel_id: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let limit_val = limit.unwrap_or(20) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT e.id, e.novel_id, e.title, e.content, e.category, e.source_type, e.source_chapter, e.tags, e.importance, e.word_count, e.created_at, e.updated_at \
             FROM wiki_entries e \
             JOIN wiki_entries_fts f ON f.rowid = e.rowid \
             WHERE e.novel_id = ? AND wiki_entries_fts MATCH ? \
             ORDER BY e.importance DESC, e.updated_at DESC LIMIT ?"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, query, limit_val], map_wiki_entry_row).map_err(db_err)?;
        rows.map(|r| build_wiki_entry(r.map_err(db_err)?)).collect()
    }

    pub fn get_wiki_context_for_chapter(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at FROM wiki_entries WHERE novel_id = ? AND (source_chapter = ? OR importance >= 5) ORDER BY importance DESC, updated_at DESC"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, chapter_number as i64], map_wiki_entry_row).map_err(db_err)?;
        rows.map(|r| build_wiki_entry(r.map_err(db_err)?)).collect()
    }

    pub fn get_wiki_graph_view(
        &self,
        novel_id: &str,
        filter_category: Option<&crate::shared::wiki::types::WikiCategory>,
        min_importance: Option<u32>,
    ) -> Result<WikiGraphView, AppError> {
        let min_imp = min_importance.unwrap_or(0);
        let conn = self.conn()?;

        let nodes: Vec<WikiGraphNode> = {
            let mut stmt = if filter_category.is_some() {
                conn.prepare_cached(
                    "SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ? AND category = ? AND importance >= ?"
                ).map_err(db_err)?
            } else {
                conn.prepare_cached(
                    "SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ? AND importance >= ?"
                ).map_err(db_err)?
            };
            let rows = if let Some(cat) = filter_category {
                let cat_str = cat.to_string();
                stmt.query_map(params![novel_id, &cat_str, min_imp as i64], map_graph_node_row).map_err(db_err)?
            } else {
                stmt.query_map(params![novel_id, min_imp as i64], map_graph_node_row).map_err(db_err)?
            };
            rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?
        };

        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

        let all_edges: Vec<WikiGraphEdge> = {
            let mut stmt = conn.prepare_cached(
                "SELECT source_entry_id, target_entry_id, relation_type, weight FROM wiki_entity_links WHERE novel_id = ?"
            ).map_err(db_err)?;
            let rows = stmt.query_map([novel_id], |row| {
                let weight: i64 = row.get(3)?;
                Ok(WikiGraphEdge {
                    source: row.get(0)?,
                    target: row.get(1)?,
                    relation: row.get(2)?,
                    weight: weight as u32,
                })
            }).map_err(db_err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?
        };

        let edges: Vec<_> = all_edges.into_iter()
            .filter(|e| node_ids.contains(&e.source) && node_ids.contains(&e.target))
            .collect();

        Ok(WikiGraphView { nodes, edges })
    }
}

impl Database {
    pub fn create_wiki_link(&self, req: &CreateWikiLinkRequest) -> Result<WikiEntityLink, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let weight = req.weight.unwrap_or(1);
        let source_chapter_i64 = req.source_chapter.map(|n| n as i64);

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO wiki_entity_links (id, novel_id, source_entry_id, target_entry_id, relation_type, relation_desc, weight, source_chapter, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &id, &req.novel_id, &req.source_entry_id, &req.target_entry_id,
                &req.relation_type, &req.relation_desc, weight as i64,
                source_chapter_i64, &now,
            ],
        ).map_err(db_err)?;

        Ok(WikiEntityLink {
            id, novel_id: req.novel_id.clone(), source_entry_id: req.source_entry_id.clone(),
            target_entry_id: req.target_entry_id.clone(), relation_type: req.relation_type.clone(),
            relation_desc: req.relation_desc.clone(), weight, source_chapter: req.source_chapter,
            created_at: now,
        })
    }

    pub fn delete_wiki_link(&self, link_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM wiki_entity_links WHERE id = ?", params![link_id]).map_err(db_err)?;
        Ok(affected > 0)
    }
}

fn map_graph_node_row(row: &rusqlite::Row) -> rusqlite::Result<WikiGraphNode> {
    let importance: i64 = row.get(3)?;
    Ok(WikiGraphNode {
        id: row.get(0)?,
        title: row.get(1)?,
        category: row.get(2)?,
        importance: importance as u32,
    })
}

fn count_words(content: &str) -> u32 {
    crate::shared::story::types::count_words_default(content)
}