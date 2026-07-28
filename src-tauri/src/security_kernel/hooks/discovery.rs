//! ═══════════════════════════════════════════════════════════════════════════
//! discovery - Hook 发现机制模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::shared::error::AppError;

use super::registry::HookRegistry;
use super::types::{HookConfig, HookEvent, HookHandler, HookSpec};

// ── Hook 发现器 ────────────────────────────────────────────────────────────────

/// Hook 发现器 —— 从文件系统发现 hook 配置。
pub struct HookDiscovery {
    registry: Arc<HookRegistry>,
}

impl HookDiscovery {
    pub fn new(registry: Arc<HookRegistry>) -> Self {
        Self { registry }
    }

    /// 从配置文件加载 hooks。
    pub fn load_from_file(&self, path: &Path) -> Result<Vec<String>, AppError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::internal(format!("Failed to read hook config '{}': {}", path.display(), e)))?;

        let configs: Vec<HookConfig> = if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| AppError::invalid_input(format!("Invalid YAML hook config '{}': {}", path.display(), e)))?
        } else {
            serde_json::from_str(&content)
                .map_err(|e| AppError::invalid_input(format!("Invalid JSON hook config '{}': {}", path.display(), e)))?
        };

        let mut ids = Vec::new();
        for config in configs {
            let id = self.registry.register_config(config)?;
            ids.push(id);
        }

        tracing::info!(
            path = %path.display(),
            count = ids.len(),
            "Loaded hooks from config file"
        );

        Ok(ids)
    }

    /// 从目录扫描并加载 hooks。
    pub fn load_from_directory(&self, dir: &Path) -> Result<Vec<String>, AppError> {
        if !dir.exists() {
            tracing::debug!(dir = %dir.display(), "Hook directory does not exist, skipping");
            return Ok(Vec::new());
        }

        let mut all_ids = Vec::new();
        let entries = std::fs::read_dir(dir)
            .map_err(|e| AppError::internal(format!("Failed to read hook directory '{}': {}", dir.display(), e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::internal(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml" || ext == "json") {
                match self.load_from_file(&path) {
                    Ok(ids) => all_ids.extend(ids),
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to load hooks from file, skipping"
                        );
                    }
                }
            }
        }

        tracing::info!(
            dir = %dir.display(),
            total_count = all_ids.len(),
            "Loaded hooks from directory"
        );

        Ok(all_ids)
    }

    /// 注册内置 hooks（硬编码默认 hooks）。
    pub fn register_builtin_hooks(&self) -> Vec<String> {
        let mut ids = Vec::new();

        let builtin_configs = vec![
            HookConfig {
                id: Some("builtin-log-all".to_string()),
                event: HookEvent::PreToolUse,
                matcher: None,
                action: super::types::HookAction::Log,
                priority: -1000,
            },
        ];

        for config in builtin_configs {
            match self.registry.register_config(config) {
                Ok(id) => ids.push(id),
                Err(e) => {
                    tracing::warn!("Failed to register builtin hook: {}", e);
                }
            }
        }

        tracing::info!(count = ids.len(), "Registered builtin hooks");
        ids
    }
}

// ── Hook 加载器 ────────────────────────────────────────────────────────────────

/// Hook 加载器 —— 从 HookSpec 创建可执行的 hook。
pub struct HookLoader;

impl HookLoader {
    /// 从 HookSpec 创建配置型 HookConfig。
    pub fn spec_to_config(spec: &HookSpec) -> Result<HookConfig, AppError> {
        let action = match &spec.handler {
            HookHandler::Command { cmd } => {
                if cmd.is_empty() {
                    return Err(AppError::invalid_input("Command handler requires non-empty cmd"));
                }
                super::types::HookAction::Custom(format!("command:{}", cmd))
            }
            HookHandler::Http { url, .. } => {
                if url.is_empty() {
                    return Err(AppError::invalid_input("Http handler requires non-empty url"));
                }
                super::types::HookAction::Custom(format!("http:{}", url))
            }
        };

        Ok(HookConfig {
            id: Some(spec.name.clone()),
            event: spec.event,
            matcher: spec.matcher.clone(),
            action,
            priority: 0,
        })
    }

    /// 验证 HookSpec 配置。
    pub fn validate_spec(spec: &HookSpec) -> Result<(), AppError> {
        if spec.name.is_empty() {
            return Err(AppError::invalid_input("Hook name cannot be empty"));
        }

        if !spec.enabled {
            return Ok(());
        }

        match &spec.handler {
            HookHandler::Command { cmd } => {
                if cmd.is_empty() {
                    return Err(AppError::invalid_input(format!(
                        "Hook '{}' has empty command",
                        spec.name
                    )));
                }
            }
            HookHandler::Http { url, .. } => {
                if url.is_empty() {
                    return Err(AppError::invalid_input(format!(
                        "Hook '{}' has empty URL",
                        spec.name
                    )));
                }
                super::runner::validate_url_for_ssrf(url)?;
            }
        }

        if let Some(matcher) = &spec.matcher {
            if let Some(pattern) = &matcher.tool_name_pattern {
                match pattern {
                    super::types::MatcherPattern::Regex(re) => {
                        regex::Regex::new(re)
                            .map_err(|e| AppError::invalid_input(format!(
                                "Hook '{}' has invalid regex pattern '{}': {}",
                                spec.name, re, e
                            )))?;
                    }
                    super::types::MatcherPattern::Glob(glob) => {
                        glob::Pattern::new(glob)
                            .map_err(|e| AppError::invalid_input(format!(
                                "Hook '{}' has invalid glob pattern '{}': {}",
                                spec.name, glob, e
                            )))?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

/// 默认 hook 配置文件名。
pub const DEFAULT_HOOK_CONFIG_FILE: &str = "hooks.json";

/// 默认 hook 目录名。
pub const DEFAULT_HOOK_DIR: &str = "hooks";

/// 获取默认 hook 配置路径。
pub fn default_hook_config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DEFAULT_HOOK_CONFIG_FILE)
}

/// 获取默认 hook 目录路径。
pub fn default_hook_dir_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DEFAULT_HOOK_DIR)
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_to_config() {
        let spec = HookSpec::new(
            "test-hook",
            HookEvent::PreToolUse,
            HookHandler::Command {
                cmd: "echo hello".to_string(),
            },
        );

        let config = HookLoader::spec_to_config(&spec).unwrap();
        assert_eq!(config.id, Some("test-hook".to_string()));
        assert_eq!(config.event, HookEvent::PreToolUse);
    }

    #[test]
    fn test_validate_spec_empty_name() {
        let spec = HookSpec::new(
            "",
            HookEvent::PreToolUse,
            HookHandler::Command { cmd: "test".to_string() },
        );
        assert!(HookLoader::validate_spec(&spec).is_err());
    }

    #[test]
    fn test_validate_spec_empty_command() {
        let spec = HookSpec::new(
            "test",
            HookEvent::PreToolUse,
            HookHandler::Command { cmd: "".to_string() },
        );
        assert!(HookLoader::validate_spec(&spec).is_err());
    }

    #[test]
    fn test_validate_spec_ssrf_url() {
        let spec = HookSpec::new(
            "test",
            HookEvent::PreToolUse,
            HookHandler::Http {
                url: "https://localhost/hook".to_string(),
                method: "POST".to_string(),
                headers: Default::default(),
                timeout_ms: 30000,
            },
        );
        assert!(HookLoader::validate_spec(&spec).is_err());
    }
}