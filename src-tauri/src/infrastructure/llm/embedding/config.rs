//! ═══════════════════════════════════════════════════════════════════════════
//! Embedding 配置 - 配置读写服务
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;
use crate::shared::error::AppError;
use super::types::EmbeddingConfig;

// ── 配置读写 ────────────────────────────────────────────────────────────────

/// 从 config.json 读取 embedding 配置
///
/// 如果配置不存在或解析失败，返回默认配置。
///
/// # 参数
/// - `config_path`: config.json 文件路径
///
/// # 返回值
/// 返回 EmbeddingConfig，失败时返回默认值
pub fn read_embedding_config(config_path: &Path) -> EmbeddingConfig {
    if let Ok(data) = std::fs::read_to_string(config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(ai) = json.get("ai") {
                if let Some(emb) = ai.get("embedding") {
                    if let Ok(cfg) = serde_json::from_value::<EmbeddingConfig>(emb.clone()) {
                        return cfg;
                    }
                }
            }
        }
    }
    EmbeddingConfig::default()
}

/// 写入 embedding 配置到 config.json（保留其他字段）
///
/// # 参数
/// - `config_path`: config.json 文件路径
/// - `cfg`: Embedding 配置
///
/// # 返回值
/// 成功返回 Ok(())，失败返回错误
pub fn write_embedding_config(config_path: &Path, cfg: &EmbeddingConfig) -> Result<(), AppError> {
    // 读取现有配置或创建空对象
    let mut json: serde_json::Value = if let Ok(data) = std::fs::read_to_string(config_path) {
        serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 确保 ai 对象存在
    if json.get("ai").is_none() {
        json["ai"] = serde_json::json!({});
    }

    // 设置 embedding 配置
    json["ai"]["embedding"] = serde_json::to_value(cfg)
        .map_err(|e| AppError::internal(format!("Failed to serialize embedding config: {}", e)))?;

    // 写入文件
    let pretty = serde_json::to_string_pretty(&json)
        .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
    std::fs::write(config_path, pretty)
        .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;

    Ok(())
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = EmbeddingConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.model, "nomic-embed-text");
        assert_eq!(cfg.dim, 768);
    }
}