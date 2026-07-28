//! ═══════════════════════════════════════════════════════════════════════════
//! 雷达智能体 - 市场扫描分析
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 流程:
//! 1. 并行抓取所有数据源排行榜
//! 2. 格式化为 prompt 文本
//! 3. 调用 LLM 分析(要求输出 JSON)
//! 4. 解析 JSON 为结构化结果

use crate::core::agent::engine::AgentEngine;
use crate::infrastructure::db::types::{PlatformRankings, RadarRecommendation, RadarResult};
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::sources::{format_rankings_for_prompt, default_sources, RadarSource};

const SYSTEM_PROMPT_TEMPLATE: &str = r#"<identity>
You are a professional web-novel market analyst. Below is real-time ranking data scraped from various platforms. Analyze market trends based on this real data.
</identity>

## Real-time Ranking Data

{rankings}

<responsibilities>
1. Identify currently hot themes and tags from the ranking data.
2. Analyze which categories of works occupy the top of the charts.
3. Discover market gaps and opportunities (directions that are absent from the charts but have potential).
4. Flag risks (themes that are overcrowded on the charts).
</responsibilities>

<outputs>
The output must be JSON in this exact format:
{
  "recommendations": [
    {
      "platform": "platform name",
      "genre": "genre type",
      "concept": "one-line concept description",
      "confidence": 0.0-1.0,
      "reasoning": "recommendation rationale (cite specific chart data)",
      "benchmarkTitles": ["benchmark title 1", "benchmark title 2"]
    }
  ],
  "marketSummary": "overall market summary (based on real chart data)"
}

Number of recommendations: 3-5, sorted by confidence in descending order.
</outputs>"#;

const USER_MESSAGE: &str = "Based on the real-time ranking data above, analyze current web-novel market trends and provide book-launch recommendations.";

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
    let start = std::time::Instant::now();
    tracing::info!("[RadarAgent] Starting radar scan");
    
    let sources = sources.unwrap_or_else(default_sources);

    // 1. 并行抓取所有数据源
    tracing::debug!(source_count = sources.len(), "[RadarAgent] Fetching from sources");
    let rankings = futures_util::future::join_all(sources.iter().map(|s| s.fetch())).await;
    for (source, result) in sources.iter().zip(&rankings) {
        tracing::debug!(source = source.name(), entries = result.entries.len(), "[RadarAgent] Source fetched");
    }

    // 2. 格式化排行榜为 prompt 文本
    let rankings_text = format_rankings_for_prompt(&rankings);
    let system_prompt = SYSTEM_PROMPT_TEMPLATE.replace("{rankings}", &rankings_text);

    // 3. 调用 LLM 分析
    tracing::debug!("[RadarAgent] Calling LLM for analysis");
    let response = engine.prompt_once(&system_prompt, USER_MESSAGE).await?;
    tracing::info!(
        response_len = response.len(),
        response_preview = safe_char_slice(&response, 200),
        "[RadarAgent] LLM response received"
    );

    // 4. 解析 JSON(容错:LLM 可能输出额外文字)
    let result = parse_result(&response)
        .map_err(|e| {
            tracing::warn!(
                error = %e,
                full_response = %response,
                "[RadarAgent] JSON parse failed, dumping full LLM response"
            );
            e
        })?;
    
    tracing::info!(
        recommendation_count = result.recommendations.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "[RadarAgent] Radar scan completed"
    );
    Ok(ScanOutcome { result, raw_rankings: rankings })
}

/// 安全切片:按字符边界截取前 max_chars 个字符(避免 UTF-8 字节切片 panic)。
fn safe_char_slice(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// 从 LLM 输出中提取 JSON 并解析为 RadarResult。
/// 容错:用括号匹配提取首个 `{...}` JSON 块。
fn parse_result(content: &str) -> Result<RadarResult, AppError> {
    let json_str = extract_json_block(content).ok_or_else(|| {
        let preview = if content.is_empty() {
            "(empty response)"
        } else {
            safe_char_slice(content, 300)
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

#[cfg(test)]
mod tests {
    use super::*;

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
