//! ═══════════════════════════════════════════════════════════════════════════
//! Evolution - 技能进化模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 实现闭环学习系统：
//! - 任务完成检测自动创建技能
//! - 技能使用追踪统计和评分
//! - 技能自动改进定期优化
//! - 技能归档清理过时技能

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

// ── SkillUsage: 技能使用统计 ────────────────────────────────────────────────────────

/// 技能使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    /// 技能名称
    pub name: String,
    /// 使用次数
    pub use_count: u64,
    /// 查看次数
    pub view_count: u64,
    /// 修改次数
    pub patch_count: u64,
    /// 最后活动时间
    pub last_activity_at: DateTime<Utc>,
    /// 状态 (active / stale / archived)
    pub state: SkillState,
    /// 是否固定（固定技能不会被归档）
    pub pinned: bool,
    /// 创建来源 (agent / user / bundled)
    pub created_by: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl Default for SkillUsage {
    fn default() -> Self {
        Self {
            name: String::new(),
            use_count: 0,
            view_count: 0,
            patch_count: 0,
            last_activity_at: Utc::now(),
            state: SkillState::Active,
            pinned: false,
            created_by: "user".to_string(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillState {
    Active,
    Stale,
    Archived,
}

// ── SkillCreationRequest: 技能创建请求 ────────────────────────────────────────────────────────

/// 技能自动创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCreationRequest {
    /// 技能名称（自动生成或用户指定）
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 任务摘要
    pub task_summary: String,
    /// 学到的经验
    pub lessons_learned: Vec<String>,
    /// 工具使用记录
    pub tools_used: Vec<String>,
    /// 创建来源
    pub created_by: String,
    /// 分类
    pub category: String,
}

// ── CuratorConfig: Curator 配置 ────────────────────────────────────────────────────────

/// Curator 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorConfig {
    /// 是否启用
    pub enabled: bool,
    /// 检查间隔（小时）
    pub interval_hours: u64,
    /// 最小空闲时间（小时）
    pub min_idle_hours: u64,
    /// 标记为过时的天数
    pub stale_after_days: u64,
    /// 归档天数
    pub archive_after_days: u64,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_hours: 24,
            min_idle_hours: 4,
            stale_after_days: 30,
            archive_after_days: 90,
        }
    }
}

// ── SkillEvolutionManager: 技能进化管理器 ────────────────────────────────────────────────────────

/// 技能进化管理器
pub struct SkillEvolutionManager {
    usage_store: Arc<RwLock<HashMap<String, SkillUsage>>>,
    config: CuratorConfig,
    skills_dir: PathBuf,
    usage_file: PathBuf,
}

impl SkillEvolutionManager {
    pub fn new(skills_dir: PathBuf) -> Self {
        let usage_file = skills_dir.join(".usage.json");
        Self {
            usage_store: Arc::new(RwLock::new(HashMap::new())),
            config: CuratorConfig::default(),
            skills_dir,
            usage_file,
        }
    }

    pub fn with_config(mut self, config: CuratorConfig) -> Self {
        self.config = config;
        self
    }

    pub fn load(&self) -> Result<(), AppError> {
        if self.usage_file.exists() {
            let content = std::fs::read_to_string(&self.usage_file)
                .map_err(|e| AppError::internal(format!("Failed to read usage file: {}", e)))?;
            let usage: HashMap<String, SkillUsage> = serde_json::from_str(&content)
                .map_err(|e| AppError::internal(format!("Failed to parse usage file: {}", e)))?;
            let mut store = self.usage_store.write().unwrap();
            *store = usage;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), AppError> {
        let store = self.usage_store.read().unwrap();
        let content = serde_json::to_string_pretty(&*store)
            .map_err(|e| AppError::internal(format!("Failed to serialize usage: {}", e)))?;
        std::fs::write(&self.usage_file, content)
            .map_err(|_e| AppError::file_write_error(self.usage_file.display().to_string()))?;
        Ok(())
    }

    /// 记录技能使用
    pub fn record_use(&self, skill_name: &str) {
        let mut store = self.usage_store.write().unwrap();
        let usage = store.entry(skill_name.to_string()).or_insert_with(|| SkillUsage {
            name: skill_name.to_string(),
            created_by: "agent".to_string(),
            ..Default::default()
        });
        usage.use_count += 1;
        usage.last_activity_at = Utc::now();
        usage.state = SkillState::Active;
    }

    /// 记录技能查看
    pub fn record_view(&self, skill_name: &str) {
        let mut store = self.usage_store.write().unwrap();
        let usage = store.entry(skill_name.to_string()).or_insert_with(|| SkillUsage {
            name: skill_name.to_string(),
            created_by: "agent".to_string(),
            ..Default::default()
        });
        usage.view_count += 1;
        usage.last_activity_at = Utc::now();
    }

    /// 记录技能修改
    pub fn record_patch(&self, skill_name: &str) {
        let mut store = self.usage_store.write().unwrap();
        let usage = store.entry(skill_name.to_string()).or_insert_with(|| SkillUsage {
            name: skill_name.to_string(),
            created_by: "agent".to_string(),
            ..Default::default()
        });
        usage.patch_count += 1;
        usage.last_activity_at = Utc::now();
    }

    /// 标记技能固定
    pub fn pin(&self, skill_name: &str) -> Result<(), AppError> {
        let mut store = self.usage_store.write().unwrap();
        let usage = store.get_mut(skill_name)
            .ok_or_else(|| AppError::skill_not_found(skill_name))?;
        usage.pinned = true;
        Ok(())
    }

    /// 取消固定
    pub fn unpin(&self, skill_name: &str) -> Result<(), AppError> {
        let mut store = self.usage_store.write().unwrap();
        let usage = store.get_mut(skill_name)
            .ok_or_else(|| AppError::skill_not_found(skill_name))?;
        usage.pinned = false;
        Ok(())
    }

    /// 获取技能使用统计
    pub fn get_usage(&self, skill_name: &str) -> Option<SkillUsage> {
        let store = self.usage_store.read().unwrap();
        store.get(skill_name).cloned()
    }

    /// 列出所有使用统计
    pub fn list_usage(&self) -> Vec<SkillUsage> {
        let store = self.usage_store.read().unwrap();
        store.values().cloned().collect()
    }

    /// 运行 Curator 检查
    pub fn run_curator(&self) -> Result<CuratorReport, AppError> {
        let now = Utc::now();
        let mut report = CuratorReport::default();

        let mut store = self.usage_store.write().unwrap();
        for (name, usage) in store.iter_mut() {
            if usage.pinned {
                continue;
            }

            let age_days = (now - usage.last_activity_at).num_days() as u64;

            if usage.state == SkillState::Active && age_days > self.config.stale_after_days {
                usage.state = SkillState::Stale;
                report.marked_stale.push(name.clone());
            } else if usage.state == SkillState::Stale && age_days > self.config.archive_after_days {
                usage.state = SkillState::Archived;
                report.archived.push(name.clone());
            }
        }

        if !report.marked_stale.is_empty() || !report.archived.is_empty() {
            self.save()?;
        }

        Ok(report)
    }

    /// 创建技能（任务完成后自动调用）
    pub fn create_skill_from_task(&self, request: SkillCreationRequest) -> Result<String, AppError> {
        let skill_name = request.name.replace(' ', "_").to_lowercase();
        let skill_dir = self.skills_dir.join(&skill_name);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| AppError::file_write_error(format!("{}: {}", skill_dir.display(), e)))?;

        let lessons_section = if request.lessons_learned.is_empty() {
            String::new()
        } else {
            format!("\n## 经验教训\n\n{}\n", request.lessons_learned.iter()
                .map(|l| format!("- {}", l))
                .collect::<Vec<_>>()
                .join("\n"))
        };

        let tools_section = if request.tools_used.is_empty() {
            String::new()
        } else {
            format!("\n## 使用工具\n\n{}\n", request.tools_used.iter()
                .map(|t| format!("- `{}`", t))
                .collect::<Vec<_>>()
                .join("\n"))
        };

        let content = format!(
            "# {}\n\n{}\n\n{}\n{}",
            request.name,
            request.task_summary,
            lessons_section,
            tools_section
        );

        let meta = super::types::SkillMeta {
            name: request.name,
            description: request.description,
            category: request.category,
            requires_tools: request.tools_used,
            platforms: None,
            version: 1,
            tags: vec!["auto-created".to_string()],
            depends_on: vec![],
            metadata: None,
            policy: None,
        };

        let frontmatter = serde_yaml::to_string(&meta)
            .map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;

        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);

        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, &file_content)
            .map_err(|e| AppError::file_write_error(format!("{}: {}", skill_path.display(), e)))?;

        let mut store = self.usage_store.write().unwrap();
        store.insert(skill_name.clone(), SkillUsage {
            name: skill_name.clone(),
            use_count: 1,
            view_count: 0,
            patch_count: 0,
            last_activity_at: Utc::now(),
            state: SkillState::Active,
            pinned: false,
            created_by: request.created_by,
            created_at: Utc::now(),
        });

        self.save()?;

        Ok(skill_path.to_string_lossy().to_string())
    }
}

impl std::fmt::Debug for SkillEvolutionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillEvolutionManager")
            .field("config", &self.config)
            .field("skills_dir", &self.skills_dir)
            .finish()
    }
}

// ── CuratorReport: Curator 运行报告 ────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CuratorReport {
    pub marked_stale: Vec<String>,
    pub archived: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_record_use() {
        let dir = tempdir().unwrap();
        let manager = SkillEvolutionManager::new(dir.path().to_path_buf());

        manager.record_use("test_skill");
        let usage = manager.get_usage("test_skill").unwrap();
        assert_eq!(usage.use_count, 1);
        assert_eq!(usage.state, SkillState::Active);
    }

    #[test]
    fn test_pin_skill() {
        let dir = tempdir().unwrap();
        let manager = SkillEvolutionManager::new(dir.path().to_path_buf());

        manager.record_use("test_skill");
        manager.pin("test_skill").unwrap();

        let usage = manager.get_usage("test_skill").unwrap();
        assert!(usage.pinned);
    }
}