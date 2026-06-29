use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::db::models::{PlatformRankings, RankingEntry};

#[async_trait]
pub trait RadarSource: Send + Sync {
    fn name(&self) -> &str;
    async fn fetch(&self) -> Result<PlatformRankings, AppError>;
}

pub struct FanqieRadarSource;

#[async_trait]
impl RadarSource for FanqieRadarSource {
    fn name(&self) -> &str { "fanqie" }

    async fn fetch(&self) -> Result<PlatformRankings, AppError> {
        let client = reqwest::Client::new();
        let mut entries = Vec::new();

        let rank_types: &[(i32, &str)] = &[(10, "热门榜"), (13, "黑马榜")];

        for &(side_type, label) in rank_types {
            let url = format!(
                "https://api-lf.fanqiesdk.com/api/novel/channel/homepage/rank/rank_list/v2/?aid=13&limit=15&offset=0&side_type={}",
                side_type
            );
            let resp = client
                .get(&url)
                .header("User-Agent", "Mozilla/5.0 (compatible; Mnemosyne/0.1)")
                .send()
                .await;
            let resp = match resp {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };
            let json: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };
            let list = match json["data"]["result"].as_array() {
                Some(arr) => arr,
                None => continue,
            };
            for item in list {
                entries.push(RankingEntry {
                    title: item["book_name"].as_str().unwrap_or("").to_string(),
                    author: item["author"].as_str().unwrap_or("").to_string(),
                    category: item["category"].as_str().unwrap_or("").to_string(),
                    extra: format!("[{}]", label),
                });
            }
        }

        Ok(PlatformRankings { platform: "番茄小说".to_string(), entries })
    }
}

pub struct QidianRadarSource;

#[async_trait]
impl RadarSource for QidianRadarSource {
    fn name(&self) -> &str { "qidian" }

    async fn fetch(&self) -> Result<PlatformRankings, AppError> {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://www.qidian.com/rank/")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch Qidian: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(PlatformRankings { platform: "起点中文网".to_string(), entries: Vec::new() });
        }

        let html = resp.text().await
            .map_err(|e| AppError::internal(format!("Failed to read Qidian response: {}", e)))?;

        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let re = regex::Regex::new(r#"<a[^>]*href="//book\.qidian\.com/info/(\d+)"[^>]*>([^<]+)</a>"#)
            .map_err(|e| AppError::internal(format!("Regex compile error: {}", e)))?;

        for cap in re.captures_iter(&html) {
            let title = cap[2].trim().to_string();
            if title.len() > 1 && title.len() < 30 && seen.insert(title.clone()) {
                entries.push(RankingEntry {
                    title,
                    author: String::new(),
                    category: String::new(),
                    extra: "[起点热榜]".to_string(),
                });
                if entries.len() >= 20 { break; }
            }
        }

        Ok(PlatformRankings { platform: "起点中文网".to_string(), entries })
    }
}

pub fn default_sources() -> Vec<Box<dyn RadarSource>> {
    vec![
        Box::new(FanqieRadarSource),
        Box::new(QidianRadarSource),
    ]
}
