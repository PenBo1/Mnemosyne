use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::*;
use crate::errors::AppError;

pub struct SkillManager {
    skills: HashMap<String, Skill>,
    dirs: Vec<PathBuf>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            dirs: Vec::new(),
        }
    }

    pub fn add_dir(&mut self, dir: PathBuf) {
        if !self.dirs.contains(&dir) {
            self.dirs.push(dir);
        }
    }

    pub fn discover(&mut self) -> Result<(), AppError> {
        self.skills.clear();
        let dirs: Vec<PathBuf> = self.dirs.clone();
        for dir in &dirs {
            if !dir.exists() {
                continue;
            }
            self.discover_in_dir(dir, 0)?;
        }
        Ok(())
    }

    fn discover_in_dir(&mut self, dir: &Path, depth: usize) -> Result<(), AppError> {
        if depth > 5 {
            return Ok(());
        }
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
                self.discover_in_dir(&path, depth + 1)?;
            } else if name == "SKILL.md" || name == "skill.md" {
                if let Ok(skill) = self.parse_skill_file(&path) {
                    self.skills.insert(skill.meta.name.clone(), skill);
                }
            }
        }
        Ok(())
    }

    fn parse_skill_file(&self, path: &Path) -> Result<Skill, AppError> {
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::internal(format!("Failed to read skill file: {}", e)))?;

        let (meta, body) = parse_frontmatter(&content)?;

        Ok(Skill {
            meta,
            content: body,
            path: path.to_string_lossy().to_string(),
        })
    }

    pub fn load(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn create_skill(&mut self, meta: SkillMeta, content: &str) -> Result<Skill, AppError> {
        let skill_dir = self.dirs.first()
            .ok_or_else(|| AppError::internal("No skill directory configured"))?;

        fs::create_dir_all(skill_dir)
            .map_err(|_e| AppError::file_write_error(skill_dir.to_string_lossy()))?;

        let skill_name = meta.name.replace(' ', "_").to_lowercase();
        let skill_path = skill_dir.join(&skill_name).join("SKILL.md");

        fs::create_dir_all(skill_path.parent().unwrap())
            .map_err(|_e| AppError::file_write_error(skill_path.to_string_lossy()))?;

        let frontmatter = serde_yaml::to_string(&meta)
            .map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;

        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);

        fs::write(&skill_path, &file_content)
            .map_err(|_e| AppError::file_write_error(skill_path.to_string_lossy()))?;

        tracing::info!(name = %meta.name, path = %skill_path.display(), "Skill created");

        let skill = Skill {
            meta,
            content: content.to_string(),
            path: skill_path.to_string_lossy().to_string(),
        };

        self.skills.insert(skill.meta.name.clone(), skill.clone());
        Ok(skill)
    }

    pub fn update_skill(&mut self, name: &str, meta: SkillMeta, content: &str) -> Result<Skill, AppError> {
        let skill_path = {
            let existing = self.skills.get(name)
                .ok_or_else(|| AppError::skill_not_found(name))?;
            existing.path.clone()
        };

        let skill_path = Path::new(&skill_path);

        let frontmatter = serde_yaml::to_string(&meta)
            .map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;

        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);

        fs::write(skill_path, &file_content)
            .map_err(|_e| AppError::file_write_error(skill_path.to_string_lossy()))?;

        tracing::info!(name = %name, "Skill updated");

        // Remove old entry if name changed
        if name != meta.name {
            self.skills.remove(name);
        }

        let skill = Skill {
            meta,
            content: content.to_string(),
            path: skill_path.to_string_lossy().to_string(),
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

    pub fn build_index(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }

        let mut categories: HashMap<String, Vec<&Skill>> = HashMap::new();
        for skill in self.skills.values() {
            categories
                .entry(skill.meta.category.clone())
                .or_default()
                .push(skill);
        }

        let mut lines = vec!["## Available Skills".to_string()];
        lines.push("Before replying, check if a skill matches your task. Load it with the skill tool if relevant.".to_string());
        lines.push(String::new());

        let mut cats: Vec<_> = categories.iter().collect();
        cats.sort_by_key(|(k, _)| (*k).clone());

        for (cat, skills) in cats {
            lines.push(format!("### {}", cat));
            for skill in skills {
                lines.push(format!(
                    "- **{}**: {}",
                    skill.meta.name, skill.meta.description
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

fn parse_frontmatter(content: &str) -> Result<(SkillMeta, String), AppError> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return Ok((
            SkillMeta {
                name: "unnamed".into(),
                description: String::new(),
                category: "general".into(),
                requires_tools: Vec::new(),
                platforms: None,
            },
            content.to_string(),
        ));
    }

    let end = content[3..]
        .find("---")
        .ok_or_else(|| AppError::invalid_format("Unclosed frontmatter"))?;
    let yaml_str = &content[3..end + 3];
    let body = content[end + 6..].trim().to_string();

    let meta: SkillMeta = serde_yaml::from_str(yaml_str)
        .map_err(|e| AppError::invalid_format(format!("Invalid frontmatter YAML: {}", e)))?;

    Ok((meta, body))
}
