// 研究报告生成器 —— 通过 AgentEngine.prompt_once 让 LLM 综合分析主题。
//
// 不调用外部 web search(无网络搜索依赖),
// 而是让 LLM 基于其知识库生成结构化研究报告。depth(quick/standard/deep)
// 控制 prompt 注入的查询角度数量(1/2/3),从而影响报告详尽程度。
//
// 输出结构: claims + conflicts + unknowns + creativeImplications + markdown。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::types::{ResearchClaim, ResearchInput, ResearchReport, ResearchSource};

/// 规范化 depth 字符串(非法值回退为 standard)。
fn normalize_depth(raw: &str) -> String {
    match raw {
        "quick" | "standard" | "deep" => raw.to_string(),
        _ => "standard".to_string(),
    }
}

/// 构造基于 depth 的查询角度提示片段。
fn build_query_angles(topic: &str, depth: &str) -> Vec<String> {
    let mut angles = vec![format!(
        "从背景与基础事实角度分析「{}」:定义、起源、关键人物/机构、核心概念。",
        topic
    )];
    if depth != "quick" {
        angles.push(format!(
            "从来源与争议角度分析「{}»:主流来源、存在分歧的观点、常见误区。",
            topic
        ));
    }
    if depth == "deep" {
        angles.push(format!(
            "从核查与未解问题角度分析「{}」:难以核实的事实、关键未知点、对创作的影响。",
            topic
        ));
    }
    angles
}

const SYSTEM_PROMPT_TEMPLATE: &str = r#"<identity>
You are a professional research analyst for novel-writing projects. You synthesize knowledge into structured research reports.
</identity>

<task>
Analyze the given topic across multiple query angles and produce a structured research report in JSON format.
</task>

<query_angles>
{angles}
</query_angles>

<output_format>
Respond with a single JSON object in this exact shape:
{
  "claims": [
    {
      "claim": "the factual statement",
      "sources": [
        { "title": "source title or work name", "url": "", "snippet": "brief context", "publishedAt": null }
      ],
      "confidence": 0.0-1.0
    }
  ],
  "conflicts": ["description of conflicting claims or viewpoints"],
  "unknowns": ["unresolved or unverifiable points"],
  "creativeImplications": ["how this can inform story/setting decisions"]
}

Rules:
- claims should be concrete and source-backed where possible; leave sources[].url empty if unknown.
- confidence: 0.0-1.0 (higher = more certain).
- If you genuinely lack knowledge on the topic, return empty claims and note it in unknowns.
- Do NOT fabricate specific URLs, dates, or citations you cannot verify.
- Output ONLY the JSON object, no surrounding prose.
</output_format>"#;

/// 生成研究报告。
///
/// 流程:
/// 1. 按 depth 构造查询角度
/// 2. 调用 AgentEngine.prompt_once
/// 3. 解析 JSON 为结构化报告
/// 4. 渲染 Markdown
pub async fn run_research_report(
    engine: &AgentEngine,
    input: &ResearchInput,
) -> Result<ResearchReport, AppError> {
    let topic = input.query.trim();
    if topic.is_empty() {
        return Err(AppError::invalid_input("research query cannot be empty"));
    }
    if topic.len() > 500 {
        return Err(AppError::invalid_input("research query too long (max 500 chars)"));
    }
    let depth = normalize_depth(input.depth.as_deref().unwrap_or("standard").trim());

    let angles = build_query_angles(topic, &depth);
    let angles_text = angles
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{}. {}", i + 1, a))
        .collect::<Vec<_>>()
        .join("\n");
    let system_prompt = SYSTEM_PROMPT_TEMPLATE.replace("{angles}", &angles_text);
    let user_message = format!("Topic: {}\nDepth: {}\n\nPlease produce the structured research report as JSON.", topic, depth);

    let raw = engine.prompt_once(&system_prompt, &user_message).await?;
    tracing::info!(
        topic = topic,
        depth = %depth,
        response_len = raw.len(),
        "Researcher LLM response received"
    );

    let report = parse_report(&raw, topic, &depth)?;
    Ok(report)
}

/// LLM 返回的 JSON 结构(用于反序列化)。
#[derive(serde::Deserialize)]
struct LlmResearchOutput {
    #[serde(default)]
    claims: Vec<LlmClaim>,
    #[serde(default)]
    conflicts: Vec<String>,
    #[serde(default)]
    unknowns: Vec<String>,
    #[serde(default)]
    #[serde(rename = "creativeImplications")]
    creative_implications: Vec<String>,
}

#[derive(serde::Deserialize)]
struct LlmClaim {
    claim: String,
    #[serde(default)]
    sources: Vec<LlmSource>,
    #[serde(default)]
    confidence: f64,
}

#[derive(serde::Deserialize)]
struct LlmSource {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    #[serde(rename = "publishedAt")]
    published_at: Option<String>,
}

/// 从 LLM 输出中提取 JSON 并解析为 ResearchReport。
fn parse_report(raw: &str, topic: &str, depth: &str) -> Result<ResearchReport, AppError> {
    let json_str = extract_json_block(raw).ok_or_else(|| {
        let preview = if raw.is_empty() {
            "(empty response)"
        } else if raw.len() > 300 {
            &raw[..300]
        } else {
            raw
        };
        AppError::invalid_input(format!(
            "Researcher output format error: no JSON found. LLM response (preview): {}",
            preview
        ))
    })?;

    let parsed: LlmResearchOutput = serde_json::from_str(json_str).map_err(|e| {
        AppError::invalid_input(format!("Researcher JSON parse error: {}", e))
    })?;

    let claims: Vec<ResearchClaim> = parsed
        .claims
        .into_iter()
        .filter_map(|c| {
            if c.claim.trim().is_empty() {
                return None;
            }
            let sources: Vec<ResearchSource> = c
                .sources
                .into_iter()
                .filter_map(|s| {
                    if s.title.trim().is_empty() && s.snippet.trim().is_empty() {
                        return None;
                    }
                    Some(ResearchSource {
                        title: s.title,
                        url: s.url,
                        snippet: s.snippet,
                        published_at: s.published_at,
                    })
                })
                .collect();
            Some(ResearchClaim {
                claim: c.claim,
                sources,
                confidence: c.confidence.clamp(0.0, 1.0),
            })
        })
        .collect();

    let report = ResearchReport {
        query: topic.to_string(),
        depth: depth.to_string(),
        claims: claims.clone(),
        conflicts: parsed.conflicts,
        unknowns: parsed.unknowns,
        creative_implications: parsed.creative_implications,
        markdown: String::new(),
    };

    let markdown = render_markdown(&report);
    Ok(ResearchReport { markdown, ..report })
}

/// 渲染研究报告为 Markdown。
fn render_markdown(report: &ResearchReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("# Research: {}", report.query));
    lines.push(String::new());
    lines.push(format!("- Depth: {}", report.depth));
    lines.push(String::new());

    lines.push("## Claims".to_string());
    if report.claims.is_empty() {
        lines.push("- No sourced claims collected.".to_string());
    } else {
        for claim in &report.claims {
            let source_labels: Vec<String> = (0..claim.sources.len())
                .map(|i| format!("[S{}]", i + 1))
                .collect();
            let label = if source_labels.is_empty() {
                String::new()
            } else {
                format!(" ({}, conf={:.2})", source_labels.join(", "), claim.confidence)
            };
            lines.push(format!("- {}{}", claim.claim, label));
            for (i, src) in claim.sources.iter().enumerate() {
                let title = if src.title.is_empty() { "(untitled)" } else { &src.title };
                lines.push(format!("  - [S{}] {}", i + 1, title));
                if !src.url.is_empty() {
                    lines.push(format!("    - URL: {}", src.url));
                }
                if !src.snippet.is_empty() {
                    lines.push(format!("    - {}", src.snippet));
                }
            }
        }
    }
    lines.push(String::new());

    lines.push("## Conflicts".to_string());
    if report.conflicts.is_empty() {
        lines.push("- None detected.".to_string());
    } else {
        for item in &report.conflicts {
            lines.push(format!("- {}", item));
        }
    }
    lines.push(String::new());

    lines.push("## Unknowns".to_string());
    if report.unknowns.is_empty() {
        lines.push("- None recorded.".to_string());
    } else {
        for item in &report.unknowns {
            lines.push(format!("- {}", item));
        }
    }
    lines.push(String::new());

    lines.push("## Creative implications".to_string());
    if report.creative_implications.is_empty() {
        lines.push("- Use sourced details as references; unresolved points should stay out of hard canon.".to_string());
    } else {
        for item in &report.creative_implications {
            lines.push(format!("- {}", item));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_depth_falls_back_to_standard() {
        assert_eq!(normalize_depth("quick"), "quick");
        assert_eq!(normalize_depth("standard"), "standard");
        assert_eq!(normalize_depth("deep"), "deep");
        assert_eq!(normalize_depth(""), "standard");
        assert_eq!(normalize_depth("bogus"), "standard");
    }

    #[test]
    fn build_query_angles_quick_has_one() {
        let angles = build_query_angles("宋代官制", "quick");
        assert_eq!(angles.len(), 1);
        assert!(angles[0].contains("宋代官制"));
    }

    #[test]
    fn build_query_angles_deep_has_three() {
        let angles = build_query_angles("宋代官制", "deep");
        assert_eq!(angles.len(), 3);
    }

    #[test]
    fn parse_report_valid_json() {
        let raw = r#"```json
        {
          "claims": [
            {"claim": "宋代设参知政事", "sources": [{"title": "宋史", "url": "", "snippet": "副相", "publishedAt": null}], "confidence": 0.8}
          ],
          "conflicts": ["某记载有出入"],
          "unknowns": [],
          "creativeImplications": ["可用于朝堂场景"]
        }
        ```"#;
        let report = parse_report(raw, "宋代官制", "standard").unwrap();
        assert_eq!(report.query, "宋代官制");
        assert_eq!(report.depth, "standard");
        assert_eq!(report.claims.len(), 1);
        assert_eq!(report.claims[0].claim, "宋代设参知政事");
        assert_eq!(report.claims[0].confidence, 0.8);
        assert_eq!(report.claims[0].sources.len(), 1);
        assert_eq!(report.conflicts.len(), 1);
        assert!(report.markdown.contains("# Research: 宋代官制"));
        assert!(report.markdown.contains("宋代设参知政事"));
    }

    #[test]
    fn parse_report_empty_claims() {
        let raw = r#"{"claims":[],"conflicts":[],"unknowns":["知识不足"],"creativeImplications":[]}"#;
        let report = parse_report(raw, "未知主题", "quick").unwrap();
        assert_eq!(report.claims.len(), 0);
        assert_eq!(report.unknowns.len(), 1);
    }

    #[test]
    fn parse_report_no_json_errors() {
        let result = parse_report("no json here", "topic", "quick");
        assert!(result.is_err());
    }

    #[test]
    fn parse_report_skips_empty_claims() {
        let raw = r#"{"claims":[{"claim":"","sources":[],"confidence":0},{"claim":"有效","sources":[],"confidence":0.5}]}"#;
        let report = parse_report(raw, "topic", "quick").unwrap();
        assert_eq!(report.claims.len(), 1);
        assert_eq!(report.claims[0].claim, "有效");
    }

    #[test]
    fn confidence_clamped_to_range() {
        let raw = r#"{"claims":[{"claim":"x","sources":[],"confidence":1.5}]}"#;
        let report = parse_report(raw, "t", "quick").unwrap();
        assert_eq!(report.claims[0].confidence, 1.0);
    }

    #[test]
    fn render_markdown_contains_all_sections() {
        let report = ResearchReport {
            query: "test".into(),
            depth: "standard".into(),
            claims: vec![],
            conflicts: vec!["c1".into()],
            unknowns: vec!["u1".into()],
            creative_implications: vec!["ci1".into()],
            markdown: String::new(),
        };
        let md = render_markdown(&report);
        assert!(md.contains("# Research: test"));
        assert!(md.contains("## Claims"));
        assert!(md.contains("## Conflicts"));
        assert!(md.contains("- c1"));
        assert!(md.contains("## Unknowns"));
        assert!(md.contains("- u1"));
        assert!(md.contains("## Creative implications"));
        assert!(md.contains("- ci1"));
    }
}
