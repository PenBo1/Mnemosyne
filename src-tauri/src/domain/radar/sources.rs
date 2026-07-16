// 雷达数据源:从小说平台抓取排行榜数据。
//
// 用 reqwest 异步抓取 + regex 解析。
// 网络错误静默跳过(返回空 entries),不阻断扫描流程——
// LLM 会基于已获取的数据分析,全部失败时基于自身知识分析。

use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;

use crate::infrastructure::db::types::{PlatformRankings, RankingEntry};

use super::types::RadarSourceInfo;

// ── 复用 HTTP 客户端(OnceLock 缓存,避免每次 fetch 重建) ──────────

fn fanqie_client() -> Option<&'static reqwest::Client> {
    static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; Mnemosyne/0.1)")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .ok()
        })
        .as_ref()
}

fn qidian_client() -> Option<&'static reqwest::Client> {
    static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .ok()
        })
        .as_ref()
}

/// 可插拔的雷达数据源接口。
#[async_trait]
pub trait RadarSource: Send + Sync {
    /// 数据源名称
    fn name(&self) -> &str;
    /// 抓取平台排行榜
    async fn fetch(&self) -> PlatformRankings;
}

// ── 番茄小说 ─────────────────────────────────────────────────

const FANQIE_RANK_TYPES: &[(u32, &str)] = &[(10, "热门榜"), (13, "黑马榜")];

pub struct FanqieRadarSource;

impl Default for FanqieRadarSource {
    fn default() -> Self {
        Self::new()
    }
}

impl FanqieRadarSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RadarSource for FanqieRadarSource {
    fn name(&self) -> &str {
        "fanqie"
    }

    async fn fetch(&self) -> PlatformRankings {
        let client = match fanqie_client() {
            Some(c) => c,
            None => return PlatformRankings { platform: "番茄小说".into(), entries: vec![] },
        };

        let mut entries: Vec<RankingEntry> = Vec::new();

        for (side_type, label) in FANQIE_RANK_TYPES {
            let url = format!(
                "https://api-lf.fanqiesdk.com/api/novel/channel/homepage/rank/rank_list/v2/?aid=13&limit=15&offset=0&side_type={}",
                side_type
            );
            let resp = match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };
            let body: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(list) = body.pointer("/data/result").and_then(|v| v.as_array()) {
                for item in list {
                    entries.push(RankingEntry {
                        title: item.get("book_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        author: item.get("author").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        category: item.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        extra: format!("[{}]", label),
                    });
                }
            }
        }

        PlatformRankings { platform: "番茄小说".into(), entries }
    }
}

// ── 起点中文网 ───────────────────────────────────────────────

pub struct QidianRadarSource;

impl Default for QidianRadarSource {
    fn default() -> Self {
        Self::new()
    }
}

impl QidianRadarSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RadarSource for QidianRadarSource {
    fn name(&self) -> &str {
        "qidian"
    }

    async fn fetch(&self) -> PlatformRankings {
        let client = match qidian_client() {
            Some(c) => c,
            None => return PlatformRankings { platform: "起点中文网".into(), entries: vec![] },
        };

        let resp = match client.get("https://www.qidian.com/rank/").send().await {
            Ok(r) if r.status().is_success() => r,
            _ => return PlatformRankings { platform: "起点中文网".into(), entries: vec![] },
        };
        let html = match resp.text().await {
            Ok(t) => t,
            Err(_) => return PlatformRankings { platform: "起点中文网".into(), entries: vec![] },
        };

        // 正则提取书名
        let re = match Regex::new(r#"<a[^>]*href="//book\.qidian\.com/info/\d+"[^>]*>([^<]+)</a>"#) {
            Ok(r) => r,
            Err(_) => return PlatformRankings { platform: "起点中文网".into(), entries: vec![] },
        };

        let mut seen = std::collections::HashSet::new();
        let mut entries: Vec<RankingEntry> = Vec::new();

        for cap in re.captures_iter(&html) {
            let title = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if title.is_empty() || title.len() <= 1 || title.len() >= 30 {
                continue;
            }
            if !seen.insert(title.to_string()) {
                continue;
            }
            entries.push(RankingEntry {
                title: title.to_string(),
                author: String::new(),
                category: String::new(),
                extra: "[起点热榜]".into(),
            });
            if entries.len() >= 20 {
                break;
            }
        }

        PlatformRankings { platform: "起点中文网".into(), entries }
    }
}

/// 格式化排行榜数据为 prompt 文本。
pub fn format_rankings_for_prompt(rankings: &[PlatformRankings]) -> String {
    let sections: Vec<String> = rankings
        .iter()
        .filter(|r| !r.entries.is_empty())
        .map(|r| {
            let lines: Vec<String> = r
                .entries
                .iter()
                .map(|e| {
                    let author = if e.author.is_empty() { String::new() } else { format!(" ({})", e.author) };
                    let category = if e.category.is_empty() { String::new() } else { format!(" [{}]", e.category) };
                    format!("- {}{}{} {}", e.title, author, category, e.extra)
                })
                .collect();
            format!("### {}\n{}", r.platform, lines.join("\n"))
        })
        .collect();

    if sections.is_empty() {
        "（未能获取到实时排行数据，请基于你的知识分析）".to_string()
    } else {
        sections.join("\n\n")
    }
}

/// 默认数据源列表。
pub fn default_sources() -> Vec<Box<dyn RadarSource>> {
    vec![
        Box::new(FanqieRadarSource::new()),
        Box::new(QidianRadarSource::new()),
    ]
}

// ── 文本数据源(用户外部分析注入) ─────────────────────────────

/// 用原始自然语言文本作为数据源。
/// 用于把外部抓取/分析结果(如 OpenClaw)注入雷达 pipeline。
pub struct TextRadarSource {
    name: String,
    text: String,
}

impl TextRadarSource {
    pub fn new(text: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
        }
    }
}

#[async_trait]
impl RadarSource for TextRadarSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn fetch(&self) -> PlatformRankings {
        PlatformRankings {
            platform: self.name.clone(),
            entries: vec![RankingEntry {
                title: self.text.clone(),
                author: String::new(),
                category: String::new(),
                extra: "[外部分析]".into(),
            }],
        }
    }
}

/// 列出内置数据源元信息(供 radar_list_sources 命令使用)。
pub fn builtin_source_infos() -> Vec<RadarSourceInfo> {
    vec![
        RadarSourceInfo {
            name: "fanqie".into(),
            label: "番茄小说".into(),
            kind: "builtin".into(),
        },
        RadarSourceInfo {
            name: "qidian".into(),
            label: "起点中文网".into(),
            kind: "builtin".into(),
        },
        RadarSourceInfo {
            name: "text".into(),
            label: "外部分析文本".into(),
            kind: "text".into(),
        },
    ]
}
