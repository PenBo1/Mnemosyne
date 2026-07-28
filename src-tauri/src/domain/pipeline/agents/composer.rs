//! ═══════════════════════════════════════════════════════════════════════════
//! Composer Agent - 上下文编译代理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：为 writer 编译上下文。包含两个 LLM 子任务：
//! 1. select_outline_sections —— 语义选段，从候选大纲段落中挑出当前章节真正需要的部分
//! 2. compile_compressible_context —— 语义压缩，把可压缩上下文编译为简洁 Markdown

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::{extract_json_block, match_braces};

/// 大纲段落候选
#[derive(Debug, Clone)]
pub struct OutlineSectionCandidate {
    pub source: String, // 例如 "story/outline/story_frame.md#section-anchor"
    pub heading: String,
    pub excerpt: String,
}

/// 选段请求
pub struct OutlineSelectionRequest {
    pub file_name: String,
    pub kind: OutlineKind,
    pub chapter_number: u32,
    pub goal: String,
    pub outline_node: String,
    pub candidates: Vec<OutlineSectionCandidate>,
}

/// 大纲类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineKind {
    StoryFrame,
    VolumeMap,
}

/// 可压缩上下文编译请求
pub struct CompressibleContextRequest {
    pub chapter_number: u32,
    pub goal: String,
    pub max_input_tokens: u32,
    pub protected_entries: Vec<ContextEntry>,
    pub compressible_entries: Vec<ContextEntry>,
}

/// 上下文条目（用于压缩）
#[derive(Debug, Clone)]
pub struct ContextEntry {
    pub source: String,
    pub reason: String,
    pub excerpt: String,
}

/// 语义选段：从候选大纲段落中选择当前章节真正需要的部分。
///
/// 候选数 ≤ 1 时直接返回全部，不调用 LLM。
/// 输出 JSON: {"selectedSources":["..."]}，过滤掉不在候选 source 集合中的 id。
pub async fn select_outline_sections(
    engine: &AgentEngine,
    request: &OutlineSelectionRequest,
) -> Result<Vec<String>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        function = "select_outline_sections",
        chapter_number = request.chapter_number,
        candidates_count = request.candidates.len(),
        "入口"
    );

    // 候选 ≤ 1 直接返回全部
    if should_skip_selection(&request.candidates) {
        tracing::info!(
            function = "select_outline_sections",
            chapter_number = request.chapter_number,
            skipped = true,
            reason = "candidates <= 1",
            duration_ms = start.elapsed().as_millis() as u64,
            "出口"
        );
        return Ok(request
            .candidates
            .iter()
            .map(|c| c.source.clone())
            .collect());
    }

    let system_prompt = build_select_system_prompt();
    let user_message = build_select_user_message(request);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;

    // 过滤掉不在候选 source 集合中的 id，防止 LLM 编造来源指针
    let allowed: std::collections::HashSet<&str> = request
        .candidates
        .iter()
        .map(|c| c.source.as_str())
        .collect();
    let selected = parse_selected_sources(&response)?;
    let result: Vec<String> = selected
        .into_iter()
        .filter(|s| allowed.contains(s.as_str()))
        .collect();

    tracing::info!(
        function = "select_outline_sections",
        chapter_number = request.chapter_number,
        selected_count = result.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "出口"
    );

    Ok(result)
}

/// 判断是否需要调用 LLM（候选 ≤ 1 时直接返回全部，无需 LLM）
fn should_skip_selection(candidates: &[OutlineSectionCandidate]) -> bool {
    candidates.len() <= 1
}

/// 语义压缩：将可压缩上下文编译为简洁 Markdown。
///
/// 受保护上下文只作为参照，不得改写/替代/削弱。
/// 保留会影响下一章的人名、未兑现承诺、证据、时间点、约束，丢弃低相关噪声。
pub async fn compile_compressible_context(
    engine: &AgentEngine,
    request: &CompressibleContextRequest,
) -> Result<String, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        function = "compile_compressible_context",
        chapter_number = request.chapter_number,
        protected_count = request.protected_entries.len(),
        compressible_count = request.compressible_entries.len(),
        max_tokens = request.max_input_tokens,
        "入口"
    );

    let system_prompt = build_compile_system_prompt();
    let user_message = build_compile_user_message(request);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;

    tracing::info!(
        function = "compile_compressible_context",
        chapter_number = request.chapter_number,
        result_len = response.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "出口"
    );

    Ok(response.trim().to_string())
}

fn build_select_system_prompt() -> String {
    r###"<identity>
你是创作系统的语义大纲选段器（semantic outline-section selector）。
</identity>

<responsibilities>
仅选择本章真正需要的大纲段落。判断依据是语义相关性，而非机械的关键词重叠。
</responsibilities>

<outputs>
仅返回严格 JSON：{"selectedSources":["..."]}。必须使用候选列表中确切的 source id。当不确定时，选择最安全的相关锚点——绝不编造 id。
</outputs>

<safety>
- NEVER 编造不在候选列表中的 source id；越界 id 会被下游过滤丢弃。
- NEVER 仅凭字面关键词命中就选段；必须从章节目标（goal）的语义出发判断。
- NEVER 输出 JSON 以外的解释性文字、Markdown 代码围栏或前后缀。
</safety>

<verification>
在交付前自检：
1. 返回的是否仅是 `{"selectedSources":[...]}` 这一个 JSON 对象？
2. 数组中每个 id 是否都来自候选 source 集合？
3. 选中数量是否 ≤ 候选总数？
</verification>"###.to_string()
}

fn build_select_user_message(request: &OutlineSelectionRequest) -> String {
    let candidates = request
        .candidates
        .iter()
        .enumerate()
        .map(|(i, c)| format!("#{} {}\n标题：{}\n{}", i + 1, c.source, c.heading, c.excerpt))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r###"文件：{file_name}
章节：{chapter_number}
本章目标：{goal}
大纲节点：{outline_node}

候选段落：
{candidates}"###,
        file_name = request.file_name,
        chapter_number = request.chapter_number,
        goal = request.goal,
        outline_node = request.outline_node,
        candidates = candidates,
    )
}

fn build_compile_system_prompt() -> String {
    r###"<identity>
你是创作系统的语义上下文编译器（semantic context compiler）。
</identity>

<rules>
你只能编译【可压缩上下文】。【受保护上下文】是绑定参照：不得改写、不得用摘要替代、不得削弱。
输出简洁 Markdown 并保留来源指针。保留会影响下一章的人名、未兑现承诺、证据、时间点、约束；丢弃低相关噪声。
</rules>

<safety>
- NEVER 改写、替代或削弱【受保护上下文】；它必须原样作为参照保留。
- NEVER 丢弃会影响下一章的人名、承诺、证据、时间点或硬约束。
- NEVER 编造未在原文出现的细节；压缩只做减法，不做加法。
</safety>

<verification>
在交付前自检：
1. 输出是否仅包含【可压缩上下文】的编译结果，未混入受保护上下文的改写？
2. 来源指针（source 路径锚点）是否保留？
3. 估算 token 数是否在 user message 给出的预算内？
4. 是否丢弃了低相关噪声（不影响下一章的细节）？
</verification>"###.to_string()
}

fn build_compile_user_message(request: &CompressibleContextRequest) -> String {
    let protected_block = render_context_entries(&request.protected_entries);
    let compressible_block = render_context_entries(&request.compressible_entries);

    format!(
        r###"章节：{chapter_number}
本章目标：{goal}
压缩后目标预算：不超过 {max_input_tokens} 估算输入 token

## Protected Context（受保护上下文：仅作参照，不得编译此部分）
{protected_block}

## Compressible Context（可压缩上下文：仅编译此部分）
{compressible_block}"###,
        chapter_number = request.chapter_number,
        goal = request.goal,
        max_input_tokens = request.max_input_tokens,
        protected_block = if protected_block.is_empty() {
            "（无）".to_string()
        } else {
            protected_block
        },
        compressible_block = if compressible_block.is_empty() {
            "（无）".to_string()
        } else {
            compressible_block
        },
    )
}

fn render_context_entries(entries: &[ContextEntry]) -> String {
    entries
        .iter()
        .map(|e| {
            format!(
                "### {}\n理由：{}\n{}",
                e.source,
                e.reason,
                if e.excerpt.is_empty() {
                    "（无摘录）"
                } else {
                    &e.excerpt
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 从 LLM 输出解析 selectedSources JSON。
/// 多策略：纯JSON → ```json代码块 → 平衡大括号提取 → 失败返回错误（不再静默兜底空数组）。
fn parse_selected_sources(content: &str) -> Result<Vec<String>, AppError> {
    let trimmed = content.trim();

    // 策略 1: 整体作为 JSON
    if let Ok(parsed) = serde_json::from_str::<SelectedSourcesJson>(trimmed) {
        return Ok(parsed.selected_sources);
    }

    // 策略 2: 提取 ```json ... ``` 代码块（shared 版内部回退到 match_braces）
    if let Some(json_str) = extract_json_block(trimmed) {
        if let Ok(parsed) = serde_json::from_str::<SelectedSourcesJson>(json_str) {
            return Ok(parsed.selected_sources);
        }
    }

    // 策略 3: 提取第一个平衡的 { ... } 对象
    if let Some(json_str) = match_braces(trimmed) {
        if let Ok(parsed) = serde_json::from_str::<SelectedSourcesJson>(json_str) {
            return Ok(parsed.selected_sources);
        }
    }

    // 所有策略失败 → 显式错误（不再静默兜底）
    Err(AppError::invalid_format(format!(
        "parse_selected_sources: 无法从 LLM 响应中解析 selectedSources JSON（响应前 200 字符：{}）",
        trimmed.chars().take(200).collect::<String>()
    )))
}

#[derive(serde::Deserialize)]
struct SelectedSourcesJson {
    #[serde(default, rename = "selectedSources")]
    selected_sources: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(source: &str, heading: &str, excerpt: &str) -> OutlineSectionCandidate {
        OutlineSectionCandidate {
            source: source.to_string(),
            heading: heading.to_string(),
            excerpt: excerpt.to_string(),
        }
    }

    #[test]
    fn returns_all_when_single_candidate() {
        // 1 个候选 → 无需 LLM，直接返回全部
        let candidates = vec![candidate("a.md#h1", "H1", "excerpt")];
        assert!(should_skip_selection(&candidates));
    }

    #[test]
    fn returns_all_when_empty_candidates() {
        // 0 个候选 → 无需 LLM，返回空
        let candidates: Vec<OutlineSectionCandidate> = vec![];
        assert!(should_skip_selection(&candidates));
    }

    #[test]
    fn does_not_skip_when_multiple_candidates() {
        // ≥ 2 个候选 → 必须调用 LLM
        let candidates = vec![
            candidate("a.md#h1", "H1", "excerpt1"),
            candidate("b.md#h2", "H2", "excerpt2"),
        ];
        assert!(!should_skip_selection(&candidates));
    }

    #[test]
    fn parses_pure_json() {
        let content = r#"{"selectedSources":["a","b"]}"#;
        let result = parse_selected_sources(content).unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn parses_json_code_block() {
        let content = "```json\n{\"selectedSources\":[\"a\"]}\n```";
        let result = parse_selected_sources(content).unwrap();
        assert_eq!(result, vec!["a".to_string()]);
    }

    #[test]
    fn parses_balanced_json_with_prefix() {
        let content = "前文 {\"selectedSources\":[\"x\"]} 后文";
        let result = parse_selected_sources(content).unwrap();
        assert_eq!(result, vec!["x".to_string()]);
    }

    #[test]
    fn parses_invalid_returns_err() {
        // 非 JSON 且不含平衡大括号 → 显式错误（不再静默兜底空数组）
        let result = parse_selected_sources("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parses_empty_array_field() {
        // selectedSources 为空数组 → 返回空 Vec
        let content = r#"{"selectedSources":[]}"#;
        let result = parse_selected_sources(content).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parses_missing_field_defaults_empty() {
        // 缺少 selectedSources 字段 → serde default 给空 Vec
        let content = r#"{"other":"value"}"#;
        let result = parse_selected_sources(content).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn extract_json_block_finds_code_block() {
        let content = "prefix\n```json\n{\"a\":1}\n```\nsuffix";
        assert_eq!(extract_json_block(content), Some("{\"a\":1}"));
    }

    #[test]
    fn extract_json_block_returns_none_without_marker() {
        assert_eq!(extract_json_block("no code block here"), None);
    }

    #[test]
    fn match_braces_handles_nested() {
        // 嵌套大括号 + 字符串内的 } 不应提前闭合
        let content = r#"prefix {"a":{"b":"}"},"c":1} suffix"#;
        let result = match_braces(content);
        assert_eq!(result, Some(r#"{"a":{"b":"}"},"c":1}"#));
    }

    #[test]
    fn match_braces_returns_none_without_brace() {
        assert_eq!(match_braces("no braces here"), None);
    }

    #[test]
    fn renders_context_entries() {
        let entries = vec![
            ContextEntry {
                source: "story/outline.md#a1".to_string(),
                reason: "主线锚点".to_string(),
                excerpt: "主角进城".to_string(),
            },
            ContextEntry {
                source: "wiki/chars.md#h2".to_string(),
                reason: "人设参照".to_string(),
                excerpt: String::new(),
            },
        ];
        let rendered = render_context_entries(&entries);
        assert!(rendered.contains("### story/outline.md#a1"));
        assert!(rendered.contains("理由：主线锚点"));
        assert!(rendered.contains("主角进城"));
        assert!(rendered.contains("### wiki/chars.md#h2"));
        assert!(rendered.contains("（无摘录）"));
    }

    #[test]
    fn renders_empty_context_entries() {
        let entries: Vec<ContextEntry> = vec![];
        assert_eq!(render_context_entries(&entries), "");
    }
}
