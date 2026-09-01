//! ═══════════════════════════════════════════════════════════════════════════
//! SkillsFragment - Skills prompt 两层缓存
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;
use std::path::Path;
use std::time::SystemTime;

use crate::infrastructure::cache::TwoTierCache;
use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// Skills prompt fragment。
///
/// 通过 `TwoTierCache<String>` 加载（缓存键为 skills 源文件路径或聚合键）。
pub struct SkillsFragment {
    content: String,
}

impl SkillsFragment {
    /// 直接用预渲染内容构造（供测试或已加载场景使用）。
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// 通过两层缓存加载 skills prompt。
    ///
    /// `key` 为缓存键（如 "skills_prompt" 或源文件路径）。
    /// `source_path` 为源文件路径（用于写入 snapshot manifest，供后续校验 mtime/size）。
    /// `loader` 为缓存未命中时的加载函数，返回 `(content, mtime, size)`。
    ///
    /// 复用 `TwoTierCache::get_or_load` 的 LRU + disk snapshot + manifest 验证逻辑。
    pub async fn from_cache<F, Fut>(
        cache: &TwoTierCache<String>,
        key: &str,
        source_path: &Path,
        loader: F,
    ) -> Result<Self, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(String, SystemTime, u64), AppError>>,
    {
        let content = cache.get_or_load(key, source_path, loader).await?;
        Ok(Self::from_content(content))
    }
}

impl ContextualUserFragment for SkillsFragment {
    fn name(&self) -> &str {
        "skills"
    }

    fn render_full(&self) -> Result<String, AppError> {
        Ok(self.content.clone())
    }

    fn bounded_size(&self) -> Option<usize> {
        Some(4096)
    }

    fn role(&self) -> FragmentRole {
        FragmentRole::System
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ── SkillInstructions：单个 skill 加载后的 instructions 注入 ──────────────
//
// 对照 codex `ext/skills/src/fragments.rs::SkillInstructions`：
// 当 agent 显式或隐式调用某个 skill 时，将其完整 instructions 内容用 `<skill>`
// 标记包裹注入到 user 消息，role=user。
//
// 格式（对照 codex）：
//   <skill>
//   <name>{name}</name>
//   <path>{path}</path>
//   {contents}
//   </skill>

/// 单个 skill 的 instructions fragment（对照 codex `SkillInstructions`）。
///
/// 与 `SkillsFragment`（可用 skills 列表，role=system）不同：
/// - `SkillInstructions` 是被调用 skill 的完整内容，role=user
/// - 用 `<skill>` / `</skill>` 标记包裹，便于模型识别边界
pub struct SkillInstructions {
    name: String,
    path: String,
    contents: String,
}

impl SkillInstructions {
    pub fn new(name: impl Into<String>, path: impl Into<String>, contents: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            contents: contents.into(),
        }
    }
}

impl ContextualUserFragment for SkillInstructions {
    fn name(&self) -> &str {
        "skill_instructions"
    }

    fn render_full(&self) -> Result<String, AppError> {
        // 对照 codex 格式：<skill>\n<name>...</name>\n<path>...</path>\n{contents}\n</skill>
        Ok(format!(
            "<skill>\n<name>{}</name>\n<path>{}</path>\n{}\n</skill>",
            self.name, self.path, self.contents
        ))
    }

    fn bounded_size(&self) -> Option<usize> {
        // 单个 skill instructions 不应超过 8K tokens（codex 默认 skill metadata budget）
        Some(8192)
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
        let frag = SkillsFragment::from_content("# Available skills\n- skill_a\n- skill_b");
        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("Available skills"));
        assert!(rendered.contains("skill_a"));
    }

    #[test]
    fn renders_empty_when_no_skills() {
        let frag = SkillsFragment::from_content("");
        assert_eq!(frag.render_full().unwrap(), "");
    }

    #[test]
    fn fragment_metadata() {
        let frag = SkillsFragment::from_content("x");
        assert_eq!(frag.name(), "skills");
        assert_eq!(frag.bounded_size(), Some(4096));
        assert_eq!(frag.role(), FragmentRole::System);
    }

    #[test]
    fn render_diff_returns_none() {
        let a = SkillsFragment::from_content("a");
        let b = SkillsFragment::from_content("b");
        // skills 不支持 diff（缓存内容由源文件 mtime 失效，非 turn 间 diff）
        assert!(a.render_diff(&b).is_none());
    }

    #[tokio::test]
    async fn from_cache_uses_two_tier_cache() {
        // 验证 from_cache 正确包装 TwoTierCache
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("skills.md");
        std::fs::write(&src, "# Skills\n- test_skill").unwrap();

        let cache: TwoTierCache<String> = TwoTierCache::new(tmp.path().to_path_buf(), 4);

        let meta = std::fs::metadata(&src).unwrap();
        let mtime = meta.modified().unwrap();
        let size = meta.len();

        let frag = SkillsFragment::from_cache(&cache, "skills_prompt", &src, || {
            let content = "# Skills\n- test_skill".to_string();
            async move { Ok((content, mtime, size)) }
        })
        .await
        .unwrap();

        assert_eq!(frag.render_full().unwrap(), "# Skills\n- test_skill");

        // 第二次加载应命中 LRU（loader 不再被调用）
        let frag2 = SkillsFragment::from_cache(&cache, "skills_prompt", &src, || {
            // 若被调用说明缓存未命中
            panic!("loader should not be called on LRU hit")
        })
        .await
        .unwrap();

        assert_eq!(frag2.render_full().unwrap(), "# Skills\n- test_skill");
    }

    // ── SkillInstructions fragment 测试 ──

    #[test]
    fn skill_instructions_wraps_with_skill_markers() {
        let frag = super::SkillInstructions::new(
            "novel_writing",
            "/path/to/SKILL.md",
            "Write the next chapter of the novel.",
        );
        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("<skill>"));
        assert!(rendered.contains("</skill>"));
        assert!(rendered.contains("<name>novel_writing</name>"));
        assert!(rendered.contains("<path>/path/to/SKILL.md</path>"));
        assert!(rendered.contains("Write the next chapter"));
    }

    #[test]
    fn skill_instructions_role_is_user() {
        let frag = super::SkillInstructions::new("a", "b", "c");
        assert_eq!(frag.role(), FragmentRole::User);
    }

    #[test]
    fn skill_instructions_name_field() {
        let frag = super::SkillInstructions::new("my_skill", "p", "c");
        assert_eq!(frag.name(), "skill_instructions");
    }
}
