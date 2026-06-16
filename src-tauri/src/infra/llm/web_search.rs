use crate::errors::AppError;
use serde::{Deserialize, Serialize};

/// Search result from web search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Search the web using Tavily API or fallback
pub async fn search_web(query: &str, max_results: usize) -> Result<Vec<SearchResult>, AppError> {
    // Try Tavily API first
    if let Ok(api_key) = std::env::var("TAVILY_API_KEY") {
        if !api_key.is_empty() {
            return search_tavily(&api_key, query, max_results).await;
        }
    }

    // Fallback: use DuckDuckGo Lite
    search_ddg(query, max_results).await
}

/// Fetch URL content
pub async fn fetch_url(url: &str, max_chars: usize) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::internal(format!("Failed to create client: {}", e)))?;

    let resp = client.get(url)
        .header("User-Agent", "Mnemosyne/0.1")
        .send().await
        .map_err(|e| AppError::internal(format!("Failed to fetch URL: {}", e)))?;

    let html = resp.text().await
        .map_err(|e| AppError::internal(format!("Failed to read response: {}", e)))?;

    // Simple HTML to text extraction
    let text = html_to_text(&html);
    if text.len() > max_chars {
        Ok(text[..max_chars].to_string())
    } else {
        Ok(text)
    }
}

async fn search_tavily(api_key: &str, query: &str, max_results: usize) -> Result<Vec<SearchResult>, AppError> {
    let client = reqwest::Client::new();
    let resp = client.post("https://api.tavily.com/search")
        .json(&serde_json::json!({
            "api_key": api_key,
            "query": query,
            "max_results": max_results,
        }))
        .send().await
        .map_err(|e| AppError::internal(format!("Tavily search failed: {}", e)))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| AppError::internal(format!("Tavily parse failed: {}", e)))?;

    let results = json["results"].as_array()
        .map(|arr| {
            arr.iter().filter_map(|item| {
                Some(SearchResult {
                    title: item["title"].as_str()?.to_string(),
                    url: item["url"].as_str()?.to_string(),
                    snippet: item["content"].as_str().unwrap_or("").to_string(),
                })
            }).collect()
        })
        .unwrap_or_default();

    Ok(results)
}

async fn search_ddg(query: &str, max_results: usize) -> Result<Vec<SearchResult>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::internal(format!("Failed to create client: {}", e)))?;

    let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query));
    let resp = client.get(&url)
        .header("User-Agent", "Mnemosyne/0.1")
        .send().await
        .map_err(|e| AppError::internal(format!("DDG search failed: {}", e)))?;

    let html = resp.text().await
        .map_err(|e| AppError::internal(format!("DDG read failed: {}", e)))?;

    let mut results = Vec::new();
    // Simple regex extraction from DDG lite HTML
    let re = regex::Regex::new(r#"<a[^>]*class="result-link"[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#).unwrap();
    for cap in re.captures_iter(&html) {
        if results.len() >= max_results { break; }
        results.push(SearchResult {
            title: cap[2].trim().to_string(),
            url: cap[1].to_string(),
            snippet: String::new(),
        });
    }

    Ok(results)
}

fn html_to_text(html: &str) -> String {
    // Remove script and style tags
    let re = regex::Regex::new(r"(?s)<script[^>]*>.*?</script>").unwrap();
    let no_scripts = re.replace_all(html, "");
    let re = regex::Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
    let no_scripts = re.replace_all(&no_scripts, "");
    // Remove HTML tags
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let text = re.replace_all(&no_scripts, " ");
    // Collapse whitespace
    let re = regex::Regex::new(r"\s+").unwrap();
    let text = re.replace_all(&text, " ");
    text.trim().to_string()
}
