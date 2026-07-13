// Composer Agent。
//
// 职责：为 writer 编译上下文。包含两个 LLM 子任务：
// 1. select_outline_sections —— 语义选段，从候选大纲段落中挑出当前章节真正需要的部分
// 2. compile_compressible_context —— 语义压缩，把可压缩上下文编译为简洁 Markdown
//
// prompt 策略：语义选段 + 受保护/可压缩分区编译模式。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

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
    // 候选 ≤ 1 直接返回全部
    if should_skip_selection(&request.candidates) {
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
    let selected = parse_selected_sources(&response);
    Ok(selected
        .into_iter()
        .filter(|s| allowed.contains(s.as_str()))
        .collect())
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
    let system_prompt = build_compile_system_prompt();
    let user_message = build_compile_user_message(request);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(response.trim().to_string())
}

fn build_select_system_prompt() -> String {
    r###"你是创作系统的语义大纲选段器。
只选择当前章节真正需要的大纲段落。按语义相关性判断，不要按关键词重合机械选择。
只返回严格 JSON：{"selectedSources":["..."]}。必须使用候选里的精确 source id；不确定时选最安全的相关锚点，不要编造 id。"###.to_string()
}

fn build_select_user_message(request: &OutlineSelectionRequest) -> String {
    let candidates = request
        .candidates
        .iter()
        .enumerate()
        .map(|(i, c)| format!("#{} {}\nheading: {}\n{}", i + 1, c.source, c.heading, c.excerpt))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r###"文件：{file_name}
章节：第{chapter_number}章
目标：{goal}
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
    r###"你是创作系统的语义上下文编译器。
只能编译【可压缩上下文】。【受保护上下文】是绑定参照，不得改写、不得替代总结、不得削弱。
输出简洁 Markdown，保留来源指针。保留会影响下一章的人名、未兑现承诺、证据、时间点和约束，丢弃低相关噪声。"###.to_string()
}

fn build_compile_user_message(request: &CompressibleContextRequest) -> String {
    let protected_block = render_context_entries(&request.protected_entries);
    let compressible_block = render_context_entries(&request.compressible_entries);

    format!(
        r###"章节：第{chapter_number}章
目标：{goal}
压缩后目标预算：不超过 {max_input_tokens} 估算输入 tokens

## 受保护上下文（只作为参照，不要编译它）
{protected_block}

## 可压缩上下文（只编译这一部分）
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
                "### {}\nReason: {}\n{}",
                e.source,
                e.reason,
                if e.excerpt.is_empty() {
                    "(no excerpt)"
                } else {
                    &e.excerpt
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 从 LLM 输出解析 selectedSources JSON。
/// 多策略：纯JSON → ```json代码块 → 平衡大括号提取 → 空数组兜底。
fn parse_selected_sources(content: &str) -> Vec<String> {
    let trimmed = content.trim();

    // 策略 1: 整体作为 JSON
    if let Ok(parsed) = serde_json::from_str::<SelectedSourcesJson>(trimmed) {
        return parsed.selected_sources;
    }

    // 策略 2: 提取 ```json ... ``` 代码块
    if let Some(json_str) = extract_json_code_block(trimmed) {
        if let Ok(parsed) = serde_json::from_str::<SelectedSourcesJson>(json_str) {
            return parsed.selected_sources;
        }
    }

    // 策略 3: 提取第一个平衡的 { ... } 对象
    if let Some(json_str) = extract_balanced_json(trimmed) {
        if let Ok(parsed) = serde_json::from_str::<SelectedSourcesJson>(&json_str) {
            return parsed.selected_sources;
        }
    }

    // 策略 4: 兜底空数组
    Vec::new()
}

#[derive(serde::Deserialize)]
struct SelectedSourcesJson {
    #[serde(default, rename = "selectedSources")]
    selected_sources: Vec<String>,
}

fn extract_json_code_block(content: &str) -> Option<&str> {
    let start_marker = "```json";
    let start = content.find(start_marker)?;
    let json_start = start + start_marker.len();
    let end = content[json_start..].find("```")?;
    Some(content[json_start..json_start + end].trim())
}

fn extract_balanced_json(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for (i, c) in content[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(content[start..start + i + c.len_utf8()].to_string());
            }
        }
    }
    None
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
        let result = parse_selected_sources(content);
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn parses_json_code_block() {
        let content = "```json\n{\"selectedSources\":[\"a\"]}\n```";
        let result = parse_selected_sources(content);
        assert_eq!(result, vec!["a".to_string()]);
    }

    #[test]
    fn parses_balanced_json_with_prefix() {
        let content = "前文 {\"selectedSources\":[\"x\"]} 后文";
        let result = parse_selected_sources(content);
        assert_eq!(result, vec!["x".to_string()]);
    }

    #[test]
    fn parses_invalid_returns_empty() {
        // 非 JSON 且不含平衡大括号 → 兜底空数组
        let result = parse_selected_sources("not json");
        assert!(result.is_empty());
    }

    #[test]
    fn parses_empty_array_field() {
        // selectedSources 为空数组 → 返回空 Vec
        let content = r#"{"selectedSources":[]}"#;
        let result = parse_selected_sources(content);
        assert!(result.is_empty());
    }

    #[test]
    fn parses_missing_field_defaults_empty() {
        // 缺少 selectedSources 字段 → serde default 给空 Vec
        let content = r#"{"other":"value"}"#;
        let result = parse_selected_sources(content);
        assert!(result.is_empty());
    }

    #[test]
    fn extract_json_code_block_finds_block() {
        let content = "prefix\n```json\n{\"a\":1}\n```\nsuffix";
        assert_eq!(extract_json_code_block(content), Some("{\"a\":1}"));
    }

    #[test]
    fn extract_json_code_block_returns_none_without_marker() {
        assert_eq!(extract_json_code_block("no code block here"), None);
    }

    #[test]
    fn extract_balanced_json_handles_nested() {
        // 嵌套大括号 + 字符串内的 } 不应提前闭合
        let content = r#"prefix {"a":{"b":"}"},"c":1} suffix"#;
        let result = extract_balanced_json(content);
        assert_eq!(result.as_deref(), Some(r#"{"a":{"b":"}"},"c":1}"#));
    }

    #[test]
    fn extract_balanced_json_returns_none_without_brace() {
        assert_eq!(extract_balanced_json("no braces here"), None);
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
        assert!(rendered.contains("Reason: 主线锚点"));
        assert!(rendered.contains("主角进城"));
        assert!(rendered.contains("### wiki/chars.md#h2"));
        assert!(rendered.contains("(no excerpt)"));
    }

    #[test]
    fn renders_empty_context_entries() {
        let entries: Vec<ContextEntry> = vec![];
        assert_eq!(render_context_entries(&entries), "");
    }
}
