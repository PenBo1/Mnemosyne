//! ═══════════════════════════════════════════════════════════════════════════
//! Parser - Frontmatter解析器
//! ═══════════════════════════════════════════════════════════════════════════

use super::types::SkillMeta;
use crate::shared::error::AppError;

/// 解析 SKILL.md 文件的 frontmatter
///
/// 支持标准的 YAML frontmatter 格式：
/// ```markdown
/// ---
/// name: skill_name
/// description: Skill description
/// ---
///
/// Skill content here...
/// ```
pub fn parse_frontmatter(content: &str) -> Result<(SkillMeta, String), AppError> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Ok((SkillMeta {
            name: "unnamed".into(),
            description: String::new(),
            category: "general".into(),
            requires_tools: Vec::new(),
            platforms: None,
            version: 1,
            tags: Vec::new(),
            depends_on: Vec::new(),
            metadata: None,
            policy: None,
        }, content.to_string()));
    }

    let after_open = content.get(3..).unwrap_or("");
    let (yaml_str, rest) = after_open
        .split_once("---")
        .ok_or_else(|| AppError::invalid_format("Unclosed frontmatter"))?;

    let body = rest.trim().to_string();
    let meta: SkillMeta = serde_yaml::from_str(yaml_str)
        .map_err(|e| AppError::invalid_format(format!("Invalid frontmatter YAML: {}", e)))?;

    Ok((meta, body))
}