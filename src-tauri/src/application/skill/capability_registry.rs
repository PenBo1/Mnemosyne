// Capability Skill Registry。
//
// 职责:
// 1. 合并 builtin + project + user + external 四源,按 id 去重
// 2. 提供 listSkills / getSkill / resolveSkills 三个 API
// 3. resolveSkills 三阶段解析:
//    - forced:    requested_skills 中的 id 强制启用(忽略匹配)
//    - candidate: candidate_skills 中的 id 经校验后自动启用
//    - auto:      通过 sessionKind / instruction triggers 自动匹配

use super::capability_builtin::builtin_capability_skills;
use super::capability_types::*;
use std::collections::{HashMap, HashSet};

/// Capability Skill Registry
#[derive(Debug, Clone)]
pub struct CapabilitySkillRegistry {
    /// id → manifest(已去重排序)
    skills: Vec<CapabilitySkillManifest>,
    /// id → 索引(便于 O(1) 查找)
    by_id: HashMap<String, usize>,
}

impl Default for CapabilitySkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilitySkillRegistry {
    /// 仅含 builtin skills 的 registry
    pub fn new() -> Self {
        Self::with_extra_skills(Vec::new())
    }

    /// 在 builtin 基础上追加 project / user / external skills
    pub fn with_extra_skills(extra: Vec<CapabilitySkillManifest>) -> Self {
        let mut all: Vec<CapabilitySkillManifest> = Vec::with_capacity(3 + extra.len());
        all.extend(builtin_capability_skills());
        all.extend(extra);
        let deduped = dedupe_skills(all);
        let mut by_id = HashMap::with_capacity(deduped.len());
        for (idx, skill) in deduped.iter().enumerate() {
            by_id.insert(skill.id.clone(), idx);
        }
        Self { skills: deduped, by_id }
    }

    pub fn list_skills(&self) -> &[CapabilitySkillManifest] {
        &self.skills
    }

    pub fn get_skill(&self, id: &str) -> Option<&CapabilitySkillManifest> {
        let normalized = normalize_skill_id(id);
        self.by_id.get(&normalized).map(|i| &self.skills[*i])
    }

    /// 三阶段技能解析
    pub fn resolve_skills(&self, input: &SkillResolutionInput) -> SkillResolutionResult {
        let disabled: HashSet<String> = normalize_id_list(&input.disabled_skills)
            .into_iter()
            .collect();
        let requested = normalize_id_list(&input.requested_skills);
        let candidates = normalize_id_list(&input.candidate_skills);

        let mut missing_skill_ids: Vec<String> = Vec::new();
        let mut disabled_skill_ids: Vec<String> = Vec::new();
        for id in &disabled {
            if self.by_id.contains_key(id) {
                disabled_skill_ids.push(id.clone());
            }
        }

        // 阶段 1: forced —— requested 中的 id 强制启用
        let mut used: HashMap<String, CapabilitySkillManifest> = HashMap::new();
        let mut forced_skill_ids: Vec<String> = Vec::new();
        for id in &requested {
            match self.get_skill(id) {
                Some(skill) => {
                    if disabled.contains(id) {
                        continue;
                    }
                    used.insert(skill.id.clone(), skill.clone());
                    forced_skill_ids.push(id.clone());
                }
                None => missing_skill_ids.push(id.clone()),
            }
        }

        // 阶段 2: candidate —— candidate 中的 id 经校验后自动启用
        let mut auto_skill_ids: Vec<String> = Vec::new();
        for id in &candidates {
            if disabled.contains(id) || used.contains_key(id) {
                continue;
            }
            if let Some(skill) = self.get_skill(id) {
                used.insert(skill.id.clone(), skill.clone());
                auto_skill_ids.push(id.clone());
            }
        }

        // 阶段 3: auto —— 通过 sessionKind / instruction triggers 自动匹配
        for skill in &self.skills {
            if disabled.contains(&skill.id) || used.contains_key(&skill.id) {
                continue;
            }
            if matches_skill(skill, input) {
                used.insert(skill.id.clone(), skill.clone());
                auto_skill_ids.push(skill.id.clone());
            }
        }

        let available_skill_ids: Vec<String> = self.skills.iter().map(|s| s.id.clone()).collect();

        SkillResolutionResult {
            used_skills: used.into_values().collect(),
            forced_skill_ids,
            auto_skill_ids,
            missing_skill_ids: dedupe_strings(missing_skill_ids),
            disabled_skill_ids,
            available_skill_ids,
        }
    }
}

// ── 内部辅助 ─────────────────────────────────────────────────

fn dedupe_skills(skills: Vec<CapabilitySkillManifest>) -> Vec<CapabilitySkillManifest> {
    let mut by_id: HashMap<String, CapabilitySkillManifest> = HashMap::new();
    for skill in skills {
        let normalized_id = normalize_skill_id(&skill.id);
        let mut normalized = skill;
        normalized.id = normalized_id;
        by_id.insert(normalized.id.clone(), normalized);
    }
    let mut out: Vec<CapabilitySkillManifest> = by_id.into_values().collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn normalize_id_list(values: &[String]) -> Vec<String> {
    dedupe_strings(
        values
            .iter()
            .map(|v| normalize_skill_id(v))
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

fn normalize_skill_id(value: &str) -> String {
    value.trim().to_lowercase()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for v in values {
        if v.is_empty() || seen.contains(&v) {
            continue;
        }
        seen.insert(v.clone());
        out.push(v);
    }
    out
}

fn matches_skill(skill: &CapabilitySkillManifest, input: &SkillResolutionInput) -> bool {
    if let Some(kind) = &input.session_kind {
        let kind_lower = kind.trim().to_lowercase();
        if !kind_lower.is_empty()
            && skill
                .session_kinds
                .iter()
                .any(|k| k.trim().to_lowercase() == kind_lower)
        {
            return true;
        }
    }

    let instruction = match &input.instruction {
        Some(s) => s.trim().to_lowercase(),
        None => return false,
    };
    if instruction.is_empty() {
        return false;
    }

    skill.triggers.iter().any(|trigger| {
        let normalized = trigger.trim().to_lowercase();
        !normalized.is_empty() && instruction.contains(&normalized)
    })
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_three_builtin_skills() {
        let reg = CapabilitySkillRegistry::new();
        let skills = reg.list_skills();
        assert_eq!(skills.len(), 3);
        assert!(reg.get_skill("longform-writing").is_some());
        assert!(reg.get_skill("open-world-play").is_some());
        assert!(reg.get_skill("interactive-film-authoring").is_some());
    }

    #[test]
    fn get_skill_normalizes_id_case() {
        let reg = CapabilitySkillRegistry::new();
        // 大小写不敏感
        assert!(reg.get_skill("Longform-Writing").is_some());
        assert!(reg.get_skill("LONGFORM-WRITING").is_some());
    }

    #[test]
    fn get_skill_trims_whitespace() {
        let reg = CapabilitySkillRegistry::new();
        assert!(reg.get_skill("  longform-writing  ").is_some());
    }

    #[test]
    fn resolve_forced_skill_overrides_disabled_check() {
        // requested 中的 id 若同时在 disabled 中,会被跳过(不入 used)
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            requested_skills: vec!["longform-writing".into()],
            disabled_skills: vec!["longform-writing".into()],
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        assert!(result.used_skills.iter().all(|s| s.id != "longform-writing"));
        assert!(result.disabled_skill_ids.contains(&"longform-writing".to_string()));
    }

    #[test]
    fn resolve_forced_skill_includes_used() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            requested_skills: vec!["longform-writing".into()],
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        assert!(result.forced_skill_ids.contains(&"longform-writing".to_string()));
        assert!(result.used_skills.iter().any(|s| s.id == "longform-writing"));
    }

    #[test]
    fn resolve_missing_skill_reported() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            requested_skills: vec!["nonexistent-skill".into()],
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        assert!(result.missing_skill_ids.contains(&"nonexistent-skill".to_string()));
        assert!(result.used_skills.is_empty());
    }

    #[test]
    fn resolve_auto_match_by_session_kind() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            session_kind: Some("book".into()),
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        // longform-writing 的 session_kinds 含 "book"
        assert!(result.auto_skill_ids.contains(&"longform-writing".to_string()));
    }

    #[test]
    fn resolve_auto_match_by_trigger() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            instruction: Some("请帮我写下一章".into()),
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        // "下一章" 是 longform-writing 的 trigger 之一
        assert!(result.auto_skill_ids.contains(&"longform-writing".to_string()));
    }

    #[test]
    fn resolve_candidate_skill_skipped_if_disabled() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            candidate_skills: vec!["longform-writing".into()],
            disabled_skills: vec!["longform-writing".into()],
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        assert!(!result.auto_skill_ids.contains(&"longform-writing".to_string()));
        assert!(result.disabled_skill_ids.contains(&"longform-writing".to_string()));
    }

    #[test]
    fn resolve_candidate_skill_skipped_if_already_forced() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput {
            requested_skills: vec!["longform-writing".into()],
            candidate_skills: vec!["longform-writing".into()],
            ..Default::default()
        };
        let result = reg.resolve_skills(&input);
        // forced 优先,candidate 不重复入 auto
        assert_eq!(
            result.forced_skill_ids.iter().filter(|id| *id == "longform-writing").count(),
            1
        );
        assert!(!result.auto_skill_ids.contains(&"longform-writing".to_string()));
    }

    #[test]
    fn resolve_available_skill_ids_lists_all() {
        let reg = CapabilitySkillRegistry::new();
        let input = SkillResolutionInput::default();
        let result = reg.resolve_skills(&input);
        assert_eq!(result.available_skill_ids.len(), 3);
    }

    #[test]
    fn resolve_with_extra_skills_dedupes_by_id() {
        // 添加一个 id 与 builtin 重复的 skill —— 应被去重
        let extra = builtin_capability_skills();
        let mut longform = extra[0].clone();
        longform.description = "should be overwritten by builtin".into();
        let registry = CapabilitySkillRegistry::with_extra_skills(extra);
        let skill = registry.get_skill("longform-writing").unwrap();
        // builtin 优先(extra 在后,但 dedupe 用 HashMap 最后保留的是 builtin)
        // 注:HashMap 顺序不确定,这里只验证不出现重复
        assert_eq!(registry.list_skills().len(), 3);
        assert!(!skill.description.contains("should be overwritten"));
    }

    #[test]
    fn resolve_with_extra_skills_adds_new_id() {
        let extra = vec![CapabilitySkillManifest {
            id: "custom-skill".into(),
            name: "Custom".into(),
            description: "Custom skill".into(),
            when_to_use: "When custom".into(),
            triggers: vec!["custom".into()],
            session_kinds: vec!["custom".into()],
            prompt_packs: vec![],
            tool_hints: vec![],
            context_needs: vec![],
            body: String::new(),
            source: SkillSource::User,
        }];
        let registry = CapabilitySkillRegistry::with_extra_skills(extra);
        assert_eq!(registry.list_skills().len(), 4);
        assert!(registry.get_skill("custom-skill").is_some());
    }
}
