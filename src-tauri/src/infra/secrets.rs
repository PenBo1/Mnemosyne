use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Encrypted secrets store for API keys
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretsStore {
    pub entries: std::collections::HashMap<String, String>,
}

impl SecretsStore {
    /// Load secrets from file
    pub fn load(path: &PathBuf) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save secrets to file
    pub fn save(&self, path: &PathBuf) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("Failed to create secrets dir: {}", e)))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::internal(format!("Failed to serialize secrets: {}", e)))?;
        std::fs::write(path, json)
            .map_err(|e| AppError::internal(format!("Failed to write secrets: {}", e)))
    }

    /// Get a secret value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    /// Set a secret value
    pub fn set(&mut self, key: String, value: String) {
        self.entries.insert(key, value);
    }

    /// Remove a secret
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key)
    }

    /// Get API key for a provider, checking env var first, then secrets store
    pub fn get_api_key(&self, provider: &str, env_var: &str) -> String {
        // Check env var first
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return key;
            }
        }
        // Fall back to secrets store
        self.get(&format!("{}_api_key", provider))
            .unwrap_or("")
            .to_string()
    }
}

/// Resolve secrets path for the application
pub fn secrets_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("secrets.json")
}
