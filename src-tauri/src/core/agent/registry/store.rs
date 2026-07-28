//! ═══════════════════════════════════════════════════════════════════════════
//! AgentRegistry - Agent 元数据注册表
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 统一 Agent 元数据注册表。
//!
//! 职责：
//! - 加载内置 agent（main + 15 pipeline + 3 subagent + 3 loopskill = 22）
//! - 提供 list_all / list_by_category / get / register 查询接口
//! - 支持 register() 用于未来扩展（如用户自定义 agent）
//!
//! 线程安全：内置 agent 列表只读，register() 通过 RwLock 保护。

use std::sync::RwLock;

use super::builtin::builtin_agents;
use super::types::{AgentCategory, AgentDescriptor};

/// Agent 注册表。
///
/// 内置 agent 在 new() 时加载；自定义 agent 通过 register() 追加。
/// 查询时先查自定义，再查内置（自定义可覆盖内置）。
pub struct AgentRegistry {
    /// 内置 agent（不可变）
    builtin: Vec<AgentDescriptor>,
    /// 自定义 agent（可变，RwLock 保护）
    custom: RwLock<Vec<AgentDescriptor>>,
}

impl AgentRegistry {
    /// 创建注册表并加载内置 agent。
    pub fn new() -> Self {
        Self {
            builtin: builtin_agents(),
            custom: RwLock::new(Vec::new()),
        }
    }

    /// 列出所有 agent（内置 + 自定义）。
    /// 自定义 agent 覆盖同 id 的内置 agent。
    pub fn list_all(&self) -> Vec<AgentDescriptor> {
        let custom = self.custom.read().unwrap_or_else(|e| e.into_inner());
        let custom_ids: std::collections::HashSet<&str> =
            custom.iter().map(|a| a.id.as_str()).collect();
        let mut result: Vec<AgentDescriptor> = self
            .builtin
            .iter()
            .filter(|a| !custom_ids.contains(a.id.as_str()))
            .cloned()
            .collect();
        result.extend(custom.iter().cloned());
        // 按 category 排序，便于前端展示
        result.sort_by(|a, b| {
            (a.category.as_str(), a.id.as_str()).cmp(&(b.category.as_str(), b.id.as_str()))
        });
        result
    }

    /// 按类别列出 agent。
    pub fn list_by_category(&self, category: AgentCategory) -> Vec<AgentDescriptor> {
        self.list_all()
            .into_iter()
            .filter(|a| a.category == category)
            .collect()
    }

    /// 按 id 查询单个 agent。自定义优先于内置。
    pub fn get(&self, id: &str) -> Option<AgentDescriptor> {
        // 先查自定义
        let custom = self.custom.read().unwrap_or_else(|e| e.into_inner());
        if let Some(a) = custom.iter().find(|a| a.id == id) {
            return Some(a.clone());
        }
        drop(custom);
        // 再查内置
        self.builtin.iter().find(|a| a.id == id).cloned()
    }

    /// 注册自定义 agent。
    ///
    /// 若 id 与已有 agent（内置或自定义）冲突，返回 Err。
    /// 不允许覆盖内置 agent（避免破坏一致性，对齐 AGENTS.md "禁止兼容层" 原则）。
    pub fn register(&self, descriptor: AgentDescriptor) -> Result<(), String> {
        // 检查内置冲突
        if self.builtin.iter().any(|a| a.id == descriptor.id) {
            return Err(format!(
                "Cannot register agent '{}': id conflicts with builtin agent",
                descriptor.id
            ));
        }
        let mut custom = self.custom.write().unwrap_or_else(|e| e.into_inner());
        // 检查自定义冲突
        if custom.iter().any(|a| a.id == descriptor.id) {
            return Err(format!(
                "Cannot register agent '{}': id already registered",
                descriptor.id
            ));
        }
        custom.push(descriptor);
        Ok(())
    }

    /// 列出所有类别（去重）。
    pub fn list_categories(&self) -> Vec<AgentCategory> {
        vec![
            AgentCategory::Main,
            AgentCategory::Pipeline,
            AgentCategory::SubAgent,
            AgentCategory::LoopSkill,
        ]
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_loads_22_builtin_agents() {
        let registry = AgentRegistry::new();
        let all = registry.list_all();
        assert_eq!(all.len(), 22);
    }

    #[test]
    fn get_returns_builtin() {
        let registry = AgentRegistry::new();
        let main = registry.get("main").expect("main should exist");
        assert_eq!(main.category, AgentCategory::Main);
        assert!(main.can_spawn_subagent);

        let planner = registry.get("planner").expect("planner should exist");
        assert_eq!(planner.category, AgentCategory::Pipeline);
    }

    #[test]
    fn get_returns_none_for_unknown() {
        let registry = AgentRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn list_by_category_filters_correctly() {
        let registry = AgentRegistry::new();
        let pipelines = registry.list_by_category(AgentCategory::Pipeline);
        assert_eq!(pipelines.len(), 15);
        assert!(pipelines.iter().all(|a| a.category == AgentCategory::Pipeline));

        let subs = registry.list_by_category(AgentCategory::SubAgent);
        assert_eq!(subs.len(), 3);

        let skills = registry.list_by_category(AgentCategory::LoopSkill);
        assert_eq!(skills.len(), 3);

        let mains = registry.list_by_category(AgentCategory::Main);
        assert_eq!(mains.len(), 1);
    }

    #[test]
    fn register_adds_custom_agent() {
        let registry = AgentRegistry::new();
        let desc = AgentDescriptor {
            id: "custom-1".to_string(),
            name: "Custom".to_string(),
            category: AgentCategory::SubAgent,
            role: "custom-1".to_string(),
            description: "test custom agent".to_string(),
            system_prompt: None,
            tool_whitelist: None,
            model_override: None,
            can_spawn_subagent: false,
        };
        registry.register(desc).unwrap();

        let got = registry.get("custom-1").expect("custom agent should exist");
        assert_eq!(got.name, "Custom");
        assert_eq!(registry.list_all().len(), 23);
    }

    #[test]
    fn register_rejects_builtin_conflict() {
        let registry = AgentRegistry::new();
        let desc = AgentDescriptor {
            id: "main".to_string(),
            name: "Fake Main".to_string(),
            category: AgentCategory::Main,
            role: "main".to_string(),
            description: "attempt to override builtin".to_string(),
            system_prompt: None,
            tool_whitelist: None,
            model_override: None,
            can_spawn_subagent: false,
        };
        let err = registry.register(desc).unwrap_err();
        assert!(err.contains("conflicts with builtin"));
    }

    #[test]
    fn register_rejects_duplicate_custom() {
        let registry = AgentRegistry::new();
        let desc = AgentDescriptor {
            id: "custom-2".to_string(),
            name: "Custom 2".to_string(),
            category: AgentCategory::SubAgent,
            role: "custom-2".to_string(),
            description: "test".to_string(),
            system_prompt: None,
            tool_whitelist: None,
            model_override: None,
            can_spawn_subagent: false,
        };
        registry.register(desc.clone()).unwrap();
        let err = registry.register(desc).unwrap_err();
        assert!(err.contains("already registered"));
    }

    #[test]
    fn list_categories_returns_four() {
        let registry = AgentRegistry::new();
        let cats = registry.list_categories();
        assert_eq!(cats.len(), 4);
    }
}
