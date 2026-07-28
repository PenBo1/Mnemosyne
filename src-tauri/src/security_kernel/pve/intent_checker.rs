//! ═══════════════════════════════════════════════════════════════════════════
//! intent_checker - 意图检查模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SensitiveSurfaceType {
    Secrets,
    Credentials,
    Configuration,
    Environment,
    Database,
    SystemFiles,
    UserPrivateData,
    NetworkCredentials,
    ApiKeys,
}

impl std::fmt::Display for SensitiveSurfaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Secrets => write!(f, "secrets"),
            Self::Credentials => write!(f, "credentials"),
            Self::Configuration => write!(f, "configuration"),
            Self::Environment => write!(f, "environment"),
            Self::Database => write!(f, "database"),
            Self::SystemFiles => write!(f, "system_files"),
            Self::UserPrivateData => write!(f, "user_private_data"),
            Self::NetworkCredentials => write!(f, "network_credentials"),
            Self::ApiKeys => write!(f, "api_keys"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveSurface {
    pub surface_type: SensitiveSurfaceType,
    pub patterns: Vec<String>,
    pub description: String,
}

static DEFAULT_SENSITIVE_SURFACES: LazyLock<Vec<SensitiveSurface>> = LazyLock::new(|| {
    vec![
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::Secrets,
            patterns: vec![".env".to_string(), ".env.local".to_string(), ".env.production".to_string()],
            description: "Environment files containing secrets".to_string(),
        },
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::Credentials,
            patterns: vec!["credentials.json".to_string(), "secrets.json".to_string()],
            description: "Credential files".to_string(),
        },
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::Configuration,
            patterns: vec!["config/secrets".to_string(), ".config/credentials".to_string()],
            description: "Configuration with sensitive data".to_string(),
        },
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::Environment,
            patterns: vec![".env.".to_string()],
            description: "Environment configuration files".to_string(),
        },
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::ApiKeys,
            patterns: vec!["api_key".to_string(), "apikey".to_string(), "api-key".to_string()],
            description: "Files named with API key patterns".to_string(),
        },
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::SystemFiles,
            patterns: vec!["/etc/passwd".to_string(), "/etc/shadow".to_string()],
            description: "System files with sensitive data".to_string(),
        },
        SensitiveSurface {
            surface_type: SensitiveSurfaceType::UserPrivateData,
            patterns: vec![".ssh/".to_string(), ".gnupg/".to_string()],
            description: "User private key directories".to_string(),
        },
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveSurfaceMatch {
    pub surface: SensitiveSurface,
    pub matched_pattern: String,
    pub matched_path: String,
}

#[derive(Debug, Clone)]
pub struct OperationIntent {
    pub declared_intent: String,
    pub operation_type: String,
    pub target_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentCheckResult {
    pub is_aligned: bool,
    pub warnings: Vec<String>,
    pub sensitive_surface_hits: Vec<SensitiveSurfaceMatch>,
}

pub struct IntentChecker {
    sensitive_surfaces: Vec<SensitiveSurface>,
}

impl IntentChecker {
    pub fn new() -> Self {
        Self {
            sensitive_surfaces: DEFAULT_SENSITIVE_SURFACES.clone(),
        }
    }

    pub fn with_surfaces(surfaces: Vec<SensitiveSurface>) -> Self {
        Self {
            sensitive_surfaces: surfaces,
        }
    }

    pub fn check_alignment(
        &self,
        intent: &OperationIntent,
        actual_operation: &str,
    ) -> Result<bool, AppError> {
        let intent_lower = intent.declared_intent.to_lowercase();
        let operation_lower = actual_operation.to_lowercase();

        let intent_keywords = self.extract_keywords(&intent_lower);
        let operation_keywords = self.extract_keywords(&operation_lower);

        let common_keywords: Vec<_> = intent_keywords
            .iter()
            .filter(|k| operation_keywords.contains(k))
            .collect();

        let alignment_score = if intent_keywords.is_empty() {
            0.0
        } else {
            common_keywords.len() as f64 / intent_keywords.len() as f64
        };

        Ok(alignment_score >= 0.3)
    }

    pub fn check_sensitive_surface(
        &self,
        path: &str,
    ) -> Vec<SensitiveSurfaceMatch> {
        let mut matches = Vec::new();
        let path_lower = path.to_lowercase();

        for surface in &self.sensitive_surfaces {
            for pattern in &surface.patterns {
                let pattern_lower = pattern.to_lowercase();
                if path_lower.contains(&pattern_lower) {
                    matches.push(SensitiveSurfaceMatch {
                        surface: surface.clone(),
                        matched_pattern: pattern.clone(),
                        matched_path: path.to_string(),
                    });
                    break;
                }
            }
        }

        matches
    }

    pub fn check_sensitive_surface_and_fail(
        &self,
        path: &str,
    ) -> Result<Vec<SensitiveSurfaceMatch>, AppError> {
        let matches = self.check_sensitive_surface(path);

        if !matches.is_empty() {
            let descriptions: Vec<_> = matches
                .iter()
                .map(|m| format!("{}: {}", m.surface.surface_type, m.surface.description))
                .collect();

            return Err(AppError::forbidden(format!(
                "Access to sensitive surface denied: {}",
                descriptions.join(", ")
            )));
        }

        Ok(matches)
    }

    pub fn check_full(
        &self,
        intent: &OperationIntent,
        actual_operation: &str,
        paths: &[&str],
    ) -> Result<IntentCheckResult, AppError> {
        let is_aligned = self.check_alignment(intent, actual_operation)?;
        let mut warnings = Vec::new();

        if !is_aligned {
            warnings.push(format!(
                "Operation '{}' may not align with declared intent '{}'",
                actual_operation, intent.declared_intent
            ));
        }

        let mut sensitive_surface_hits = Vec::new();
        for path in paths {
            let hits = self.check_sensitive_surface(path);
            for hit in hits {
                warnings.push(format!(
                    "Operation touches sensitive surface: {} at {}",
                    hit.surface.surface_type, hit.matched_path
                ));
                sensitive_surface_hits.push(hit);
            }
        }

        Ok(IntentCheckResult {
            is_aligned,
            warnings,
            sensitive_surface_hits,
        })
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let stop_words = ["the", "a", "an", "is", "are", "was", "were", "to", "of", "and", "in", "for", "on", "with", "at", "by"];

        text.split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 2 && !stop_words.contains(&w.as_str()))
            .collect()
    }

    pub fn sensitive_surfaces(&self) -> &[SensitiveSurface] {
        &self.sensitive_surfaces
    }

    pub fn add_sensitive_surface(&mut self, surface: SensitiveSurface) {
        self.sensitive_surfaces.push(surface);
    }

    pub fn get_surfaces_by_type(&self, surface_type: SensitiveSurfaceType) -> Vec<&SensitiveSurface> {
        self.sensitive_surfaces
            .iter()
            .filter(|s| s.surface_type == surface_type)
            .collect()
    }
}

impl Default for IntentChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub fn get_default_sensitive_surfaces() -> &'static [SensitiveSurface] {
    &DEFAULT_SENSITIVE_SURFACES
}

pub fn is_path_sensitive(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let checker = IntentChecker::new();
    !checker.check_sensitive_surface(&path_str).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_sensitive_surface_env_file() {
        let checker = IntentChecker::new();
        let matches = checker.check_sensitive_surface("/app/.env");

        assert!(!matches.is_empty());
        assert_eq!(matches[0].surface.surface_type, SensitiveSurfaceType::Secrets);
    }

    #[test]
    fn test_check_sensitive_surface_normal_file() {
        let checker = IntentChecker::new();
        let matches = checker.check_sensitive_surface("/app/src/main.rs");

        assert!(matches.is_empty());
    }

    #[test]
    fn test_check_sensitive_surface_and_fail() {
        let checker = IntentChecker::new();
        let result = checker.check_sensitive_surface_and_fail("/app/.env");

        assert!(result.is_err());
    }

    #[test]
    fn test_check_alignment_aligned() {
        let checker = IntentChecker::new();
        let intent = OperationIntent {
            declared_intent: "read the configuration file".to_string(),
            operation_type: "read".to_string(),
            target_resources: vec!["config.json".to_string()],
        };

        let is_aligned = checker.check_alignment(&intent, "read config file").unwrap();
        assert!(is_aligned);
    }

    #[test]
    fn test_check_alignment_not_aligned() {
        let checker = IntentChecker::new();
        let intent = OperationIntent {
            declared_intent: "write documentation".to_string(),
            operation_type: "write".to_string(),
            target_resources: vec!["README.md".to_string()],
        };

        let is_aligned = checker.check_alignment(&intent, "delete all files").unwrap();
        assert!(!is_aligned);
    }

    #[test]
    fn test_check_full() {
        let checker = IntentChecker::new();
        let intent = OperationIntent {
            declared_intent: "read configuration".to_string(),
            operation_type: "read".to_string(),
            target_resources: vec!["config.json".to_string()],
        };

        let result = checker.check_full(&intent, "read config file", &["/app/.env", "/app/config.json"]).unwrap();

        assert!(result.is_aligned);
        assert!(!result.sensitive_surface_hits.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_is_path_sensitive() {
        assert!(is_path_sensitive(Path::new("/app/.env")));
        assert!(!is_path_sensitive(Path::new("/app/src/main.rs")));
    }
}