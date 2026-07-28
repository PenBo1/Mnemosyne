//! ═══════════════════════════════════════════════════════════════════════════
//! SoulFragment - SOUL.md 加载
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;

use crate::core::agent::identity::{self, IdentityKind};
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// SOUL.md 内容 fragment。
///
/// 构造时从 `<data_dir>/agents/<role>/SOUL.md` 读取（缺失时回退到默认模板）。
pub struct SoulFragment {
    role: String,
    content: String,
}

impl SoulFragment {
    /// 从磁盘加载 SOUL.md。
    ///
    /// 缺失或空文件时回退到 `IdentityKind::Soul.default_content_for(role)`，
    /// 与 `identity::load_identity` 行为一致。
    pub async fn new(data_dir: &DataDir, role: &str) -> Self {
        let content = identity::load_identity(data_dir, role, IdentityKind::Soul)
            .await
            .unwrap_or_default();
        Self {
            role: role.to_string(),
            content,
        }
    }

    /// 角色名（用于调试）。
    pub fn role(&self) -> &str {
        &self.role
    }
}

impl ContextualUserFragment for SoulFragment {
    fn name(&self) -> &str {
        "soul"
    }

    fn render_full(&self) -> Result<String, AppError> {
        Ok(self.content.clone())
    }

    fn bounded_size(&self) -> Option<usize> {
        Some(2048)
    }

    fn role(&self) -> FragmentRole {
        FragmentRole::System
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn loads_default_soul_when_file_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let frag = SoulFragment::new(&data_dir, "main").await;

        assert_eq!(frag.role(), "main");
        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("Mnemosyne"), "应回退到默认 SOUL.md");
    }

    #[tokio::test]
    async fn loads_custom_soul_when_file_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("architect");
        std::fs::create_dir_all(&role_dir).unwrap();
        std::fs::write(role_dir.join("SOUL.md"), "# Custom Architect Soul").unwrap();

        let frag = SoulFragment::new(&data_dir, "architect").await;

        assert_eq!(frag.render_full().unwrap(), "# Custom Architect Soul");
    }

    #[test]
    fn fragment_metadata() {
        let frag = SoulFragment {
            role: "main".to_string(),
            content: "x".to_string(),
        };
        assert_eq!(frag.name(), "soul");
        assert_eq!(frag.bounded_size(), Some(2048));
        assert_eq!(frag.role(), FragmentRole::System);
    }

    #[test]
    fn render_diff_returns_none_by_default() {
        let a = SoulFragment {
            role: "main".to_string(),
            content: "x".to_string(),
        };
        let b = SoulFragment {
            role: "main".to_string(),
            content: "y".to_string(),
        };
        // SoulFragment 不支持 diff（每次全量渲染，靠 SOUL.md byte-stable 保持 cache）
        assert!(a.render_diff(&b).is_none());
    }
}
