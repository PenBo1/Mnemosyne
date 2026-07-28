//! ═══════════════════════════════════════════════════════════════════════════
//! Tiered - 三层系统提示词架构
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::infrastructure::fs::data_dir::DataDir;

use super::super::identity::{self, IdentityKind};
use super::super::user_profile::UserProfileSnapshot;

// ── 三层结构定义 ───────────────────────────────────────────────────────────

/// 稳定层：byte-stable，会话内不变，保持 LLM prompt cache 温暖。
pub struct StableTier {
    /// SOUL.md 身份定义
    pub soul: String,
    /// skills 提示词
    pub skills: String,
    /// 环境提示（含 CONTEXT.md 的环境/任务上下文）
    pub environment_hints: String,
}

/// 上下文层：caller-supplied context + context files。
pub struct ContextTier {
    /// caller 提供的上下文（custom_instructions）
    pub caller_context: String,
    /// AGENTS.md 等上下文文件（Task 8 补全）
    pub context_files: String,
}

/// 易变层：每轮可能变化。
pub struct VolatileTier {
    /// memory snapshot（来自 MEMORY.md）
    pub memory_snapshot: String,
    /// 用户画像（format_for_prompt 输出）
    pub user_profile: String,
    /// 时间戳，仅精确到日（避免破坏 byte-stable）
    pub timestamp: String,
}

/// 三层系统提示词组合。
pub struct SystemPromptParts {
    pub stable: StableTier,
    pub context: ContextTier,
    pub volatile: VolatileTier,
}

/// AgentEngine 上的缓存类型：渲染后的字符串 + 构建时的 parts。
///
/// 存储_parts 用于调试与未来按层比较；缓存命中时仅取 String。
pub type CachedSystemPrompt = Option<(String, SystemPromptParts)>;

// ── 构建逻辑 ──────────────────────────────────────────────────────────────

/// 构建 skills 提示词。
///
/// 当前为占位实现（skills 系统尚未集成），返回空字符串。
/// 后续 skills 加载逻辑完成后在此补全。
fn build_skills_prompt() -> String {
    String::new()
}

/// 将 CONTEXT.md 内容拆分为 core 与 extended 两部分。
///
/// 分割点为 `<pipeline_overview>` 标签：之前的通用 section（environment /
/// security_kernel / ipc_conventions / i18n / git 约定等）为 core，从
/// `<pipeline_overview>` 到末尾的小说创作专用 section 为 extended。
///
/// - simple chat（无 book/chapter 上下文）只需 core，节省 token
/// - book/章节任务加载 core + extended
///
/// 未找到 `<pipeline_overview>` 时视全文为 core，extended 为空（向后兼容）。
pub fn split_context_md(content: &str) -> (String, String) {
    match content.find("<pipeline_overview>") {
        Some(idx) => {
            let core = content[..idx].trim_end().to_string();
            let extended = content[idx..].to_string();
            (core, extended)
        }
        None => (content.to_string(), String::new()),
    }
}

/// 构建三层系统提示词各层内容。
///
/// - StableTier：SOUL.md + skills + environment hints（CONTEXT.md 的 core[+extended]）
/// - ContextTier：caller context + context files（Task 8 补全 context_files）
/// - VolatileTier：MEMORY.md snapshot + user profile + timestamp（%Y-%m-%d）
///
/// `load_extended_context` 控制 CONTEXT.md 的 extended 部分（pipeline_overview 等）
/// 是否加载。simple chat 传 false 仅加载 core，book/章节任务传 true 加载全文。
///
/// 时间戳仅精确到日，避免破坏 StableTier 的 byte-stable 契约
/// （同一天内多次调用产生相同 timestamp，保证前缀稳定）。
pub async fn build_system_prompt_parts(
    data_dir: &DataDir,
    role: &str,
    custom_instructions: Option<&str>,
    user_profile: Option<&UserProfileSnapshot>,
    load_extended_context: bool,
) -> SystemPromptParts {
    // ── StableTier ──
    let soul = identity::load_identity(data_dir, role, IdentityKind::Soul)
        .await
        .unwrap_or_default();
    let skills = build_skills_prompt();
    // CONTEXT.md 拆分 core/extended：simple chat 仅加载 core
    let raw_context = identity::load_identity(data_dir, role, IdentityKind::Context)
        .await
        .unwrap_or_default();
    let (core, extended) = split_context_md(&raw_context);
    let environment_hints = if load_extended_context {
        if extended.is_empty() {
            core
        } else {
            format!("{}\n\n{}", core, extended)
        }
    } else {
        core
    };

    // ── ContextTier ──
    let caller_context = custom_instructions
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_default();
    // Task 8 补全：加载 AGENTS.md 等上下文文件
    let context_files = String::new();

    // ── VolatileTier ──
    let memory_snapshot = identity::load_identity(data_dir, role, IdentityKind::Memory)
        .await
        .unwrap_or_default();
    let user_profile_str = user_profile
        .map(|p| p.format_for_prompt())
        .unwrap_or_default();
    let timestamp = format!("Current date: {}", chrono::Utc::now().format("%Y-%m-%d"));

    SystemPromptParts {
        stable: StableTier {
            soul,
            skills,
            environment_hints,
        },
        context: ContextTier {
            caller_context,
            context_files,
        },
        volatile: VolatileTier {
            memory_snapshot,
            user_profile: user_profile_str,
            timestamp,
        },
    }
}

/// 渲染三层提示词为最终 system prompt 字符串。
///
/// 层内非空 section 用 `\n\n` 连接；三层之间用 `\n\n---\n\n` 连接。
/// 空层（所有 section 为空）会被跳过，避免产生连续分隔符。
pub fn render_system_prompt(parts: &SystemPromptParts) -> String {
    let stable_sections: Vec<String> = vec![
        parts.stable.soul.clone(),
        parts.stable.skills.clone(),
        parts.stable.environment_hints.clone(),
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .collect();
    let stable = stable_sections.join("\n\n");

    let context_sections: Vec<String> = vec![
        parts.context.caller_context.clone(),
        parts.context.context_files.clone(),
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .collect();
    let context = context_sections.join("\n\n");

    let volatile_sections: Vec<String> = vec![
        parts.volatile.memory_snapshot.clone(),
        parts.volatile.user_profile.clone(),
        parts.volatile.timestamp.clone(),
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .collect();
    let volatile = volatile_sections.join("\n\n");

    [stable, context, volatile]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

// ── 缓存逻辑 ──────────────────────────────────────────────────────────────

/// 从缓存获取或构建 system prompt。
///
/// 缓存命中（Some）时直接返回缓存的字符串，避免磁盘 I/O 与重建。
/// 缓存未命中（None）时构建 parts → 渲染 → 写入缓存 → 返回。
///
/// 字节稳定契约：缓存仅在 `invalidate_cached_prompt` 显式调用后失效，
/// 会话内未失效时内容视为不变（byte-stable），保持 LLM Provider 端
/// prompt cache 温暖。
pub async fn get_or_build_cached_prompt(
    cache: &Arc<RwLock<CachedSystemPrompt>>,
    data_dir: &DataDir,
    role: &str,
    custom_instructions: Option<&str>,
    user_profile: Option<&UserProfileSnapshot>,
    load_extended_context: bool,
) -> String {
    // 缓存命中：直接返回（byte-stable 契约：会话内未失效则内容不变，忽略参数变化）
    {
        let cached = cache.read().await;
        if let Some((ref prompt, _)) = *cached {
            tracing::debug!(
                role = role,
                prompt_len = prompt.len(),
                "[tiered] system prompt cache hit, returning cached prompt"
            );
            return prompt.clone();
        }
    }

    // 缓存未命中：构建 + 渲染 + 写入缓存
    tracing::debug!(role = role, load_extended_context, "[tiered] system prompt cache miss, building");
    let parts = build_system_prompt_parts(
        data_dir,
        role,
        custom_instructions,
        user_profile,
        load_extended_context,
    )
    .await;
    let rendered = render_system_prompt(&parts);

    let mut cached = cache.write().await;
    *cached = Some((rendered.clone(), parts));

    tracing::info!(
        role = role,
        prompt_len = rendered.len(),
        "[tiered] system prompt built and cached"
    );
    rendered
}

/// 使 system prompt 缓存失效。
///
/// 仅在 context compression 后调用：压缩会改变 ContextTier 内容，
/// 需清除缓存以使下次调用重新构建。
pub async fn invalidate_cached_prompt(cache: &Arc<RwLock<CachedSystemPrompt>>) {
    let mut cached = cache.write().await;
    if cached.is_some() {
        tracing::debug!("[tiered] invalidating system prompt cache");
        *cached = None;
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::fs::data_dir::DataDir;

    // ── render_system_prompt 单元测试 ──

    fn sample_parts() -> SystemPromptParts {
        SystemPromptParts {
            stable: StableTier {
                soul: "SOUL_CONTENT".to_string(),
                skills: "SKILLS_CONTENT".to_string(),
                environment_hints: "ENV_CONTENT".to_string(),
            },
            context: ContextTier {
                caller_context: "CALLER_CONTEXT".to_string(),
                context_files: "FILES_CONTENT".to_string(),
            },
            volatile: VolatileTier {
                memory_snapshot: "MEMORY_CONTENT".to_string(),
                user_profile: "PROFILE_CONTENT".to_string(),
                timestamp: "Current date: 2026-07-23".to_string(),
            },
        }
    }

    #[test]
    fn render_byte_stable_same_input_same_output() {
        // 相同输入产生相同输出（字节稳定）
        let parts = sample_parts();
        let r1 = render_system_prompt(&parts);
        let r2 = render_system_prompt(&parts);
        assert_eq!(r1, r2);
    }

    #[test]
    fn render_joins_three_tiers_with_separator() {
        let parts = sample_parts();
        let rendered = render_system_prompt(&parts);

        // 三层之间用 \n\n---\n\n 连接
        let separators: usize = rendered.matches("\n\n---\n\n").count();
        assert_eq!(
            separators, 2,
            "三层应产生恰好 2 个 tier 分隔符, got: {}",
            separators
        );
    }

    #[test]
    fn render_joins_intra_tier_sections_with_double_newline() {
        let parts = sample_parts();
        let rendered = render_system_prompt(&parts);

        // Stable 层内三段用 \n\n 连接
        assert!(rendered.contains("SOUL_CONTENT\n\nSKILLS_CONTENT\n\nENV_CONTENT"));
        // Volatile 层内三段用 \n\n 连接
        assert!(rendered.contains("MEMORY_CONTENT\n\nPROFILE_CONTENT\n\nCurrent date: 2026-07-23"));
    }

    #[test]
    fn render_preserves_tier_order() {
        let parts = sample_parts();
        let rendered = render_system_prompt(&parts);

        let soul_pos = rendered.find("SOUL_CONTENT").unwrap();
        let caller_pos = rendered.find("CALLER_CONTEXT").unwrap();
        let memory_pos = rendered.find("MEMORY_CONTENT").unwrap();

        // Stable < Context < Volatile
        assert!(soul_pos < caller_pos, "Stable 应在 Context 之前");
        assert!(caller_pos < memory_pos, "Context 应在 Volatile 之前");
    }

    #[test]
    fn render_skips_empty_sections() {
        let parts = SystemPromptParts {
            stable: StableTier {
                soul: "SOUL".to_string(),
                skills: String::new(), // 空
                environment_hints: "   ".to_string(), // 空白
            },
            context: ContextTier {
                caller_context: String::new(), // 空
                context_files: String::new(),  // 空
            },
            volatile: VolatileTier {
                memory_snapshot: "MEM".to_string(),
                user_profile: String::new(),
                timestamp: "Current date: 2026-07-23".to_string(),
            },
        };
        let rendered = render_system_prompt(&parts);

        // Context 层全空 → 跳过，只剩 2 层 → 1 个分隔符
        let separators: usize = rendered.matches("\n\n---\n\n").count();
        assert_eq!(separators, 1, "空层应被跳过");
        assert!(rendered.contains("SOUL"));
        assert!(rendered.contains("MEM"));
        assert!(!rendered.contains("CALLER_CONTEXT"));
    }

    #[test]
    fn render_handles_all_empty_returns_empty() {
        let parts = SystemPromptParts {
            stable: StableTier {
                soul: String::new(),
                skills: String::new(),
                environment_hints: String::new(),
            },
            context: ContextTier {
                caller_context: String::new(),
                context_files: String::new(),
            },
            volatile: VolatileTier {
                memory_snapshot: String::new(),
                user_profile: String::new(),
                timestamp: String::new(),
            },
        };
        let rendered = render_system_prompt(&parts);
        assert!(rendered.is_empty(), "全空应返回空字符串");
    }

    // ── build_system_prompt_parts 单元测试 ──

    #[tokio::test]
    async fn build_parts_populates_stable_from_identity() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let parts = build_system_prompt_parts(&data_dir, "main", None, None, false).await;

        // SOUL.md 默认内容包含 Mnemosyne 身份声明
        assert!(
            parts.stable.soul.contains("Mnemosyne"),
            "stable.soul 应加载 SOUL.md 默认内容"
        );
        // environment_hints 来自 CONTEXT.md，包含 environment 章节
        assert!(
            parts.stable.environment_hints.contains("<environment>"),
            "stable.environment_hints 应加载 CONTEXT.md 内容"
        );
        // skills 暂为占位空字符串
        assert!(parts.stable.skills.is_empty(), "stable.skills 当前应为空");
    }

    #[tokio::test]
    async fn build_parts_populates_volatile_memory_and_timestamp() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let parts = build_system_prompt_parts(&data_dir, "main", None, None, false).await;

        // memory_snapshot 来自 MEMORY.md
        assert!(
            parts.volatile.memory_snapshot.contains("Agent 记忆"),
            "volatile.memory_snapshot 应加载 MEMORY.md"
        );
        // timestamp 格式为 "Current date: YYYY-MM-DD"
        assert!(
            parts.volatile.timestamp.starts_with("Current date: "),
            "volatile.timestamp 应有前缀, got: {}",
            parts.volatile.timestamp
        );
        // 仅精确到日（10 字符日期），不含时分秒
        let date_part = parts.volatile.timestamp.strip_prefix("Current date: ").unwrap();
        assert_eq!(
            date_part.len(),
            10,
            "timestamp 仅精确到日 (YYYY-MM-DD = 10 字符), got: {}",
            date_part
        );
    }

    #[tokio::test]
    async fn build_parts_includes_custom_instructions_in_context() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let parts =
            build_system_prompt_parts(&data_dir, "main", Some("Be extra careful"), None, false).await;

        assert_eq!(
            parts.context.caller_context, "Be extra careful",
            "context.caller_context 应承载 custom_instructions"
        );
    }

    #[tokio::test]
    async fn build_parts_omits_blank_custom_instructions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let parts = build_system_prompt_parts(&data_dir, "main", Some("   "), None, false).await;

        assert!(
            parts.context.caller_context.is_empty(),
            "空白 custom_instructions 不应进入 caller_context"
        );
    }

    #[tokio::test]
    async fn build_parts_includes_user_profile_in_volatile() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let profile = UserProfileSnapshot::default();

        let parts = build_system_prompt_parts(&data_dir, "main", None, Some(&profile), false).await;

        assert!(
            parts.volatile.user_profile.contains("## User Profile"),
            "volatile.user_profile 应包含格式化的 profile, got: {}",
            parts.volatile.user_profile
        );
    }

    #[tokio::test]
    async fn build_parts_byte_stable_same_day() {
        // 同一天内多次调用产生相同内容（timestamp 不变）
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let parts1 = build_system_prompt_parts(&data_dir, "main", None, None, false).await;
        let parts2 = build_system_prompt_parts(&data_dir, "main", None, None, false).await;

        assert_eq!(parts1.stable.soul, parts2.stable.soul);
        assert_eq!(parts1.volatile.timestamp, parts2.volatile.timestamp);
        assert_eq!(
            render_system_prompt(&parts1),
            render_system_prompt(&parts2),
            "同一天内相同输入应产生相同渲染结果"
        );
    }

    // ── 缓存逻辑单元测试 ──

    #[tokio::test]
    async fn cache_miss_builds_and_caches() {
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        assert!(cache.read().await.is_none(), "初始缓存应为空");

        let prompt = get_or_build_cached_prompt(&cache, &data_dir, "main", None, None, false).await;

        assert!(!prompt.is_empty(), "首次调用应返回非空 prompt");
        assert!(cache.read().await.is_some(), "首次调用后缓存应被填充");
    }

    #[tokio::test]
    async fn cache_hit_returns_cached_string() {
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let prompt1 = get_or_build_cached_prompt(&cache, &data_dir, "main", None, None, false).await;
        let prompt2 = get_or_build_cached_prompt(&cache, &data_dir, "main", None, None, false).await;

        assert_eq!(
            prompt1, prompt2,
            "缓存命中应返回相同字符串"
        );
    }

    #[tokio::test]
    async fn cache_hit_ignores_changed_arguments() {
        // 缓存命中时直接返回缓存，不重新构建（即使参数变化）。
        // 这体现 byte-stable 契约：会话内未失效则内容不变。
        // 参数变化的失效由 invalidate 显式触发。
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let prompt1 =
            get_or_build_cached_prompt(&cache, &data_dir, "main", Some("first"), None, false).await;
        // 第二次用不同 custom_instructions，但缓存命中应返回首次结果
        let prompt2 =
            get_or_build_cached_prompt(&cache, &data_dir, "main", Some("second"), None, false).await;

        assert_eq!(prompt1, prompt2, "缓存命中应忽略参数变化");
        assert!(prompt1.contains("first"), "应返回首次构建的内容");
        assert!(!prompt1.contains("second"));
    }

    #[tokio::test]
    async fn invalidate_clears_cache() {
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        get_or_build_cached_prompt(&cache, &data_dir, "main", None, None, false).await;
        assert!(cache.read().await.is_some(), "缓存应已填充");

        invalidate_cached_prompt(&cache).await;
        assert!(cache.read().await.is_none(), "invalidate 后缓存应为空");
    }

    #[tokio::test]
    async fn invalidate_then_rebuild_picks_up_new_arguments() {
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let prompt1 =
            get_or_build_cached_prompt(&cache, &data_dir, "main", Some("first"), None, false).await;
        invalidate_cached_prompt(&cache).await;
        let prompt2 =
            get_or_build_cached_prompt(&cache, &data_dir, "main", Some("second"), None, false).await;

        assert_ne!(prompt1, prompt2, "invalidate 后重建应反映新参数");
        assert!(prompt2.contains("second"));
        assert!(!prompt2.contains("first"));
    }

    #[tokio::test]
    async fn invalidate_on_empty_cache_is_noop() {
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        // 空缓存上调用 invalidate 不应 panic
        invalidate_cached_prompt(&cache).await;
        assert!(cache.read().await.is_none());
    }

    #[tokio::test]
    async fn cache_shared_across_arc_clones() {
        // AgentEngine clone 时 Arc 被克隆，缓存应共享
        let cache: Arc<RwLock<CachedSystemPrompt>> = Arc::new(RwLock::new(None));
        let cache_clone = Arc::clone(&cache);
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        let prompt1 = get_or_build_cached_prompt(&cache, &data_dir, "main", None, None, false).await;
        // 通过克隆的 Arc 读取，应命中共享缓存
        let prompt2 = get_or_build_cached_prompt(&cache_clone, &data_dir, "main", None, None, false).await;

        assert_eq!(prompt1, prompt2, "Arc 克隆应共享同一缓存");
    }

    // ── split_context_md 单元测试 ──

    #[test]
    fn split_context_md_splits_at_pipeline_overview() {
        let content = "CORE_SECTION\n\n<pipeline_overview>\nEXTENDED_CONTENT\n";
        let (core, extended) = split_context_md(content);

        assert!(core.contains("CORE_SECTION"), "core 应含分割点前内容");
        assert!(!core.contains("<pipeline_overview>"), "core 不应含分割点");
        assert!(
            extended.starts_with("<pipeline_overview>"),
            "extended 应从分割标签开始"
        );
        assert!(extended.contains("EXTENDED_CONTENT"), "extended 应含分割点后内容");
    }

    #[test]
    fn split_context_md_no_marker_returns_all_core() {
        let content = "ONLY_CORE_NO_MARKER\n";
        let (core, extended) = split_context_md(content);

        assert_eq!(core, content, "无标记时全文为 core");
        assert!(extended.is_empty(), "无标记时 extended 为空");
    }

    #[tokio::test]
    async fn build_parts_load_extended_false_omits_pipeline_overview() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        // simple chat：load_extended_context=false，environment_hints 仅含 core
        let parts = build_system_prompt_parts(&data_dir, "main", None, None, false).await;

        assert!(
            parts.stable.environment_hints.contains("<environment>"),
            "core 应含 environment 通用 section"
        );
        assert!(
            !parts.stable.environment_hints.contains("<pipeline_overview>"),
            "simple chat 不应加载 extended 的 pipeline_overview"
        );
    }

    #[tokio::test]
    async fn build_parts_load_extended_true_includes_pipeline_overview() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());

        // book/章节任务：load_extended_context=true，environment_hints 含 core + extended
        let parts = build_system_prompt_parts(&data_dir, "main", None, None, true).await;

        assert!(
            parts.stable.environment_hints.contains("<environment>"),
            "应含 core 的 environment section"
        );
        assert!(
            parts.stable.environment_hints.contains("<pipeline_overview>"),
            "load_extended=true 应加载 extended 的 pipeline_overview"
        );
    }
}
