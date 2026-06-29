//! Config loader utilities.

use std::path::Path;

/// Load a JSON config file
pub fn load_json_config<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, crate::shared::errors::AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to read config: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to parse config: {}", e)))
}

/// Save a JSON config file
pub fn save_json_config<T: Serialize>(path: &Path, config: &T) -> Result<(), crate::shared::errors::AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to create dir: {}", e)))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to serialize: {}", e)))?;
    std::fs::write(path, json)
        .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to write: {}", e)))
}

use serde::{Deserialize, Serialize};
