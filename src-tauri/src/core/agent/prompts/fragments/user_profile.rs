//! ═══════════════════════════════════════════════════════════════════════════
//! UserProfileFragment - 用户画像注入
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;

use crate::core::agent::user_profile::{UserProfileProvider, UserProfileSnapshot};
use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// 用户画像 fragment。
pub struct UserProfileFragment {
    content: String,
}

impl UserProfileFragment {
    /// 从快照构造（直接格式化）。
    pub fn from_snapshot(snapshot: &UserProfileSnapshot) -> Self {
        Self {
            content: snapshot.format_for_prompt(),
        }
    }

    /// 从 UserProfileProvider 构造（每次调用读盘，反映最新 profile）。
    ///
    /// 通过 trait 解耦，不直接依赖 domain::user。
    pub fn from_provider(provider: &dyn UserProfileProvider) -> Self {
        let snapshot = provider.load_user_profile();
        Self::from_snapshot(&snapshot)
    }

    /// 直接用预渲染内容构造（供测试使用）。
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl ContextualUserFragment for UserProfileFragment {
    fn name(&self) -> &str {
        "user_profile"
    }

    fn render_full(&self) -> Result<String, AppError> {
        Ok(self.content.clone())
    }

    fn bounded_size(&self) -> Option<usize> {
        Some(1024)
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
    fn renders_default_profile() {
        let snapshot = UserProfileSnapshot::default();
        let frag = UserProfileFragment::from_snapshot(&snapshot);

        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("## User Profile"));
        assert!(rendered.contains("User: Writer"));
    }

    #[test]
    fn renders_custom_profile() {
        let mut snapshot = UserProfileSnapshot::default();
        snapshot.name = "Alice".to_string();
        snapshot.genres = vec!["fantasy".to_string(), "scifi".to_string()];

        let frag = UserProfileFragment::from_snapshot(&snapshot);

        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("User: Alice"));
        assert!(rendered.contains("fantasy, scifi"));
    }

    #[test]
    fn fragment_metadata() {
        let frag = UserProfileFragment::from_content("x");
        assert_eq!(frag.name(), "user_profile");
        assert_eq!(frag.bounded_size(), Some(1024));
        assert_eq!(frag.role(), FragmentRole::User);
    }

    #[test]
    fn render_diff_returns_none() {
        let a = UserProfileFragment::from_content("a");
        let b = UserProfileFragment::from_content("b");
        assert!(a.render_diff(&b).is_none());
    }
}
