use std::path::Path;
use crate::shared::errors::AppError;
use crate::infrastructure::db::Database;
use super::models::*;
use super::diff_engine::DiffEngine;

pub struct VersionService {
    db: Database,
}

impl VersionService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// List all versions for a chapter
    pub async fn list_versions(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<ChapterVersion>, AppError> {
        self.db.list_chapter_versions(novel_id, chapter_number).await
    }

    /// Get a specific version by ID
    pub async fn get_version(&self, version_id: &str) -> Result<Option<ChapterVersion>, AppError> {
        self.db.get_chapter_version(version_id).await
    }

    /// Get the latest version for a chapter
    pub async fn get_latest_version(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Option<ChapterVersion>, AppError> {
        self.db.get_latest_chapter_version(novel_id, chapter_number).await
    }

    /// Save a new version (called after revision)
    pub async fn save_version(
        &self,
        novel_id: &str,
        chapter_number: u32,
        content: &str,
        revision_mode: RevisionMode,
        revision_reason: &str,
    ) -> Result<ChapterVersion, AppError> {
        // Get next version number
        let next_version_number = self.db.get_next_version_number(novel_id, chapter_number).await?;

        // Compute content hash
        let content_hash = DiffEngine::compute_hash(content);

        // Count words (simplified: count Chinese chars + English words)
        let word_count = count_words(content);

        let request = CreateVersionRequest {
            novel_id: novel_id.to_string(),
            chapter_number,
            content: content.to_string(),
            revision_mode,
            revision_reason: revision_reason.to_string(),
        };

        self.db.create_chapter_version(&request, next_version_number, &content_hash, word_count).await
    }

    /// Compute diff between two versions
    pub async fn compute_diff(
        &self,
        from_version_id: &str,
        to_version_id: &str,
    ) -> Result<LineDiffResult, AppError> {
        let from_version = self.db.get_chapter_version(from_version_id).await?
            .ok_or_else(|| AppError::not_found("From version not found"))?;

        let to_version = self.db.get_chapter_version(to_version_id).await?
            .ok_or_else(|| AppError::not_found("To version not found"))?;

        Ok(DiffEngine::compute_line_diff(&from_version.content, &to_version.content))
    }

    /// Compute diff between latest two versions for a chapter
    pub async fn compute_latest_diff(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Option<LineDiffResult>, AppError> {
        let versions = self.db.list_chapter_versions(novel_id, chapter_number).await?;

        if versions.len() < 2 {
            return Ok(None);
        }

        // Get the two most recent versions
        let to_version = &versions[0];
        let from_version = &versions[1];

        Ok(Some(DiffEngine::compute_line_diff(&from_version.content, &to_version.content)))
    }

    /// Restore a chapter to a previous version
    pub async fn restore_version(
        &self,
        version_id: &str,
        book_dir: &Path,
    ) -> Result<bool, AppError> {
        let version = self.db.get_chapter_version(version_id).await?
            .ok_or_else(|| AppError::not_found("Version not found"))?;
        
        // Write content back to chapter file
        crate::core::agent::pipeline::chapter_persistence::save_chapter_file(
            book_dir,
            version.chapter_number,
            "",  // title will be preserved from file
            &version.content,
        )?;
        
        Ok(true)
    }
}

/// Count words in content (Chinese chars + English words approximation)
fn count_words(content: &str) -> u32 {
    let chinese_chars = content.chars().filter(|c| c.is_ascii()).count() as u32;
    let english_words = content.split_whitespace().count() as u32;
    // Approximate: Chinese ~1.5 chars per word, English ~1 word
    (chinese_chars / 2 + english_words) as u32
}