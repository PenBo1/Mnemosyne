use std::path::Path;
use crate::shared::errors::AppError;
use super::types::BookSource;

pub fn load_sources_from_dir(dir: &Path) -> Result<Vec<BookSource>, AppError> {
    let mut sources = Vec::new();
    if !dir.exists() {
        return Ok(sources);
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| AppError::internal(format!("Failed to read rules dir: {}", e)))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match load_sources_from_file(&path) {
                Ok(mut file_sources) => sources.append(&mut file_sources),
                Err(_e) => {
                    tracing::warn!(path = %path.display(), "Failed to load book sources");
                }
            }
        }
    }
    Ok(sources)
}

pub fn load_sources_from_file(path: &Path) -> Result<Vec<BookSource>, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| AppError::file_read_error(path.display().to_string()))?;
    let sources: Vec<BookSource> = serde_json::from_str(&content)
        .map_err(|e| AppError::invalid_format(format!("Invalid book source JSON: {}", e)))?;
    Ok(sources)
}

pub fn load_builtin_sources_from_dir(dir: &Path) -> Vec<BookSource> {
    match load_sources_from_dir(dir) {
        Ok(sources) => sources,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load book sources from directory");
            Vec::new()
        }
    }
}

/// Embedded book source files from resources/book_sources/
const MAIN_SOURCES: &str = include_str!("../../../resources/book_sources/main.json");
const PROXY_SOURCES: &str = include_str!("../../../resources/book_sources/proxy-required.json");
const RATE_LIMIT_SOURCES: &str = include_str!("../../../resources/book_sources/rate-limit.json");
const CLOUDFLARE_SOURCES: &str = include_str!("../../../resources/book_sources/cloudflare.json");

/// Extract embedded book sources to the target directory
pub fn extract_builtin_sources_to_dir(dir: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dir)
        .map_err(|e| AppError::internal(format!("Failed to create book sources dir: {}", e)))?;
    
    let files = [
        ("main.json", MAIN_SOURCES),
        ("proxy-required.json", PROXY_SOURCES),
        ("rate-limit.json", RATE_LIMIT_SOURCES),
        ("cloudflare.json", CLOUDFLARE_SOURCES),
    ];
    
    for (filename, content) in files {
        let path = dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, content)
                .map_err(|e| AppError::internal(format!("Failed to write {}: {}", filename, e)))?;
            tracing::info!(path = %path.display(), "Extracted book source file");
        }
    }
    
    Ok(())
}
