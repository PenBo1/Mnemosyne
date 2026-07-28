//! ═══════════════════════════════════════════════════════════════════════════
//! Prompt Cache - 提示词缓存系统
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 核心设计原则:
//! 1. Per-conversation prompt caching is sacred
//!    - 长对话复用缓存前缀, 每次复用节省成本
//! 2. The core is a narrow waist
//!    - 工具集在对话中保持稳定, 不动态改变
//! 3. Byte-stable system prompt
//!    - system prompt 在对话生命周期内字节稳定

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::core::agent::effort::EffortLevel;

// ── 缓存键 ──────────────────────────────────────────────────────────────────

/// 提示词缓存键
/// 
/// 用于验证 prompt 是否变化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCacheKey {
    /// 角色哈希
    pub role_hash: u64,
    /// 身份文件哈希
    pub identity_hash: u64,
    /// 工具集哈希
    pub toolset_hash: u64,
    /// Effort 级别
    pub effort: EffortLevel,
    /// 整体哈希
    pub total_hash: u64,
}

impl PromptCacheKey {
    /// 创建缓存键
    /// 
    /// # 参数
    /// - `role`: 角色名称
    /// - `soul`: 灵魂文件内容
    /// - `context`: 上下文文件内容
    /// - `memory`: 记忆文件内容
    /// - `toolsets`: 工具集列表
    /// - `effort`: Effort 级别
    pub fn new(role: &str, soul: &str, context: &str, memory: &str, toolsets: &[&str], effort: EffortLevel) -> Self {
        let role_hash = hash_string(role);
        let identity_hash = hash_strings(&[soul, context, memory]);
        let toolset_hash = hash_strings(toolsets);
        
        let mut total_hasher = DefaultHasher::new();
        role_hash.hash(&mut total_hasher);
        identity_hash.hash(&mut total_hasher);
        toolset_hash.hash(&mut total_hasher);
        format!("{:?}", effort).hash(&mut total_hasher);
        let total_hash = total_hasher.finish();

        Self {
            role_hash,
            identity_hash,
            toolset_hash,
            effort,
            total_hash,
        }
    }

    /// 判断两个缓存键是否相同
    pub fn is_same(&self, other: &Self) -> bool {
        self.total_hash == other.total_hash
    }
}

/// 计算字符串哈希值
fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// 计算字符串数组哈希值
fn hash_strings(strings: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for s in strings {
        s.hash(&mut hasher);
    }
    hasher.finish()
}

// ── 系统提示构建器 ────────────────────────────────────────────────────────

/// 系统提示构建器
/// 
/// 保证:
/// 1. 相同输入产生相同输出 (字节稳定)
/// 2. 对话中不改变工具集
/// 3. 对话中不重建系统提示
pub struct SystemPromptBuilder {
    role: String,
    soul: String,
    context: String,
    memory: String,
    toolsets: Vec<String>,
    effort: EffortLevel,
    frozen: bool,
}

impl SystemPromptBuilder {
    /// 创建构建器实例
    /// 
    /// # 参数
    /// - `role`: 角色名称
    pub fn new(role: &str) -> Self {
        Self {
            role: role.to_string(),
            soul: String::new(),
            context: String::new(),
            memory: String::new(),
            toolsets: Vec::new(),
            effort: EffortLevel::Medium,
            frozen: false,
        }
    }

    /// 设置身份内容
    pub fn with_identity(mut self, soul: &str, context: &str, memory: &str) -> Self {
        if self.frozen {
            tracing::warn!("SystemPromptBuilder is frozen, cannot modify identity");
            return self;
        }
        self.soul = soul.to_string();
        self.context = context.to_string();
        self.memory = memory.to_string();
        self
    }

    /// 设置工具集
    pub fn with_toolsets(mut self, toolsets: Vec<String>) -> Self {
        if self.frozen {
            tracing::warn!("SystemPromptBuilder is frozen, cannot modify toolsets");
            return self;
        }
        self.toolsets = toolsets;
        self
    }

    /// 设置 Effort 级别
    pub fn with_effort(mut self, effort: EffortLevel) -> Self {
        if self.frozen {
            tracing::warn!("SystemPromptBuilder is frozen, cannot modify effort");
            return self;
        }
        self.effort = effort;
        self
    }

    /// 冻结构建器 (防止后续修改)
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// 获取缓存键
    pub fn cache_key(&self) -> PromptCacheKey {
        PromptCacheKey::new(
            &self.role,
            &self.soul,
            &self.context,
            &self.memory,
            self.toolsets.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice(),
            self.effort,
        )
    }

    /// 构建系统提示
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        parts.push(self.build_identity_section());
        
        if !self.toolsets.is_empty() {
            parts.push(self.build_toolset_section());
        }

        parts.push(self.build_effort_section());

        parts.join("\n\n")
    }

    /// 构建身份部分
    fn build_identity_section(&self) -> String {
        format!(
            "# Identity\n\n{}\n\n{}\n\n{}",
            self.soul,
            self.context,
            self.memory,
        )
    }

    /// 构建工具集部分
    fn build_toolset_section(&self) -> String {
        format!(
            "# Available Tools\n\nYou have access to the following toolsets:\n{}",
            self.toolsets.iter()
                .map(|t| format!("- {}", t))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// 构建 Effort 部分
    fn build_effort_section(&self) -> String {
        let (max_iterations, max_files, max_tokens) = match self.effort {
            EffortLevel::Low => (5, 10, 2000),
            EffortLevel::Medium => (20, 50, 8000),
            EffortLevel::High => (50, 200, 16000),
            EffortLevel::Ultra => (100, 1000, 32000),
        };

        format!(
            "# Effort Level: {:?}\n\nMax iterations: {}\nMax files: {}\nMax tokens per call: {}",
            self.effort,
            max_iterations,
            max_files,
            max_tokens
        )
    }

    /// 检查是否已冻结
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
}

// ── 对话级缓存 ──────────────────────────────────────────────────────────────

/// 对话级提示词缓存
pub struct ConversationPromptCache {
    builder: Option<SystemPromptBuilder>,
    cache_key: Option<PromptCacheKey>,
    system_prompt: Option<String>,
}

impl Default for ConversationPromptCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationPromptCache {
    /// 创建缓存实例
    pub fn new() -> Self {
        Self {
            builder: None,
            cache_key: None,
            system_prompt: None,
        }
    }

    /// 初始化缓存
    pub fn initialize(&mut self, builder: SystemPromptBuilder) {
        if self.builder.is_some() {
            tracing::warn!("ConversationPromptCache already initialized, ignoring re-initialization");
            return;
        }

        self.cache_key = Some(builder.cache_key());
        self.system_prompt = Some(builder.build());
        let mut builder = builder;
        builder.freeze();
        self.builder = Some(builder);
    }

    /// 获取系统提示
    pub fn get_system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// 获取缓存键
    pub fn get_cache_key(&self) -> Option<&PromptCacheKey> {
        self.cache_key.as_ref()
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.builder.is_some()
    }

    /// 验证不变量
    pub fn validate_invariants(&self) -> Result<(), String> {
        if let Some(builder) = &self.builder {
            if !builder.is_frozen() {
                return Err("SystemPromptBuilder is not frozen".to_string());
            }
        }
        Ok(())
    }
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_stability() {
        let key1 = PromptCacheKey::new("main", "soul", "context", "memory", &["file", "terminal"], EffortLevel::Medium);
        let key2 = PromptCacheKey::new("main", "soul", "context", "memory", &["file", "terminal"], EffortLevel::Medium);
        
        assert!(key1.is_same(&key2));
    }

    #[test]
    fn test_cache_key_change_detection() {
        let key1 = PromptCacheKey::new("main", "soul", "context", "memory", &["file", "terminal"], EffortLevel::Medium);
        let key2 = PromptCacheKey::new("main", "soul", "context", "memory", &["file", "terminal", "browser"], EffortLevel::Medium);
        
        assert!(!key1.is_same(&key2));
    }

    #[test]
    fn test_prompt_builder_freeze() {
        let mut builder = SystemPromptBuilder::new("main");
        builder.freeze();
        
        assert!(builder.is_frozen());
    }
}