//! ═══════════════════════════════════════════════════════════════════════════
//! 材料检索 - 关键词匹配与评分
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashSet;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

use super::types::{MaterialAsset, RetrieveMaterialsInput, RetrievedMaterial};

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 12;
const SNIPPET_RADIUS: usize = 700;
const MAX_TERMS: usize = 24;

/// 检索材料,按相关度倒序返回。
pub fn retrieve_materials(
    data_dir: &DataDir,
    input: &RetrieveMaterialsInput,
) -> Result<Vec<RetrievedMaterial>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        query_len = input.query.len(),
        limit = input.limit,
        "[MaterialRetrieve] Starting retrieval"
    );
    
    let query = input.query.trim();
    if query.is_empty() {
        return Err(AppError::invalid_input("query cannot be empty"));
    }
    let terms = extract_terms(query);
    let limit = normalize_limit(input.limit);

    let assets = list_material_assets(data_dir)?;
    tracing::debug!(
        asset_count = assets.len(),
        "[MaterialRetrieve] Loaded material assets"
    );
    
    let mut results: Vec<RetrievedMaterial> = Vec::new();

    for asset in assets {
        if let Some(want) = &input.purpose {
            if asset.purpose != *want {
                continue;
            }
        }
        let markdown_path = data_dir.materials_dir().join(&asset.markdown_path);
        let markdown = match std::fs::read_to_string(&markdown_path) {
            Ok(s) => s,
            Err(_) => continue, // 正文缺失则跳过,不阻断检索
        };
        let score = score_material(&asset, &markdown, &terms);
        if !terms.is_empty() && score <= 0.0 {
            continue;
        }
        let snippet = build_snippet(&markdown, &terms);
        results.push(RetrievedMaterial {
            id: asset.id.clone(),
            title: asset.title.clone(),
            kind: asset.kind.clone(),
            purpose: asset.purpose.clone(),
            source: asset.source.clone(),
            markdown_path: asset.markdown_path.clone(),
            score,
            excerpt: snippet.excerpt,
            char_start: snippet.char_start,
            char_end: snippet.char_end,
        });
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.cmp(&b.title))
    });
    results.truncate(limit as usize);
    
    tracing::info!(
        result_count = results.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "[MaterialRetrieve] Retrieval completed"
    );
    Ok(results)
}

/// 列出所有已导入材料的清单(枚举 materials_dir 下的 .json 文件)。
pub fn list_material_assets(data_dir: &DataDir) -> Result<Vec<MaterialAsset>, AppError> {
    let dir = data_dir.materials_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };
    let mut assets = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        match serde_json::from_str::<MaterialAsset>(&raw) {
            Ok(asset) if !asset.id.is_empty() && !asset.title.is_empty() => assets.push(asset),
            _ => continue, // 跳过损坏/陈旧的清单,不阻断检索
        }
    }
    Ok(assets)
}

/// 删除指定 id 的材料(markdown + 清单)。
pub fn delete_material(data_dir: &DataDir, id: &str) -> Result<bool, AppError> {
    if id.is_empty() {
        return Err(AppError::invalid_input("id cannot be empty"));
    }
    // 校验 id 不含路径分隔符,防止越权删除
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(AppError::invalid_input("id contains illegal characters"));
    }
    let dir = data_dir.materials_dir();
    let md_path = dir.join(format!("{}.md", id));
    let json_path = dir.join(format!("{}.json", id));
    let mut removed = false;
    if md_path.exists() {
        std::fs::remove_file(&md_path)
            .map_err(|_e| AppError::file_write_error(format!("material markdown: {}", id)))?;
        removed = true;
    }
    if json_path.exists() {
        std::fs::remove_file(&json_path)
            .map_err(|_e| AppError::file_write_error(format!("material json: {}", id)))?;
        removed = true;
    }
    Ok(removed)
}

fn score_material(asset: &MaterialAsset, markdown: &str, terms: &[String]) -> f64 {
    if terms.is_empty() {
        return 1.0;
    }
    let title = asset.title.to_lowercase();
    let source = asset.source.to_lowercase();
    let body = markdown.to_lowercase();
    let mut score = 0.0_f64;
    for term in terms {
        let t = term.to_lowercase();
        if title.contains(&t) {
            score += 8.0;
        }
        if source.contains(&t) {
            score += 4.0;
        }
        if let Some(pos) = body.find(&t) {
            // 位置加成:越靠前加成越高(0 处 +2,每 4000 字符衰减 1,衰减到 0 截止)
            let position_bonus = (2.0 - pos as f64 / 4000.0).max(0.0);
            score += 2.0 + position_bonus;
        }
    }
    score
}

struct Snippet {
    excerpt: String,
    char_start: u32,
    char_end: u32,
}

fn build_snippet(markdown: &str, terms: &[String]) -> Snippet {
    let lower = markdown.to_lowercase();
    let mut hit_char: Option<usize> = None;
    for term in terms {
        let t = term.to_lowercase();
        if let Some(byte_idx) = lower.find(&t) {
            // 字节位置 → 字符位置
            let char_idx = lower[..byte_idx].chars().count();
            match hit_char {
                None => hit_char = Some(char_idx),
                Some(prev) if char_idx < prev => hit_char = Some(char_idx),
                _ => {}
            }
        }
    }
    let total_chars = markdown.chars().count();
    let center = hit_char.unwrap_or_else(|| total_chars.min(500));
    let start = center.saturating_sub(SNIPPET_RADIUS);
    let end = (center + SNIPPET_RADIUS).min(total_chars);
    let excerpt: String = markdown.chars().skip(start).take(end - start).collect::<String>()
        .trim()
        .to_string();
    Snippet {
        excerpt,
        char_start: start as u32,
        char_end: end as u32,
    }
}

fn normalize_limit(limit: Option<u32>) -> u32 {
    match limit {
        Some(n) => n.clamp(1, MAX_LIMIT),
        None => DEFAULT_LIMIT,
    }
}

fn extract_terms(query: &str) -> Vec<String> {
    let raw = query.trim().to_lowercase();
    if raw.is_empty() {
        return Vec::new();
    }
    let re = regex::Regex::new(r"[\p{L}\p{N}]{2,}").expect("terms regex");
    let mut terms: HashSet<String> = HashSet::new();
    for m in re.find_iter(&raw) {
        terms.insert(m.as_str().to_string());
    }
    terms.into_iter().take(MAX_TERMS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_terms_chinese_and_english() {
        let terms = extract_terms("Hello 三体 world");
        assert!(terms.contains(&"hello".to_string()));
        assert!(terms.contains(&"三体".to_string()));
        assert!(terms.contains(&"world".to_string()));
    }

    #[test]
    fn extract_terms_drops_single_chars() {
        let terms = extract_terms("a b 12");
        // 单字符(英文)被丢弃;中文单字也会被丢弃(因为 {2,})
        assert!(!terms.contains(&"a".to_string()));
        assert!(terms.contains(&"12".to_string()));
    }

    #[test]
    fn normalize_limit_clamps() {
        assert_eq!(normalize_limit(None), 5);
        assert_eq!(normalize_limit(Some(0)), 1);
        assert_eq!(normalize_limit(Some(100)), 12);
        assert_eq!(normalize_limit(Some(7)), 7);
    }

    #[test]
    fn build_snippet_finds_earliest_hit() {
        let md = "prefix 世界 middle end";
        let terms = vec!["世界".to_string()];
        let s = build_snippet(md, &terms);
        assert!(s.excerpt.contains("世界"));
        assert!(s.char_end > s.char_start);
    }
}
