
//! ═══════════════════════════════════════════════════════════════════════════
//! Discovery - 技能发现与解析
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供技能扫描、解析和管理功能：
//! - 扫描目录自动发现 SKILL.md 文件
//! - 解析 frontmatter 元数据
//! - 技能创建、更新、删除

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use super::types::*;
use crate::shared::error::AppError;

// ── 扫描限制常量 ────────────────────────────────────────────────────────
//
// 防止恶意或错误配置的 skills 目录导致扫描爆炸
const MAX_SCAN_DEPTH: usize = 6;
const MAX_SKILLS_DIRS_PER_ROOT: usize = 2000;
const MAX_NAME_LEN: usize = 64;
const MAX_DESCRIPTION_LEN: usize = 1024;

pub struct SkillManager {
    skills: HashMap<String, Skill>,
    /// 已注册的扫描根目录及其作用域（按 System → Admin → User → Repo 顺序扫描）
    dirs: Vec<(PathBuf, SkillScope)>,
}

impl Default for SkillManager { fn default() -> Self { Self::new() } }

impl SkillManager {
    pub fn new() -> Self { Self { skills: HashMap::new(), dirs: Vec::new() } }

    /// 添加扫描根目录（默认 User 作用域）。
    pub fn add_dir(&mut self, dir: PathBuf) {
        self.add_dir_with_scope(dir, SkillScope::User);
    }

    /// 添加扫描根目录并显式指定作用域。
    ///
    /// 扫描顺序遵循注册顺序；建议按 System → Admin → User → Repo 注册，
    /// 前层注册的同名技能会被后层覆盖（即更靠近 repo 的技能优先）。
    pub fn add_dir_with_scope(&mut self, dir: PathBuf, scope: SkillScope) {
        if !self.dirs.iter().any(|(d, _)| d == &dir) {
            self.dirs.push((dir, scope));
        }
    }

    pub fn discover(&mut self) -> Result<(), AppError> {
        self.skills.clear();
        let dirs: Vec<(PathBuf, SkillScope)> = self.dirs.clone();
        for (dir, scope) in &dirs {
            if dir.exists() { self.discover_in_dir(dir, 0, *scope, &mut 0)?; }
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
        if depth > MAX_SCAN_DEPTH { return Ok(()); }
        if *dirs_scanned > MAX_SKILLS_DIRS_PER_ROOT {
            tracing::warn!(
                dir = %dir.display(),
                limit = MAX_SKILLS_DIRS_PER_ROOT,
                "Skill scan limit reached, skipping remaining subdirectories"
            );
            return Ok(());
        }
        *dirs_scanned += 1;
        let entries = fs::read_dir(dir).map_err(|e| AppError::internal(format!("Failed to read skill dir: {}", e)))?;
        for entry in entries {
            let entry = entry.map_err(|e| AppError::internal(format!("Entry error: {}", e)))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" { continue; }
            if path.is_dir() { self.discover_in_dir(&path, depth + 1, scope, dirs_scanned)?; }
            else if name == "SKILL.md" || name == "skill.md" {
                match self.parse_skill_file(&path, scope) {
                    Ok(skill) => { self.skills.insert(skill.meta.name.clone(), skill); }
                    Err(e) => tracing::warn!(path = %path.display(), error = %e.message, "Skipping invalid SKILL.md"),
                }
            }
        }
        Ok(())
    }

    fn parse_skill_file(&self, path: &Path, scope: SkillScope) -> Result<Skill, AppError> {
        let content = crate::infrastructure::fs::fs_utils::read_file(path)?;
        let (meta, body) = parse_frontmatter(&content)?;
        // 名称/描述长度校验（对照 codex 扫描限制）
        if meta.name.chars().count() > MAX_NAME_LEN {
            return Err(AppError::invalid_format(format!(
                "Skill name exceeds {} chars: {}", MAX_NAME_LEN, meta.name
            )));
        }
        if meta.description.chars().count() > MAX_DESCRIPTION_LEN {
            return Err(AppError::invalid_format(format!(
                "Skill description exceeds {} chars: {}", MAX_DESCRIPTION_LEN, meta.description
            )));
        }
        Ok(Skill { meta, content: body, path: path.to_string_lossy().to_string(), history: Vec::new(), scope })
    }

    pub fn load(&self, name: &str) -> Option<&Skill> { self.skills.get(name) }
    pub fn list(&self) -> Vec<&Skill> { self.skills.values().collect() }

    pub fn create_skill(&mut self, meta: SkillMeta, content: &str) -> Result<Skill, AppError> {
        let skill_dir = self.dirs.first().map(|(d, _)| d.clone()).ok_or_else(|| AppError::internal("No skill directory configured"))?;
        fs::create_dir_all(&skill_dir).map_err(|e| AppError::file_write_error(format!("{}: {}", skill_dir.to_string_lossy(), e)))?;
        let skill_name = meta.name.replace(' ', "_").to_lowercase();
        let skill_path = skill_dir.join(&skill_name).join("SKILL.md");
        let parent = skill_path.parent().ok_or_else(|| AppError::internal("skill_path has no parent (malformed path)"))?;
        fs::create_dir_all(parent).map_err(|e| AppError::file_write_error(format!("{}: {}", parent.to_string_lossy(), e)))?;
        let frontmatter = serde_yaml::to_string(&meta).map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;
        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);
        fs::write(&skill_path, &file_content).map_err(|e| AppError::file_write_error(format!("{}: {}", skill_path.to_string_lossy(), e)))?;
        tracing::info!(name = %meta.name, path = %skill_path.display(), "Skill created");
        let scope = self.dirs.first().map(|(_, s)| *s).unwrap_or(SkillScope::User);
        let skill = Skill { meta, content: content.to_string(), path: skill_path.to_string_lossy().to_string(), history: Vec::new(), scope };
        self.skills.insert(skill.meta.name.clone(), skill.clone());
        Ok(skill)
    }

    pub fn update_skill(&mut self, name: &str, meta: SkillMeta, content: &str) -> Result<Skill, AppError> {
        let (skill_path, scope) = {
            let existing = self.skills.get(name).ok_or_else(|| AppError::skill_not_found(name))?;
            (existing.path.clone(), existing.scope)
        };
        let skill_path = Path::new(&skill_path);
        let frontmatter = serde_yaml::to_string(&meta).map_err(|e| AppError::internal(format!("Failed to serialize skill meta: {}", e)))?;
        let file_content = format!("---\n{}---\n\n{}", frontmatter, content);
        fs::write(skill_path, &file_content).map_err(|_e| AppError::file_write_error(skill_path.to_string_lossy()))?;
        tracing::info!(name = %name, "Skill updated");
        if name != meta.name { self.skills.remove(name); }
        let skill = Skill { meta, content: content.to_string(), path: skill_path.to_string_lossy().to_string(), history: Vec::new(), scope };
        self.skills.insert(skill.meta.name.clone(), skill.clone());
        Ok(skill)
    }

    pub fn delete_skill(&mut self, name: &str) -> Result<(), AppError> {
        let skill = self.skills.get(name).ok_or_else(|| AppError::skill_not_found(name))?;
        let skill_path = Path::new(&skill.path);
        let skill_dir = skill_path.parent().unwrap_or(skill_path);
        if skill_dir.exists() { fs::remove_dir_all(skill_dir).map_err(|e| AppError::internal(format!("Failed to delete skill directory: {}", e)))?; }
        self.skills.remove(name);
        tracing::info!(name = %name, "Skill deleted");
        Ok(())
    }

    pub fn score_relevance(&self, task_description: &str, skill: &Skill) -> f64 {
        let task_lower = task_description.to_lowercase();
        let mut score = 0.0;
        if task_lower.contains(&skill.meta.name.to_lowercase()) { score += 0.4; }
        let desc_words: Vec<&str> = skill.meta.description.split_whitespace().collect();
        let task_words: Vec<&str> = task_lower.split_whitespace().collect();
        let overlap = desc_words.iter().filter(|w| task_words.contains(w)).count();
        score += (overlap as f64 / desc_words.len().max(1) as f64) * 0.3;
        for tag in &skill.meta.tags { if task_lower.contains(&tag.to_lowercase()) { score += 0.1; } }
        if task_lower.contains(&skill.meta.category.to_lowercase()) { score += 0.2; }
        score
    }

    pub fn find_relevant(&self, task_description: &str, top_k: usize) -> Vec<(&Skill, f64)> {
        let mut scored: Vec<(&Skill, f64)> = self.skills.values().map(|s| (s, self.score_relevance(task_description, s))).filter(|(_, score)| *score > 0.1).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).collect()
    }

    pub fn build_index(&self) -> String {
        if self.skills.is_empty() { return String::new(); }
        let mut categories: HashMap<String, Vec<&Skill>> = HashMap::new();
        for skill in self.skills.values() { categories.entry(skill.meta.category.clone()).or_default().push(skill); }
        let mut lines = vec!["## Available Skills".to_string(), "Before replying, check if a skill matches your task. Load it with the skill tool if relevant.".to_string(), String::new()];
        let mut cats: Vec<_> = categories.iter().collect();
        cats.sort_by_key(|(k, _)| (*k).clone());
        for (cat, skills) in cats {
            lines.push(format!("### {}", cat));
            for skill in skills {
                // 优先使用 metadata.short_description，回退到 description
                let desc = skill.meta.metadata.as_ref()
                    .and_then(|m| m.short_description.as_deref())
                    .unwrap_or(&skill.meta.description);
                lines.push(format!("- **{}**: {}", skill.meta.name, desc));
            }
            lines.push(String::new());
        }
        lines.join("\n")
    }

    /// 检测命令是否隐式匹配某个 skill（对照 codex `detect_implicit_skill_invocation_for_command`）。
    ///
    /// 匹配规则：命令以 skill 名称开头（如 `/skill_name args`）或命令文本包含 skill 名称。
    /// 返回首个匹配的 skill；调用方应使用 `seen_skills` set 去重，避免重复触发遥测。
    pub fn detect_implicit_invocation(&self, command: &str) -> Option<&Skill> {
        let cmd_lower = command.trim().to_lowercase();
        if cmd_lower.is_empty() { return None; }
        // 优先：命令以 skill 名称开头（如 "/novel_writing ..."）
        for skill in self.skills.values() {
            let name_lower = skill.meta.name.to_lowercase();
            if cmd_lower.starts_with(&name_lower) || cmd_lower.starts_with(&format!("/{}", name_lower)) {
                return Some(skill);
            }
        }
        // 回退：命令文本包含 skill 名称（较弱匹配）
        for skill in self.skills.values() {
            let name_lower = skill.meta.name.to_lowercase();
            if name_lower.len() >= 3 && cmd_lower.contains(&name_lower) {
                return Some(skill);
            }
        }
        None
    }
}

fn parse_frontmatter(content: &str) -> Result<(SkillMeta, String), AppError> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Ok((SkillMeta { name: "unnamed".into(), description: String::new(), category: "general".into(), requires_tools: Vec::new(), platforms: None, version: 1, tags: Vec::new(), depends_on: Vec::new(), metadata: None, policy: None }, content.to_string()));
    }
    // 用 split_once 避免手动字节索引（content[3..] / content[end+6..] 等），
    // str 方法天然在 char boundary 上切分，杜绝非 ASCII 边界 panic。
    // `get(3..)` 已验证 starts_with("---")，安全。
    let after_open = content.get(3..).unwrap_or("");
    let (yaml_str, rest) = after_open
        .split_once("---")
        .ok_or_else(|| AppError::invalid_format("Unclosed frontmatter"))?;
    let body = rest.trim().to_string();
    let meta: SkillMeta = serde_yaml::from_str(yaml_str).map_err(|e| AppError::invalid_format(format!("Invalid frontmatter YAML: {}", e)))?;
    Ok((meta, body))
}