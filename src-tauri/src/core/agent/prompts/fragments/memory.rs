//! ═══════════════════════════════════════════════════════════════════════════
//! MemoryFragment - Memory context block
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;

use crate::core::agent::memory::manager::MemoryManager;
use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// Memory context block fragment。
pub struct MemoryFragment {
    content: String,
}

impl MemoryFragment {
    /// 从 MemoryManager 聚合并格式化 memory context block。
    ///
    /// 所有 provider 为空时返回空字符串（render_full 会渲染空内容）。
    pub async fn new(memory_manager: &MemoryManager) -> Result<Self, AppError> {
        let content = memory_manager.system_prompt_block().await?;
        Ok(Self { content })
    }

    /// 直接用预渲染的内容构造（供测试使用）。
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl ContextualUserFragment for MemoryFragment {
    fn name(&self) -> &str {
        "memory"
    }

    fn render_full(&self) -> Result<String, AppError> {
        Ok(self.content.clone())
    }

    fn bounded_size(&self) -> Option<usize> {
        None
    }

    fn role(&self) -> FragmentRole {
        FragmentRole::User
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_preloaded_content() {
        let frag = MemoryFragment::from_content("fact A\nfact B");
        assert_eq!(frag.render_full().unwrap(), "fact A\nfact B");
    }

    #[test]
    fn renders_empty_when_no_content() {
        let frag = MemoryFragment::from_content("");
        assert_eq!(frag.render_full().unwrap(), "");
    }

    #[test]
    fn fragment_metadata() {
        let frag = MemoryFragment::from_content("x");
        assert_eq!(frag.name(), "memory");
        assert_eq!(frag.bounded_size(), None);
        assert_eq!(frag.role(), FragmentRole::User);
    }

    #[test]
    fn render_diff_returns_none() {
        let a = MemoryFragment::from_content("a");
        let b = MemoryFragment::from_content("b");
        // memory 不支持 diff（每次全量，由 compaction 控制 size）
        assert!(a.render_diff(&b).is_none());
    }
}
