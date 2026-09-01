//! ═══════════════════════════════════════════════════════════════════════════
//! SkillManager - 技能管理器
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use super::types::*;
use super::parser::parse_frontmatter;
use crate::shared::error::AppError;

const MAX_SCAN_DEPTH: usize = 6;
const MAX_SKILLS_DIRS_PER_ROOT: usize = 2000;
const MAX_NAME_LEN: usize = 64;
const MAX_DESCRIPTION_LEN: usize = 1024;

pub struct SkillManager {
    skills: HashMap<String, Skill>,
    dirs: Vec<(PathBuf, SkillScope)>,
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            dirs: Vec::new(),
        }
    }

    pub fn add_dir(&mut self, dir: PathBuf) {
        self.add_dir_with_scope(dir, SkillScope::User);
    }

    pub fn add_dir_with_scope(&mut self, dir: PathBuf, scope: SkillScope) {
        if !self.dirs.iter().any(|(d, _)| d == &dir) {
            self.dirs.push((dir, scope));
        }
    }

    pub fn discover(&mut self) -> Result<(), AppError> {
        self.skills.clear();
        let dirs: Vec<(PathBuf, SkillScope)> = self.dirs.clone();
        for (dir, scope) in &dirs {
            if dir.exists() {
                self.discover_in_dir(dir, 0, *scope, &mut 0)?;
            }
        }
        Ok(())
    }

    fn discover_in_dir(
        &mut self,
        dir: &Path,
        depth: usize,
        scope: SkillScope,
        dirs_scanned: &mut usize,
    ) -> Result<(), AppError> {
        if depth > MAX_SCAN_DEPTH {
            return Ok(());
        }
        if *dirs_scanned > MAX_SKILLS_DIRS_PER_ROOT {
            tracing::warn!(
                dir = %dir.display(),
                limit = MAX_SKILLS_DIRS_PER_ROOT,
                "Skill scan limit reached, skipping remaining subdirectories"
            );
            return Ok(());
        }

        *dirs_scanned += 1;
        let entries = fs::read_dir(dir)
            .map_err(|e| AppError::internal(format!("Failed to read skill dir: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::internal(format!("Entry error: {}", e)))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') || name == "node_modules" {
                continue;
            }

            if path.is_dir() {
                self.discover_in_dir(&path, depth + 1, scope, dirs_scanned)?;
            } else if name == "SKILL.md" || name == "skill.md" {
                match self.parse_skill_file(&path, scope) {
                    Ok(skill) => {
                        self.skills.insert(skill.meta.name.clone(), skill);
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e.message, "Skipping invalid SKILL.md");
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_skill_file(&self, path: &Path, scope: SkillScope) -> Result<Skill, AppError> {
        let content = crate::infrastructure::fs::fs_utils::read_file(path)?;
        let (meta, body) = parse_frontmatter(&content)?;

        if meta.name.chars().count() > MAX_NAME_LEN {
            return Err(AppError::invalid_format(format!(
                "Skill name exceeds {} chars: {}",
                MAX_NAME_LEN, meta.name
            )));
        }
        if meta.description.chars().count() > MAX_DESCRIPTION_LEN {
            return Err(AppError::invalid_format(format!(
                "Skill description exceeds {} chars: {}",
                MAX_DESCRIPTION_LEN, meta.description
            )));
        }

        Ok(Skill {
            meta,
            content: body,
            path: path.to_string_lossy().to_string(),
            history: Vec::new(),
            scope,
        })
    }

    pub fn load(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn create_skill(&mut self, meta: SkillMeta, content: &str) -> Result<Skill, AppError> {
        let skill_dir = self.dirs.first().map(|(d, _)| d.clone())
            .ok_or_else(|| AppError::internal("No skill directory configured"))?;

        fs::create_dir_all(&skill_dir)
            .map_err(|e| AppError::file_write_error(format!("{}: {}", skill_dir.to_string_lossy(), e)))?;

        let skill_name = meta.name.replace(' ', "_").to_lowercase();
        let skill_path = skill_dir.join(&skill_name).join("SKILL.md");
        let parent = skill_path.parent()
            .ok_or_else(|| AppError::internal("skill_path has no parent (malformed path)"))?;

        fs::create_dir_all(parent)
            .map_err(|e| AppError::file_write_error(format!("{}: {}", parent.to_string_lossy(), e)))?;

        let frontmatter = serde_yaml::to_string(&meta)
            .map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;
        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);

        fs::write(&skill_path, &file_content)
            .map_err(|e| AppError::file_write_error(format!("{}: {}", skill_path.to_string_lossy(), e)))?;

        tracing::info!(name = %meta.name, path = %skill_path.display(), "Skill created");

        let scope = self.dirs.first().map(|(_, s)| *s).unwrap_or(SkillScope::User);
        let skill = Skill {
            meta,
            content: content.to_string(),
            path: skill_path.to_string_lossy().to_string(),
            history: Vec::new(),
            scope,
        };
        self.skills.insert(skill.meta.name.clone(), skill.clone());
        Ok(skill)
    }

    pub fn update_skill(&mut self, name: &str, meta: SkillMeta, content: &str) -> Result<Skill, AppError> {
        let (skill_path, scope) = {
            let existing = self.skills.get(name)
                .ok_or_else(|| AppError::skill_not_found(name))?;
            (existing.path.clone(), existing.scope)
        };

        let skill_path = Path::new(&skill_path);
        let frontmatter = serde_yaml::to_string(&meta)
            .map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;
        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);

        fs::write(skill_path, &file_content)
            .map_err(|_e| AppError::file_write_error(skill_path.to_string_lossy()))?;

        tracing::info!(name = %name, "Skill updated");

        if name != meta.name {
            self.skills.remove(name);
        }
        let skill = Skill {
            meta,
            content: content.to_string(),
            path: skill_path.to_string_lossy().to_string(),
            history: Vec::new(),
            scope,
        };
        self.skills.insert(skill.meta.name.clone(), skill.clone());
        Ok(skill)
    }

    pub fn delete_skill(&mut self, name: &str) -> Result<(), AppError> {
        let skill = self.skills.get(name)
            .ok_or_else(|| AppError::skill_not_found(name))?;
        let skill_path = Path::new(&skill.path);
        let skill_dir = skill_path.parent().unwrap_or(skill_path);

        if skill_dir.exists() {
            fs::remove_dir_all(skill_dir)
                .map_err(|e| AppError::internal(format!("Failed to delete skill directory: {}", e)))?;
        }

        self.skills.remove(name);
        tracing::info!(name = %name, "Skill deleted");
        Ok(())
    }

    /// 计算技能与任务描述的相关性分数
    pub fn score_relevance(&self, task_description: &str, skill: &Skill) -> f64 {
        let task_lower = task_description.to_lowercase();
        let mut score: f64 = 0.0;

        // 名称匹配权重最高
        if task_lower.contains(&skill.meta.name.to_lowercase()) {
            score += 0.5;
        }

        // 描述关键词匹配
        let desc_lower = skill.meta.description.to_lowercase();
        for word in desc_lower.split_whitespace() {
            if word.len() > 3 && task_lower.contains(word) {
                score += 0.1;
            }
        }

        // 限制在 [0.0, 1.0] 范围
        score.min(1.0)
    }

    /// 根据任务描述筛选最相关的技能
    pub fn find_relevant_skills(&self, task_description: &str, top_k: usize) -> Vec<&Skill> {
        let mut scored: Vec<_> = self.skills.values()
            .map(|skill| (skill, self.score_relevance(task_description, skill)))
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(skill, _)| skill).collect()
    }
}
