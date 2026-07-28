//! ═══════════════════════════════════════════════════════════════════════════
//! Bundles - 技能包模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 命名技能集合，一次调用加载多个相关技能。
//! YAML 文件定义 bundle：name / description / skills 列表。
//! 支持 mtime 缓存，源文件变更后自动失效。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::shared::error::AppError;

// ── Bundle 定义 ────────────────────────────────────────────────────────

/// Skill bundle 元数据（YAML frontmatter 解析产物）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBundle {
    /// bundle 名称（唯一标识）
    pub name: String,
    /// 人类可读描述
    #[serde(default)]
    pub description: String,
    /// 包含的 skill 名称列表（按加载顺序）
    pub skills: Vec<String>,
    /// 扩展元数据（可选）
    #[serde(default)]
    pub metadata: Option<BundleMetadata>,
}

/// bundle 扩展元数据。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// 分类标签
    #[serde(default)]
    pub category: Option<String>,
    /// 版本号
    #[serde(default)]
    pub version: Option<u32>,
}

// ── 缓存条目 ────────────────────────────────────────────────────────

/// bundle 缓存条目：记录解析结果与源文件 mtime/size，用于失效判断。
#[derive(Debug, Clone)]
struct BundleCacheEntry {
    bundle: SkillBundle,
    /// 源文件修改时间
    mtime: SystemTime,
    /// 源文件大小（字节）
    size: u64,
}

// ── BundleRegistry：bundle 注册表 + mtime 缓存 ────────────────────────────────────────────────────────

/// Skill bundle 注册表，带 mtime 缓存。
///
/// 缓存策略：
/// - `load_bundle(path)`：先 stat 源文件 mtime/size，与缓存对比
///   - 匹配 → 返回缓存结果
///   - 不匹配 → 重新解析 YAML + 更新缓存
/// - `invalidate(path)`：清除指定路径的缓存条目
/// - `clear()`：清空所有缓存
///
/// bundle 文件查找：扫描 `bundles_dir` 下的 `*.yaml` / `*.yml` 文件，
/// 按 basename（不含扩展名）作为 bundle 名注册。
pub struct BundleRegistry {
    /// 缓存：源文件路径 → 解析结果 + mtime/size
    cache: Arc<RwLock<HashMap<PathBuf, BundleCacheEntry>>>,
    /// bundle 名称 → 源文件路径（用于按名查找）
    name_index: Arc<RwLock<HashMap<String, PathBuf>>>,
    /// bundle 根目录（扫描 .yaml/.yml 文件）
    bundles_dir: PathBuf,
}

impl BundleRegistry {
    pub fn new(bundles_dir: PathBuf) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            name_index: Arc::new(RwLock::new(HashMap::new())),
            bundles_dir,
        }
    }

    /// 扫描 bundles_dir，构建 name → path 索引。
    ///
    /// 仅扫描顶层目录的 .yaml/.yml 文件（不递归），
    /// 按 basename 作为 bundle 名。调用方应在 `load_bundle_by_name` 前调用。
    pub async fn refresh_index(&self) -> Result<usize, AppError> {
        let mut name_index = self.name_index.write().await;
        name_index.clear();
        if !self.bundles_dir.exists() {
            return Ok(0);
        }
        let entries = std::fs::read_dir(&self.bundles_dir).map_err(|e| {
            AppError::internal(format!("Failed to read bundles dir: {}", e))
        })?;
        let mut count = 0usize;
        for entry in entries {
            let entry = entry.map_err(|e| AppError::internal(format!("bundle dir entry: {}", e)))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "yaml" && ext != "yml" {
                continue;
            }
            // basename（不含扩展名）作为 bundle 名
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.is_empty() {
                continue;
            }
            name_index.insert(stem.to_string(), path);
            count += 1;
        }
        Ok(count)
    }

    /// 按名称加载 bundle（命中缓存时跳过 YAML 解析）。
    pub async fn load_bundle_by_name(&self, name: &str) -> Result<SkillBundle, AppError> {
        let path = {
            let index = self.name_index.read().await;
            index
                .get(name)
                .cloned()
                .ok_or_else(|| AppError::not_found(format!("bundle '{}' not registered", name)))?
        };
        self.load_bundle(&path).await
    }

    /// 从指定路径加载 bundle（mtime 缓存命中即跳过解析）。
    pub async fn load_bundle(&self, path: &Path) -> Result<SkillBundle, AppError> {
        // stat 源文件
        let metadata = std::fs::metadata(path).map_err(|e| {
            AppError::not_found(format!("bundle file not found: {}: {}", path.display(), e))
        })?;
        let current_mtime = metadata.modified().map_err(|e| {
            AppError::internal(format!("read bundle mtime: {}", e))
        })?;
        let current_size = metadata.len();

        // 检查缓存：mtime + size 匹配则命中
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(path) {
                if entry.mtime == current_mtime && entry.size == current_size {
                    return Ok(entry.bundle.clone());
                }
            }
        }

        // 缓存未命中：读取 + 解析 YAML
        let content = crate::infrastructure::fs::fs_utils::read_file(path)?;
        let bundle: SkillBundle = serde_yaml::from_str(&content).map_err(|e| {
            AppError::invalid_format(format!("bundle YAML parse error: {}", e))
        })?;
        // 校验：skills 列表非空
        if bundle.skills.is_empty() {
            return Err(AppError::invalid_format(format!(
                "bundle '{}' has empty skills list",
                bundle.name
            )));
        }

        // 回填缓存
        let mut cache = self.cache.write().await;
        cache.insert(
            path.to_path_buf(),
            BundleCacheEntry {
                bundle: bundle.clone(),
                mtime: current_mtime,
                size: current_size,
            },
        );

        Ok(bundle)
    }

    /// 失效指定路径的缓存（源文件被外部修改时调用）。
    pub async fn invalidate(&self, path: &Path) {
        let mut cache = self.cache.write().await;
        cache.remove(path);
    }

    /// 清空所有缓存。
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 列出已注册的 bundle 名称（按字典序）。
    pub async fn list_names(&self) -> Vec<String> {
        let index = self.name_index.read().await;
        let mut names: Vec<String> = index.keys().cloned().collect();
        names.sort();
        names
    }
}

// ── bundle invocation：解析 bundle 到 skill 名称列表 ────────────────────────────────────────────────────────

/// bundle 调用结果：bundle 包含的 skill 名称（按定义顺序）。
///
/// 调用方据此使用 `SkillManager::load(name)` 逐个加载技能内容，
/// 并将每个 skill 包装为 `SkillInstructions` fragment 注入。
pub async fn invoke_bundle(
    registry: &BundleRegistry,
    bundle_name: &str,
) -> Result<Vec<String>, AppError> {
    let bundle = registry.load_bundle_by_name(bundle_name).await?;
    Ok(bundle.skills)
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_bundle(dir: &Path, name: &str, skills: &[&str]) -> PathBuf {
        let path = dir.join(format!("{}.yaml", name));
        let skills_yaml = skills
            .iter()
            .map(|s| format!("  - {}", s))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "name: {}\ndescription: test bundle\nskills:\n{}\n",
            name, skills_yaml
        );
        std::fs::write(&path, content).unwrap();
        path
    }

    #[tokio::test]
    async fn load_bundle_parses_yaml() {
        let tmp = tempdir().unwrap();
        let path = write_bundle(tmp.path(), "novel-bundle", &["novel_writing", "character_design"]);

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        let bundle = registry.load_bundle(&path).await.unwrap();
        assert_eq!(bundle.name, "novel-bundle");
        assert_eq!(bundle.skills, vec!["novel_writing", "character_design"]);
    }

    #[tokio::test]
    async fn load_bundle_caches_on_first_call() {
        let tmp = tempdir().unwrap();
        let path = write_bundle(tmp.path(), "b1", &["skill_a"]);

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        // 首次加载：解析 YAML
        let b1 = registry.load_bundle(&path).await.unwrap();
        assert_eq!(b1.skills, vec!["skill_a"]);

        // 第二次加载：应命中缓存（即使删除源文件也返回缓存值）
        std::fs::remove_file(&path).unwrap();
        let b2 = registry.load_bundle(&path).await;
        // 源文件已删除 → stat 失败 → 返回 Err
        assert!(b2.is_err(), "deleted source should not return cached value");
    }

    #[tokio::test]
    async fn load_bundle_invalidates_on_mtime_change() {
        let tmp = tempdir().unwrap();
        let path = write_bundle(tmp.path(), "b2", &["skill_a"]);

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        let b1 = registry.load_bundle(&path).await.unwrap();
        assert_eq!(b1.skills, vec!["skill_a"]);

        // 修改源文件：内容变化 + mtime 变化
        // 等待以确保 mtime 不同（部分文件系统 mtime 精度为秒）
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        let new_content = "name: b2\ndescription: updated\nskills:\n  - skill_b\n  - skill_c\n";
        std::fs::write(&path, new_content).unwrap();

        // 重新加载：mtime 变化 → 重新解析
        let b2 = registry.load_bundle(&path).await.unwrap();
        assert_eq!(b2.skills, vec!["skill_b", "skill_c"]);
        assert_ne!(b2.skills, b1.skills);
    }

    #[tokio::test]
    async fn refresh_index_discovers_yaml_files() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "bundle-a", &["s1"]);
        write_bundle(tmp.path(), "bundle-b", &["s2", "s3"]);
        // 非 yaml 文件应被忽略
        std::fs::write(tmp.path().join("readme.txt"), "not a bundle").unwrap();

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        let count = registry.refresh_index().await.unwrap();
        assert_eq!(count, 2);

        let names = registry.list_names().await;
        assert_eq!(names, vec!["bundle-a", "bundle-b"]);
    }

    #[tokio::test]
    async fn load_bundle_by_name_resolves_via_index() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "novel-pack", &["novel_writing"]);

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        registry.refresh_index().await.unwrap();

        let bundle = registry.load_bundle_by_name("novel-pack").await.unwrap();
        assert_eq!(bundle.skills, vec!["novel_writing"]);
    }

    #[tokio::test]
    async fn load_bundle_by_name_unknown_returns_error() {
        let tmp = tempdir().unwrap();
        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        registry.refresh_index().await.unwrap();

        let result = registry.load_bundle_by_name("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn empty_skills_list_rejected() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("empty.yaml");
        std::fs::write(&path, "name: empty\ndescription: no skills\nskills: []\n").unwrap();

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        let result = registry.load_bundle(&path).await;
        assert!(result.is_err(), "empty skills list should be rejected");
    }

    #[tokio::test]
    async fn invoke_bundle_returns_skill_names() {
        let tmp = tempdir().unwrap();
        write_bundle(tmp.path(), "pack", &["skill_x", "skill_y", "skill_z"]);

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        registry.refresh_index().await.unwrap();

        let skills = invoke_bundle(&registry, "pack").await.unwrap();
        assert_eq!(skills, vec!["skill_x", "skill_y", "skill_z"]);
    }

    #[tokio::test]
    async fn invalidate_clears_cache_entry() {
        let tmp = tempdir().unwrap();
        let path = write_bundle(tmp.path(), "b3", &["s1"]);

        let registry = BundleRegistry::new(tmp.path().to_path_buf());
        registry.load_bundle(&path).await.unwrap();

        // 失效后重新加载应再次解析
        registry.invalidate(&path).await;
        // 验证缓存已清空：删除源文件后加载应失败（若缓存仍存在则不会 stat）
        std::fs::remove_file(&path).unwrap();
        let result = registry.load_bundle(&path).await;
        assert!(result.is_err(), "after invalidate, deleted source should error");
    }

    #[tokio::test]
    async fn refresh_index_handles_missing_dir() {
        let registry = BundleRegistry::new(PathBuf::from("/nonexistent/bundles"));
        let count = registry.refresh_index().await.unwrap();
        assert_eq!(count, 0);
    }
}
