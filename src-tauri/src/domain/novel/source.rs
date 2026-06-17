use std::path::Path;
use crate::errors::AppError;
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

pub fn load_builtin_sources() -> Vec<BookSource> {
    let main_json = include_str!("../../../../demo/so-novel/bundle/rules/main.json");
    let proxy_json = include_str!("../../../../demo/so-novel/bundle/rules/proxy-required.json");
    let rate_json = include_str!("../../../../demo/so-novel/bundle/rules/rate-limit.json");
    let cf_json = include_str!("../../../../demo/so-novel/bundle/rules/cloudflare.json");

    let mut sources = Vec::new();
    for json_str in [main_json, proxy_json, rate_json, cf_json] {
        if let Ok(mut file_sources) = serde_json::from_str::<Vec<BookSource>>(json_str) {
            sources.append(&mut file_sources);
        }
    }
    sources
}
