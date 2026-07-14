// 内置 Agent 定义 —— 22 个 agent 的元数据。
//
// 分布:
// - Main:       1 个（main）
// - Pipeline:   15 个（architect/planner/writer/continuity/reviser/polisher/
//               length_normalizer/foundation_reviewer/state_validator/consolidator/
//               chapter_analyzer/composer/short_fiction/fanfic_canon_importer/script_storyboard）
// - SubAgent:   3 个（researcher/outliner/critic）
// - LoopSkill:  3 个（loop_triage/loop_verifier/minimal_fix）
//
// 数据来源:
// - role 与 prompts::ALL_ROLES 对齐（main + 15 pipeline）
// - SubAgentRole 与 core/agent/subagent/types.rs 对齐
// - LoopSkill 与 core/agent/loop_engine/prompts.rs::prompt_for 对齐
// - description 引用各模块的职责说明
//
// 注意:
// - 本注册表不接管 agent 执行逻辑，仅提供元数据
// - system_prompt 字段为 None（运行时由 identity::build_system_prompt 组装）
//   例外：SubAgent 的 system_prompt 引用 SubAgentRole::system_prompt()，便于前端预览

use super::types::{AgentCategory, AgentDescriptor};

/// 返回所有内置 agent 描述符。
///
/// 顺序：main → pipeline（按 ALL_ROLES 顺序）→ subagent → loopskill。
pub fn builtin_agents() -> Vec<AgentDescriptor> {
    let mut agents = Vec::with_capacity(22);
    agents.push(main_agent());
    agents.extend(pipeline_agents());
    agents.extend(sub_agents());
    agents.extend(loop_skill_agents());
    agents
}

fn main_agent() -> AgentDescriptor {
    AgentDescriptor {
        id: "main".to_string(),
        name: "Main Agent".to_string(),
        category: AgentCategory::Main,
        role: "main".to_string(),
        description: "主对话 agent，与用户直接交互，可委派任务给 sub-agent".to_string(),
        system_prompt: None,
        tool_whitelist: None,
        model_override: None,
        can_spawn_subagent: true,
    }
}

fn pipeline_agents() -> Vec<AgentDescriptor> {
    // 15 个 pipeline agent，role 与 prompts::ALL_ROLES 对齐（跳过 "main"）
    // description 引用 prompts::pipeline_role_meta 中的职责说明（简短化）
    let entries: &[(&str, &str, &str)] = &[
        (
            "architect",
            "Architect",
            "构建小说整体结构，产出卷/章结构与初始设定",
        ),
        (
            "planner",
            "Planner",
            "为每一章生成章节备忘录（chapter memo）",
        ),
        (
            "writer",
            "Writer",
            "根据 chapter memo 生成章节正文",
        ),
        (
            "continuity",
            "Continuity Auditor",
            "从多维度审计章节连续性，检测时间线/人物/设定破绽",
        ),
        (
            "reviser",
            "Reviser",
            "根据 Auditor 的 issue 列表返工章节",
        ),
        (
            "polisher",
            "Polisher",
            "对正文进行最终润色，优化遣词造句",
        ),
        (
            "length_normalizer",
            "Length Normalizer",
            "将章节字数调整到目标区间（expand/shrink）",
        ),
        (
            "foundation_reviewer",
            "Foundation Reviewer",
            "在开篇 N 章后审查地基质量（5 维度）",
        ),
        (
            "state_validator",
            "State Validator",
            "校验章节状态一致性（人物/时间线/物品）",
        ),
        (
            "consolidator",
            "Consolidator",
            "将多章内容压缩为卷摘要",
        ),
        (
            "chapter_analyzer",
            "Chapter Analyzer",
            "从章节中提取结构化标签（11 个标签块）",
        ),
        (
            "composer",
            "Composer",
            "上下文装配（select/compile 两种模式）",
        ),
        (
            "short_fiction",
            "Short Fiction",
            "短篇创作流程（outline/review/writer/package）",
        ),
        (
            "fanfic_canon_importer",
            "Fanfic Canon Importer",
            "导入原作设定（原作向/AU 平行世界）",
        ),
        (
            "script_storyboard",
            "Script & Storyboard",
            "剧本与分镜创作（script/storyboard/interactive_film）",
        ),
    ];

    entries
        .iter()
        .map(|(id, name, desc)| AgentDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            category: AgentCategory::Pipeline,
            role: id.to_string(),
            description: desc.to_string(),
            system_prompt: None,
            tool_whitelist: None,
            model_override: None,
            can_spawn_subagent: false,
        })
        .collect()
}

fn sub_agents() -> Vec<AgentDescriptor> {
    // 3 个 sub-agent，role 与 SubAgentRole::as_str 对齐
    // system_prompt 引用 SubAgentRole::system_prompt，便于前端预览
    use crate::core::agent::subagent::SubAgentRole;
    let roles = [
        SubAgentRole::Researcher,
        SubAgentRole::Outliner,
        SubAgentRole::Critic,
    ];
    roles
        .iter()
        .map(|r| AgentDescriptor {
            id: r.as_str().to_string(),
            name: display_name_for_subagent(r),
            category: AgentCategory::SubAgent,
            role: r.as_str().to_string(),
            description: r.description().to_string(),
            system_prompt: Some(r.system_prompt().to_string()),
            tool_whitelist: None,
            model_override: None,
            can_spawn_subagent: false,
        })
        .collect()
}

fn display_name_for_subagent(role: &crate::core::agent::subagent::SubAgentRole) -> String {
    match role {
        crate::core::agent::subagent::SubAgentRole::Researcher => "Researcher".to_string(),
        crate::core::agent::subagent::SubAgentRole::Outliner => "Outliner".to_string(),
        crate::core::agent::subagent::SubAgentRole::Critic => "Critic".to_string(),
    }
}

fn loop_skill_agents() -> Vec<AgentDescriptor> {
    // 3 个 loop skill agent，与 loop_engine::prompts::prompt_for 对齐
    // 使用 underscore 格式作为 id（与 loop_engine 内部一致）
    let entries: &[(&str, &str, &str)] = &[
        (
            "loop_triage",
            "Loop Triage",
            "对 loop-engineering 发现项进行优先级分类（high/watch/noise/state）",
        ),
        (
            "loop_verifier",
            "Loop Verifier",
            "验证 minimal-fix 的修复结果是否真正解决问题",
        ),
        (
            "minimal_fix",
            "Minimal Fix",
            "以最小 diff 修复单一问题，遵守 denylist 与 worktree 隔离",
        ),
    ];

    entries
        .iter()
        .map(|(id, name, desc)| AgentDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            category: AgentCategory::LoopSkill,
            // loop skill agent 没有独立的身份文件目录，role 复用 id
            role: id.to_string(),
            description: desc.to_string(),
            system_prompt: None,
            tool_whitelist: None,
            model_override: None,
            can_spawn_subagent: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_agents_has_22_entries() {
        let agents = builtin_agents();
        assert_eq!(agents.len(), 22, "expected 22 builtin agents (1+15+3+3)");
    }

    #[test]
    fn builtin_agents_has_one_main() {
        let agents = builtin_agents();
        let mains: Vec<_> = agents.iter().filter(|a| a.category == AgentCategory::Main).collect();
        assert_eq!(mains.len(), 1);
        assert_eq!(mains[0].id, "main");
        assert!(mains[0].can_spawn_subagent);
    }

    #[test]
    fn builtin_agents_has_15_pipeline() {
        let agents = builtin_agents();
        let pipelines: Vec<_> = agents
            .iter()
            .filter(|a| a.category == AgentCategory::Pipeline)
            .collect();
        assert_eq!(pipelines.len(), 15);
        // 所有 pipeline agent 不能 spawn subagent
        assert!(pipelines.iter().all(|a| !a.can_spawn_subagent));
    }

    #[test]
    fn builtin_agents_has_3_subagent() {
        let agents = builtin_agents();
        let subs: Vec<_> = agents
            .iter()
            .filter(|a| a.category == AgentCategory::SubAgent)
            .collect();
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0].id, "researcher");
        assert_eq!(subs[1].id, "outliner");
        assert_eq!(subs[2].id, "critic");
        // sub-agent 应携带 system_prompt
        assert!(subs.iter().all(|a| a.system_prompt.is_some()));
    }

    #[test]
    fn builtin_agents_has_3_loopskill() {
        let agents = builtin_agents();
        let skills: Vec<_> = agents
            .iter()
            .filter(|a| a.category == AgentCategory::LoopSkill)
            .collect();
        assert_eq!(skills.len(), 3);
        let ids: Vec<_> = skills.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"loop_triage"));
        assert!(ids.contains(&"loop_verifier"));
        assert!(ids.contains(&"minimal_fix"));
    }

    #[test]
    fn builtin_agent_ids_are_unique() {
        let agents = builtin_agents();
        let mut ids: Vec<_> = agents.iter().map(|a| a.id.as_str()).collect();
        ids.sort();
        let len_before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len_before, "duplicate agent ids found");
    }

    #[test]
    fn pipeline_roles_align_with_all_roles() {
        // pipeline agent 的 role 应是 prompts::ALL_ROLES 中除 main 外的全部
        use crate::core::agent::prompts::ALL_ROLES;
        let agents = builtin_agents();
        let pipeline_roles: Vec<_> = agents
            .iter()
            .filter(|a| a.category == AgentCategory::Pipeline)
            .map(|a| a.role.as_str())
            .collect();
        let expected: Vec<_> = ALL_ROLES.iter().filter(|r| **r != "main").copied().collect();
        assert_eq!(pipeline_roles, expected);
    }
}
