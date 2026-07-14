// Agent 身份文件加载器 —— 从 <data_dir>/agents/<role>/ 读取 SOUL/CONTEXT/MEMORY.md。
//
// AGENTS.md:Agent identity files 由 core/init.rs 生成,运行时加载
//
// 加载策略:
// 1. 优先读磁盘文件(允许用户热编辑,无需重启)
// 2. 文件不存在或为空时回退到 prompts 模块中的默认模板
// 3. 读取失败只 log warn,不阻塞 agent 启动
//
// 体积管控:
// - 单段身份文件超过 `IDENTITY_MAX_CHARS` 时 head/tail 截断 + 插入 read_file 兜底 marker
// - 防止用户编辑超长 SOUL.md/CONTEXT.md 导致 system prompt 超过模型上下文窗口
// - marker 明确告知模型用 read_file 工具读取完整内容,把上下文负载转移到工具调用阶段

use std::path::PathBuf;

use crate::domain::user::types::UserProfile;
use crate::infrastructure::fs::data_dir::DataDir;
use super::prompts;

/// 单段身份文件的字符上限。
///
/// 超过此值时 head/tail 截断,保留头部 70% + 尾部 20%,中间 10% 留给 marker。
/// 默认 20K 字符 ≈ 5K-7K tokens,加上其他段后总 system prompt 仍在可控范围。
const IDENTITY_MAX_CHARS: usize = 20_000;

/// head/tail 截断比例。
const IDENTITY_TRUNCATE_HEAD_RATIO: f64 = 0.7;
const IDENTITY_TRUNCATE_TAIL_RATIO: f64 = 0.2;

/// 三种身份文件的种类。
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

    /// main role 的默认内容(向后兼容)。
    pub fn default_content(self) -> &'static str {
        prompts::default_for(self.filename()).unwrap_or("")
    }

    /// 任意 role 的默认内容。
    ///
    /// - main role:返回完整默认模板
    /// - pipeline role:返回对应 role 的简短种子
    /// - 未知 role:返回空字符串(理论不会发生,ALL_ROLES 已覆盖)
    pub fn default_content_for(self, role: &str) -> &'static str {
        prompts::default_for_role(role, self.filename()).unwrap_or("")
    }
}

/// 加载身份文件:磁盘优先,空则回退默认。
///
/// 返回 None 当且仅当磁盘和默认都为空(理论不会发生,默认非空)。
pub fn load_identity(data_dir: &DataDir, role: &str, kind: IdentityKind) -> Option<String> {
    let path = identity_path(data_dir, role, kind);
    match std::fs::read_to_string(&path) {
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
            // 文件存在但为空,使用默认
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

/// 计算身份文件路径(不读磁盘)。
pub fn identity_path(data_dir: &DataDir, role: &str, kind: IdentityKind) -> PathBuf {
    data_dir.agents_dir().join(role).join(kind.filename())
}

/// Token 估算。
///
/// 算法：
/// - 中文（CJK 统一表意文字 + 扩展 A）：1 字 ≈ 1.5 token
/// - 英文：按空格分词，1 词 ≈ 1.3 token
/// - 代码/符号字符：字符数 ÷ 3.5
///
/// 与前端 `src/features/agent/services/utils/context-assembly.ts::estimateTextTokens`
/// 保持算法一致，确保 Rust/前端口径统一。
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut cjk_chars: usize = 0;
    for ch in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{f900}'..='\u{faff}').contains(&ch)
        {
            cjk_chars += 1;
        }
    }

    // 移除 CJK 后按空白分词统计英文单词数
    let non_cjk: String = text
        .chars()
        .map(|ch| {
            if ('\u{4e00}'..='\u{9fff}').contains(&ch)
                || ('\u{3400}'..='\u{4dbf}').contains(&ch)
                || ('\u{f900}'..='\u{faff}').contains(&ch)
            {
                ' '
            } else {
                ch
            }
        })
        .collect();
    let english_words: usize = non_cjk
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_ascii_alphabetic()))
        .count();

    // 英文单词字符数（用于推算代码字符数）
    let english_chars: usize = non_cjk
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_ascii_alphabetic()))
        .map(|w| w.chars().count())
        .sum();

    let code_chars = text.chars().count().saturating_sub(cjk_chars).saturating_sub(english_chars);

    let cjk_tokens = (cjk_chars as f64) * 1.5;
    let english_tokens = (english_words as f64) * 1.3;
    let code_tokens = (code_chars as f64) / 3.5;

    (cjk_tokens + english_tokens + code_tokens).ceil() as usize
}

/// 单段身份文件超限时 head/tail 截断 + 插入 read_file 兜底 marker。
///
/// 截断规则：
/// - 保留头部 `IDENTITY_TRUNCATE_HEAD_RATIO`（0.7）
/// - 保留尾部 `IDENTITY_TRUNCATE_TAIL_RATIO`（0.2）
/// - 中间 0.1 留给 marker，明确告知模型用 read_file 读取完整内容
///
/// `filename` 仅用于 marker 提示，让模型知道去读哪个文件。
/// `role` 用于构造 read_file 路径提示（agents/<role>/<filename>）。
///
/// 未超限时原样返回。
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

/// 拼装完整 system prompt:SOUL + CONTEXT + (可选)UserProfile + (可选)custom_instructions + MEMORY。
///
/// 顺序对齐 stable/context/volatile 三层思路,但简化为单函数:
/// - stable:   SOUL.md(身份)
/// - context:  CONTEXT.md(任务上下文) + UserProfile(用户画像) + custom_instructions
/// - volatile: MEMORY.md(累积教训)
///
/// 各段以 `\n\n---\n\n` 分隔,避免内容粘连。
///
/// 体积管控：
/// - 每段身份文件加载后调用 `truncate_identity_section`,超过 `IDENTITY_MAX_CHARS` 时
///   head/tail 截断 + 插入 read_file 兜底 marker,把上下文负载转移到工具调用阶段
/// - 防止用户编辑超长 SOUL.md/CONTEXT.md 导致 system prompt 超过模型上下文窗口
pub fn build_system_prompt(
    data_dir: &DataDir,
    role: &str,
    custom_instructions: Option<&str>,
    user_profile: Option<&UserProfile>,
) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(5);

    if let Some(soul) = load_identity(data_dir, role, IdentityKind::Soul) {
        parts.push(truncate_identity_section(&soul, "SOUL.md", role));
    }
    if let Some(context) = load_identity(data_dir, role, IdentityKind::Context) {
        parts.push(truncate_identity_section(&context, "CONTEXT.md", role));
    }
    if let Some(profile) = user_profile {
        let formatted = profile.format_for_prompt();
        if !formatted.trim().is_empty() {
            parts.push(formatted);
        }
    }
    if let Some(extra) = custom_instructions {
        if !extra.trim().is_empty() {
            parts.push(format!("# Additional Instructions\n\n{}", extra));
        }
    }
    if let Some(memory) = load_identity(data_dir, role, IdentityKind::Memory) {
        parts.push(truncate_identity_section(&memory, "MEMORY.md", role));
    }

    parts.join("\n\n---\n\n")
}

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

    #[test]
    fn load_identity_falls_back_to_default_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        // 不创建文件,直接读 —— 应回退到默认
        let soul = load_identity(&data_dir, "main", IdentityKind::Soul);
        assert!(soul.is_some());
        assert!(soul.unwrap().contains("Mnemosyne"));
    }

    #[test]
    fn load_identity_reads_disk_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("main");
        std::fs::create_dir_all(&role_dir).unwrap();
        std::fs::write(role_dir.join("SOUL.md"), "Custom Persona Content").unwrap();

        let soul = load_identity(&data_dir, "main", IdentityKind::Soul);
        assert_eq!(soul.as_deref(), Some("Custom Persona Content"));
    }

    #[test]
    fn load_identity_uses_default_when_file_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("main");
        std::fs::create_dir_all(&role_dir).unwrap();
        std::fs::write(role_dir.join("SOUL.md"), "   \n  \n").unwrap();

        let soul = load_identity(&data_dir, "main", IdentityKind::Soul);
        assert!(soul.is_some());
        assert!(soul.unwrap().contains("Mnemosyne")); // 回退默认
    }

    #[test]
    fn build_prompt_joins_sections() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let prompt = build_system_prompt(&data_dir, "main", Some("Be extra careful"), None);
        assert!(prompt.contains("Mnemosyne"));        // SOUL
        assert!(prompt.contains("Tauri"));            // CONTEXT
        assert!(prompt.contains("Be extra careful")); // custom
        assert!(prompt.contains("user_preferences")); // MEMORY
        assert!(prompt.contains("---"));              // 分隔符
    }

    #[test]
    fn build_prompt_includes_user_profile_when_provided() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let profile = UserProfile::default();
        let prompt = build_system_prompt(&data_dir, "main", None, Some(&profile));
        assert!(prompt.contains("## User Profile"));
        assert!(prompt.contains("Mnemosyne")); // SOUL 仍在
    }

    #[test]
    fn build_prompt_omits_user_profile_when_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let prompt = build_system_prompt(&data_dir, "main", None, None);
        assert!(!prompt.contains("## User Profile"));
    }

    // ── estimate_tokens 单元测试 ──────────────────────────────────

    #[test]
    fn estimate_tokens_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_pure_english() {
        // 5 个英文单词 ≈ 5 * 1.3 = 6.5 → 7 tokens
        let tokens = estimate_tokens("hello world from the agent");
        assert!(tokens >= 6 && tokens <= 8, "got {tokens}");
    }

    #[test]
    fn estimate_tokens_pure_chinese() {
        // 14 个中文字 ≈ 14 * 1.5 = 21 tokens
        let tokens = estimate_tokens("你好世界我是一个人工智能助手");
        assert!(tokens >= 20 && tokens <= 22, "got {tokens}");
    }

    #[test]
    fn estimate_tokens_mixed_content() {
        // 混合内容应大于纯中文或纯英文
        let text = "Hello 你好 World 世界 — Code: fn main() { println!(\"hi\"); }";
        let tokens = estimate_tokens(text);
        assert!(tokens > 10, "got {tokens}");
    }

    // ── truncate_identity_section 单元测试 ────────────────────────

    #[test]
    fn truncate_short_section_unchanged() {
        let content = "Short identity content";
        let result = truncate_identity_section(content, "SOUL.md", "main");
        assert_eq!(result, content);
    }

    #[test]
    fn truncate_long_section_inserts_marker() {
        // 构造超过 IDENTITY_MAX_CHARS 的超长内容
        let long_content = "A".repeat(IDENTITY_MAX_CHARS + 5000);
        let result = truncate_identity_section(&long_content, "SOUL.md", "main");
        assert!(result.contains("[...truncated SOUL.md"));
        assert!(result.contains("read_file tool: agents/main/SOUL.md"));
        // 应保留头部和尾部
        assert!(result.starts_with('A'));
        assert!(result.ends_with('A'));
        // 结果长度应小于原始长度
        assert!(result.chars().count() < long_content.chars().count());
    }

    #[test]
    fn truncate_preserves_head_and_tail_content() {
        // 头部 "HEAD" + 中间填充 + 尾部 "TAIL"
        let mut content = String::from("HEAD_START\n");
        content.push_str(&"X".repeat(IDENTITY_MAX_CHARS * 2));
        content.push_str("\nTAIL_END");
        let result = truncate_identity_section(&content, "CONTEXT.md", "main");
        assert!(result.contains("HEAD_START"), "head should be preserved");
        assert!(result.contains("TAIL_END"), "tail should be preserved");
        assert!(result.contains("[...truncated CONTEXT.md"));
    }

    #[test]
    fn build_prompt_truncates_oversized_soul() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let role_dir = data_dir.agents_dir().join("main");
        std::fs::create_dir_all(&role_dir).unwrap();
        // 写入超长 SOUL.md 触发截断
        let oversized = format!("# Custom Soul\n\n{}\n\n# End", "B".repeat(IDENTITY_MAX_CHARS + 1000));
        std::fs::write(role_dir.join("SOUL.md"), &oversized).unwrap();

        let prompt = build_system_prompt(&data_dir, "main", None, None);
        assert!(prompt.contains("[...truncated SOUL.md"));
        assert!(prompt.contains("read_file tool: agents/main/SOUL.md"));
        // 截断后总长度应远小于原始
        assert!(prompt.chars().count() < oversized.chars().count() + 5000);
    }
}
