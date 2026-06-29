use crate::shared::errors::AppError;
use crate::infrastructure::db::models::{PlatformRankings, RadarResult, RadarRecommendation};
use crate::infrastructure::llm_client::types::Provider;
use crate::features::radar::source::RadarSource;
use std::sync::Arc;

pub struct RadarAgent {
    provider: Arc<dyn Provider>,
    model: String,
    sources: Vec<Box<dyn RadarSource>>,
}

impl RadarAgent {
    pub fn new(provider: Arc<dyn Provider>, model: String, sources: Vec<Box<dyn RadarSource>>) -> Self {
        Self { provider, model, sources }
    }

    pub async fn scan(&self) -> Result<(RadarResult, Vec<PlatformRankings>), AppError> {
        let mut handles = Vec::new();

        for source in &self.sources {
            let source_name = source.name().to_string();
            let src = &**source as *const dyn RadarSource;
            // SAFETY: We hold &self and sources won't be mutated
            let source_ref = unsafe { &*src };
            let fut = source_ref.fetch();
            handles.push((source_name, fut));
        }

        let mut rankings = Vec::new();
        for (name, fut) in handles {
            match fut.await {
                Ok(r) => rankings.push(r),
                Err(e) => {
                    tracing::warn!("Radar source '{}' failed: {}", name, e);
                }
            }
        }

        let rankings_text = format_rankings_for_prompt(&rankings);

        let system_prompt = format!(
            "你是一个专业的网络小说市场分析师。下面是从各平台实时抓取的排行榜数据，请基于这些真实数据分析市场趋势。\n\n\
             ## 实时排行榜数据\n\n{}\n\n\
             分析维度：\n\
             1. 从排行榜数据中识别当前热门题材和标签\n\
             2. 分析哪些类型的作品占据榜单高位\n\
             3. 发现市场空白和机会点（榜单上缺少但有潜力的方向）\n\
             4. 风险提示（榜单上过度扎堆的题材）\n\n\
             输出格式必须为 JSON：\n\
             {{\n\
             \x20\"recommendations\": [\n\
             \x20  {{\n\
             \x20    \"platform\": \"平台名\",\n\
             \x20    \"genre\": \"题材类型\",\n\
             \x20    \"concept\": \"一句话概念描述\",\n\
             \x20    \"confidence\": 0.0-1.0,\n\
             \x20    \"reasoning\": \"推荐理由（引用具体榜单数据）\",\n\
             \x20    \"benchmark_titles\": [\"对标书1\", \"对标书2\"]\n\
             \x20  }}\n\
             \x20],\n\
             \x20\"market_summary\": \"整体市场概述（基于真实榜单数据）\"\n\
             }}\n\n\
             推荐数量：3-5个，按 confidence 降序排列。只输出 JSON，不要其他内容。",
            rankings_text
        );

        let response = self.provider.complete(
            &self.model,
            &system_prompt,
            &[crate::infrastructure::llm_client::types::Message {
                role: "user".to_string(),
                content: "请基于上面的实时排行榜数据，分析当前网文市场热度，给出开书建议。".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
        ).await?;

        let result = parse_radar_json(&response)?;
        Ok((result, rankings))
    }
}

fn format_rankings_for_prompt(rankings: &[PlatformRankings]) -> String {
    let sections: Vec<String> = rankings
        .iter()
        .filter(|r| !r.entries.is_empty())
        .map(|r| {
            let lines: Vec<String> = r.entries.iter().map(|e| {
                let mut line = format!("- {}", e.title);
                if !e.author.is_empty() {
                    line.push_str(&format!(" ({})", e.author));
                }
                if !e.category.is_empty() {
                    line.push_str(&format!(" [{}]", e.category));
                }
                line.push_str(&format!(" {}", e.extra));
                line
            }).collect();
            format!("### {}\n{}", r.platform, lines.join("\n"))
        })
        .collect();

    if sections.is_empty() {
        "（未能获取到实时排行数据，请基于你的知识分析）".to_string()
    } else {
        sections.join("\n\n")
    }
}

fn parse_radar_json(content: &str) -> Result<RadarResult, AppError> {
    let start = content.find('{').ok_or_else(|| AppError::internal("No JSON found in radar output"))?;
    let mut depth = 0i32;
    let mut end = start;
    for (i, ch) in content[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    let json_str = &content[start..end];
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AppError::internal(format!("Radar JSON parse error: {}", e)))?;

    let recommendations: Vec<RadarRecommendation> = parsed["recommendations"]
        .as_array()
        .map(|arr| {
            arr.iter().map(|item| RadarRecommendation {
                platform: item["platform"].as_str().unwrap_or("").to_string(),
                genre: item["genre"].as_str().unwrap_or("").to_string(),
                concept: item["concept"].as_str().unwrap_or("").to_string(),
                confidence: item["confidence"].as_f64().unwrap_or(0.5),
                reasoning: item["reasoning"].as_str().unwrap_or("").to_string(),
                benchmark_titles: item["benchmark_titles"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
            }).collect()
        })
        .unwrap_or_default();

    let market_summary = parsed["market_summary"].as_str().unwrap_or("").to_string();

    Ok(RadarResult { recommendations, market_summary })
}
