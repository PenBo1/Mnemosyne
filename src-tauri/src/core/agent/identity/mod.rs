//! ═══════════════════════════════════════════════════════════════════════════
//! Identity - Agent 身份管理模块
//! ═══════════════════════════════════════════════════════════════════════════

mod agents_md_tracker;

pub use agents_md_tracker::AgentsMdTracker;

use std::path::PathBuf;

use crate::infrastructure::fs::data_dir::DataDir;
use super::prompts;
use super::user_profile::UserProfileSnapshot;

// ── 常量定义 ────────────────────────────────────────────────────────────────

const IDENTITY_MAX_CHARS: usize = 20_000;

const IDENTITY_TRUNCATE_HEAD_RATIO: f64 = 0.7;
const IDENTITY_TRUNCATE_TAIL_RATIO: f64 = 0.2;

// ── 身份类型枚举 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKind {
    Soul,
    Context,
    Memory,
}

impl IdentityKind {
    pub fn filename(self) -> &'static str {
        match self {
            IdentityKind::Soul => "SOUL.md",
            IdentityKind::Context => "CONTEXT.md",
            IdentityKind::Memory => "MEMORY.md",
        }
    }

    pub fn default_content(self) -> &'static str {
        prompts::default_for(self.filename()).unwrap_or("")
    }

    pub fn default_content_for(self, role: &str) -> &'static str {
        prompts::default_for_role(role, self.filename()).unwrap_or("")
    }
}

// ── 身份文件加载 ────────────────────────────────────────────────────────────

pub async fn load_identity(data_dir: &DataDir, role: &str, kind: IdentityKind) -> Option<String> {
    let path = identity_path(data_dir, role, kind);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) if !content.trim().is_empty() => {
            tracing::debug!(
                role = role,
                kind = kind.filename(),
                path = %path.display(),
                "Loaded identity from disk"
            );
            Some(content)
        }
        Ok(_) => {
            tracing::debug!(
                role = role,
                kind = kind.filename(),
                "Identity file empty, using default"
            );
            Some(kind.default_content_for(role).to_string())
        }
        Err(e) => {
            tracing::warn!(
                role = role,
                kind = kind.filename(),
                error = %e,
                "Failed to read identity file, using default"
            );
            Some(kind.default_content_for(role).to_string())
        }
    }
}

pub fn identity_path(data_dir: &DataDir, role: &str, kind: IdentityKind) -> PathBuf {
    data_dir.agents_dir().join(role).join(kind.filename())
}

// ── Token 估算 ──────────────────────────────────────────────────────────────

pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut cjk_chars: usize = 0;
    let mut english_words: usize = 0;
    let mut code_chars: usize = 0;
    let mut in_word = false;

    for ch in text.chars() {
        let is_cjk_char = ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{f900}'..='\u{faff}').contains(&ch);

        if is_cjk_char {
            cjk_chars += 1;
            in_word = false;
        } else if ch.is_ascii_alphabetic() {
            if !in_word {
                english_words += 1;
                in_word = true;
            }
        } else {
            code_chars += 1;
            in_word = false;
        }
    }

    let cjk_tokens = (cjk_chars as f64) * 1.5;
    let english_tokens = (english_words as f64) * 1.3;
    let code_tokens = (code_chars as f64) / 3.5;

    (cjk_tokens + english_tokens + code_tokens).ceil() as usize
}

// ── 身份内容截断 ────────────────────────────────────────────────────────────

fn truncate_identity_section(content: &str, filename: &str, role: &str) -> String {
    if content.chars().count() <= IDENTITY_MAX_CHARS {
        return content.to_string();
    }

    let total_chars = content.chars().count();
    let head_chars = (IDENTITY_MAX_CHARS as f64 * IDENTITY_TRUNCATE_HEAD_RATIO) as usize;
    let tail_chars = (IDENTITY_MAX_CHARS as f64 * IDENTITY_TRUNCATE_TAIL_RATIO) as usize;

    let head: String = content.chars().take(head_chars).collect();
    let tail: String = content
        .chars()
        .skip(total_chars.saturating_sub(tail_chars))
        .collect();

    let marker = format!(
        "\n\n[...truncated {filename}: kept {head_chars}+{tail_chars} of {total_chars} chars. \
         The middle is omitted — if you need the full instructions, read the complete file with \
         the read_file tool: agents/{role}/{filename}]\n\n"
    );

    tracing::warn!(
        filename = filename,
        role = role,
        original_chars = total_chars,
        head_chars,
        tail_chars,
        "Identity section truncated to fit token budget"
    );

    format!("{head}{marker}{tail}")
}

// ── 系统提示构建 ────────────────────────────────────────────────────────────

pub async fn build_system_prompt(
    data_dir: &DataDir,
    role: &str,
    custom_instructions: Option<&str>,
    user_profile: Option<&UserProfileSnapshot>,
    load_extended_context: bool,
) -> String {
    tracing::debug!(
        role = %role,
        has_custom_instructions = custom_instructions.is_some(),
        has_user_profile = user_profile.is_some(),
        load_extended_context,
        "[identity] build_system_prompt: starting"
    );

    // 通过三层架构构建 parts：tiered 内部加载 SOUL/CONTEXT(core[+extended])/MEMORY
    // + caller_context(custom_instructions) + user_profile + timestamp
    let mut parts = prompts::tiered::build_system_prompt_parts(
        data_dir,
        role,
        custom_instructions,
        user_profile,
        load_extended_context,
    )
    .await;

    // 保留 truncate 保护：防止超大 identity 文件撑爆 context
    // （tiered 当前不截断，由 identity 层兜底；engine 缓存路径见 engine.build_system_prompt）
    parts.stable.soul = truncate_identity_section(&parts.stable.soul, "SOUL.md", role);
    parts.stable.environment_hints =
        truncate_identity_section(&parts.stable.environment_hints, "CONTEXT.md", role);
    parts.volatile.memory_snapshot =
        truncate_identity_section(&parts.volatile.memory_snapshot, "MEMORY.md", role);

    let result = prompts::tiered::render_system_prompt(&parts);
    tracing::info!(
        role = %role,
        total_chars = result.len(),
        estimated_tokens = estimate_tokens(&result),
        "[identity] build_system_prompt: completed"
    );
    result
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_kind_filenames() {
        assert_eq!(IdentityKind::Soul.filename(), "SOUL.md");
        assert_eq!(IdentityKind::Context.filename(), "CONTEXT.md");
        assert_eq!(IdentityKind::Memory.filename(), "MEMORY.md");
    }

    #[test]
    fn default_content_nonempty() {
        assert!(!IdentityKind::Soul.default_content().is_empty());
        assert!(!IdentityKind::Context.default_content().is_empty());
        assert!(!IdentityKind::Memory.default_content().is_empty());
    }

    #[tokio::test]
    async fn load_identity_falls_back_to_default_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let soul = load_identity(&data_dir, "main", IdentityKind::Soul).await;
        assert!(soul.is_some());
        assert!(soul.unwrap().contains("Mnemosyne"));
    }

    #[tokio::test]
    async fn load_identity_reads_disk_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("main");
        std::fs::create_dir_all(&role_dir).unwrap();
        std::fs::write(role_dir.join("SOUL.md"), "Custom Persona Content").unwrap();

        let soul = load_identity(&data_dir, "main", IdentityKind::Soul).await;
        assert_eq!(soul.as_deref(), Some("Custom Persona Content"));
    }

    #[tokio::test]
    async fn load_identity_uses_default_when_file_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("main");
        std::fs::create_dir_all(&role_dir).unwrap();
        std::fs::write(role_dir.join("SOUL.md"), "   \n  \n").unwrap();

        let soul = load_identity(&data_dir, "main", IdentityKind::Soul).await;
        assert!(soul.is_some());
        assert!(soul.unwrap().contains("Mnemosyne"));
    }

    #[tokio::test]
    async fn build_prompt_joins_sections() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let prompt = build_system_prompt(&data_dir, "main", Some("Be extra careful"), None, false).await;
        assert!(prompt.contains("Mnemosyne"));
        assert!(prompt.contains("Tauri"));
        assert!(prompt.contains("Be extra careful"));
        assert!(prompt.contains("user_preferences"));
        assert!(prompt.contains("---"));
    }

    #[tokio::test]
    async fn build_prompt_includes_user_profile_when_provided() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let profile = UserProfileSnapshot::default();
        let prompt = build_system_prompt(&data_dir, "main", None, Some(&profile), false).await;
        assert!(prompt.contains("## User Profile"));
        assert!(prompt.contains("Mnemosyne"));
    }

    #[tokio::test]
    async fn build_prompt_omits_user_profile_when_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let prompt = build_system_prompt(&data_dir, "main", None, None, false).await;
        assert!(!prompt.contains("## User Profile"));
    }

    #[test]
    fn estimate_tokens_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_pure_english() {
        let tokens = estimate_tokens("hello world from the agent");
        assert!(tokens >= 6 && tokens <= 8, "got {tokens}");
    }

    #[test]
    fn estimate_tokens_pure_chinese() {
        let tokens = estimate_tokens("你好世界我是一个人工智能助手");
        assert!(tokens >= 20 && tokens <= 22, "got {tokens}");
    }

    #[test]
    fn estimate_tokens_mixed_content() {
        let text = "Hello 你好 World 世界 — Code: fn main() { println!(\"hi\"); }";
        let tokens = estimate_tokens(text);
        assert!(tokens > 10, "got {tokens}");
    }

    #[test]
    fn truncate_short_section_unchanged() {
        let content = "Short identity content";
        let result = truncate_identity_section(content, "SOUL.md", "main");
        assert_eq!(result, content);
    }

    #[test]
    fn truncate_long_section_inserts_marker() {
        let long_content = "A".repeat(IDENTITY_MAX_CHARS + 5000);
        let result = truncate_identity_section(&long_content, "SOUL.md", "main");
        assert!(result.contains("[...truncated SOUL.md"));
        assert!(result.contains("read_file tool: agents/main/SOUL.md"));
        assert!(result.starts_with('A'));
        assert!(result.ends_with('A'));
        assert!(result.chars().count() < long_content.chars().count());
    }

    #[test]
    fn truncate_preserves_head_and_tail_content() {
        let mut content = String::from("HEAD_START\n");
        content.push_str(&"X".repeat(IDENTITY_MAX_CHARS * 2));
        content.push_str("\nTAIL_END");
        let result = truncate_identity_section(&content, "CONTEXT.md", "main");
        assert!(result.contains("HEAD_START"), "head should be preserved");
        assert!(result.contains("TAIL_END"), "tail should be preserved");
        assert!(result.contains("[...truncated CONTEXT.md"));
    }

    #[tokio::test]
    async fn build_prompt_truncates_oversized_soul() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("main");
        std::fs::create_dir_all(&role_dir).unwrap();
        let oversized = format!("# Custom Soul\n\n{}\n\n# End", "B".repeat(IDENTITY_MAX_CHARS + 1000));
        std::fs::write(role_dir.join("SOUL.md"), &oversized).unwrap();

        let prompt = build_system_prompt(&data_dir, "main", None, None, false).await;
        assert!(prompt.contains("[...truncated SOUL.md"));
        assert!(prompt.contains("read_file tool: agents/main/SOUL.md"));
        assert!(prompt.chars().count() < oversized.chars().count() + 5000);
    }
}