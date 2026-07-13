// RadarAgent:雷达扫描核心逻辑。
//
// 流程:
// 1. 并行抓取所有数据源排行榜
// 2. 格式化为 prompt 文本
// 3. 调用 LLM 分析(要求输出 JSON)
// 4. 解析 JSON 为结构化结果

use crate::core::agent::engine::AgentEngine;
use crate::infrastructure::db::types::{PlatformRankings, RadarRecommendation, RadarResult};
use crate::shared::error::AppError;

use super::sources::{format_rankings_for_prompt, default_sources, RadarSource};

const SYSTEM_PROMPT_TEMPLATE: &str = r#"你是一个专业的网络小说市场分析师。下面是从各平台实时抓取的排行榜数据，请基于这些真实数据分析市场趋势。

## 实时排行榜数据

{rankings}

分析维度：
1. 从排行榜数据中识别当前热门题材和标签
2. 分析哪些类型的作品占据榜单高位
3. 发现市场空白和机会点（榜单上缺少但有潜力的方向）
4. 风险提示（榜单上过度扎堆的题材）

输出格式必须为 JSON：
{
  "recommendations": [
    {
      "platform": "平台名",
      "genre": "题材类型",
      "concept": "一句话概念描述",
      "confidence": 0.0-1.0,
      "reasoning": "推荐理由（引用具体榜单数据）",
      "benchmarkTitles": ["对标书1", "对标书2"]
    }
  ],
  "marketSummary": "整体市场概述（基于真实榜单数据）"
}

推荐数量：3-5个，按 confidence 降序排列。"#;

const USER_MESSAGE: &str = "请基于上面的实时排行榜数据，分析当前网文市场热度，给出开书建议。";

/// LLM 返回的 JSON 结构(用于反序列化)。
#[derive(serde::Deserialize)]
struct LlmRadarOutput {
    #[serde(default)]
    recommendations: Vec<RadarRecommendation>,
    #[serde(default)]
    #[serde(rename = "marketSummary")]
    market_summary: String,
}

/// 雷达扫描结果:LLM 分析结论 + 原始排行榜数据。
pub struct ScanOutcome {
    pub result: RadarResult,
    pub raw_rankings: Vec<PlatformRankings>,
}

/// 执行雷达扫描:抓取排行榜 → LLM 分析 → 解析 JSON。
///
/// `engine` 提供 LLM 调用能力(prompt_once),`sources` 可选覆盖默认数据源。
pub async fn scan(engine: &AgentEngine, sources: Option<Vec<Box<dyn RadarSource>>>) -> Result<ScanOutcome, AppError> {
    let sources = sources.unwrap_or_else(default_sources);

    // 1. 并行抓取所有数据源
    let mut rankings = Vec::with_capacity(sources.len());
    for source in &sources {
        let result = source.fetch().await;
        tracing::debug!(source = source.name(), entries = result.entries.len(), "Radar source fetched");
        rankings.push(result);
    }

    // 2. 格式化排行榜为 prompt 文本
    let rankings_text = format_rankings_for_prompt(&rankings);
    let system_prompt = SYSTEM_PROMPT_TEMPLATE.replace("{rankings}", &rankings_text);

    // 3. 调用 LLM 分析
    let response = engine.prompt_once(&system_prompt, USER_MESSAGE).await?;
    tracing::info!(
        response_len = response.len(),
        response_preview = &response[..response.len().min(200)],
        "Radar LLM response received"
    );

    // 4. 解析 JSON(容错:LLM 可能输出额外文字)
    let result = parse_result(&response)
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                full_response = %response,
                "Radar JSON parse failed, dumping full LLM response"
            );
            e
        })?;
    Ok(ScanOutcome { result, raw_rankings: rankings })
}

/// 从 LLM 输出中提取 JSON 并解析为 RadarResult。
/// 容错:用括号匹配提取首个 `{...}` JSON 块。
fn parse_result(content: &str) -> Result<RadarResult, AppError> {
    let json_str = extract_json_block(content).ok_or_else(|| {
        let preview = if content.is_empty() {
            "(empty response)"
        } else if content.len() > 300 {
            &content[..300]
        } else {
            content
        };
        AppError::invalid_input(format!(
            "Radar output format error: no JSON found. LLM response (preview): {}",
            preview
        ))
    })?;

    let parsed: LlmRadarOutput = serde_json::from_str(json_str)
        .map_err(|e| AppError::invalid_input(format!("Radar JSON parse error: {}", e)))?;

    Ok(RadarResult {
        recommendations: parsed.recommendations,
        market_summary: parsed.market_summary,
    })
}

/// 从文本中提取首个 `{...}` JSON 块(简单括号匹配)。
/// 优先尝试从 ```json ... ``` 代码块中提取,回退到全文搜索。
fn extract_json_block(content: &str) -> Option<&str> {
    // 先尝试从 ```json ... ``` 代码块中提取
    if let Some(start_marker) = content.find("```json") {
        let after_marker = &content[start_marker + 7..];
        if let Some(json_start) = after_marker.find('{') {
            let abs_start = start_marker + 7 + json_start;
            if let Some(block) = match_braces(&content[abs_start..]) {
                return Some(block);
            }
        }
    }
    // 回退:全文搜索首个 {...} 块
    match_braces(content)
}

/// 从以 `{` 开头的文本中,用括号匹配找到完整的 JSON 块。
fn match_braces(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in content[start..].char_indices() {
        // 字符串内部不计数,避免 JSON 值中的花括号干扰
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&content[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block_simple() {
        let input = r#"some text {"a":1} more text"#;
        assert_eq!(extract_json_block(input), Some(r#"{"a":1}"#));
    }

    #[test]
    fn test_extract_json_block_nested() {
        let input = r#"prefix {"a":{"b":2},"c":[1,2]} suffix"#;
        assert_eq!(extract_json_block(input), Some(r#"{"a":{"b":2},"c":[1,2]}"#));
    }

    #[test]
    fn test_extract_json_block_none() {
        let input = "no json here";
        assert_eq!(extract_json_block(input), None);
    }

    #[test]
    fn test_extract_json_block_markdown() {
        let input = "```json\n{\"a\":1}\n```";
        assert_eq!(extract_json_block(input), Some(r#"{"a":1}"#));
    }

    #[test]
    fn test_extract_json_block_braces_in_string() {
        let input = r#"{"text":"hello {world}","a":1}"#;
        assert_eq!(extract_json_block(input), Some(r#"{"text":"hello {world}","a":1}"#));
    }

    #[test]
    fn test_parse_result_valid() {
        let content = r#"前缀文字 {"recommendations":[{"platform":"番茄","genre":"都市","concept":"测试","confidence":0.8,"reasoning":"理由","benchmarkTitles":["书1"]}],"marketSummary":"市场概述"} 后缀"#;
        let result = parse_result(content).unwrap();
        assert_eq!(result.market_summary, "市场概述");
        assert_eq!(result.recommendations.len(), 1);
        assert_eq!(result.recommendations[0].platform, "番茄");
    }

    #[test]
    fn test_parse_result_markdown_codeblock() {
        let content = "分析如下：\n```json\n{\"recommendations\":[],\"marketSummary\":\"测试\"}\n```\n结束";
        let result = parse_result(content).unwrap();
        assert_eq!(result.market_summary, "测试");
        assert_eq!(result.recommendations.len(), 0);
    }
}
