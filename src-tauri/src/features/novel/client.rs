use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use rand::Rng;
use url::Url;
use crate::shared::errors::AppError;
use super::types::*;

// Random User-Agent strings
const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
];

fn random_ua() -> &'static str {
    let mut rng = rand::thread_rng();
    USER_AGENTS[rng.gen_range(0..USER_AGENTS.len())]
}

fn random_interval() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(200..500)
}

/// Clean invisible Unicode characters that cause encoding issues
fn clean_invisible_chars(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() && *c != '\u{200B}' && *c != '\u{FEFF}')
        .collect()
}

pub struct NovelClient {
    http: Client,
}

impl NovelClient {
    pub fn new() -> Result<Self, AppError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| AppError::internal(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self { http })
    }

    pub async fn search(&self, source: &BookSource, keyword: &str) -> Result<Vec<SearchBookResult>, AppError> {
        let search = source.search.as_ref()
            .ok_or_else(|| AppError::invalid_input(format!("Source '{}' does not support search", source.name)))?;

        if search.disabled {
            return Err(AppError::invalid_input(format!("Search is disabled for source '{}'", source.name)));
        }

        let method = search.method.to_uppercase();
        let referer = Url::parse(&search.url)
            .ok()
            .and_then(|u| u.host_str().map(|h| format!("https://{}", h)))
            .unwrap_or_default();

        let resp = if method == "POST" {
            let url = if search.url.contains("%s") {
                search.url.replace("%s", keyword)
            } else {
                search.url.clone()
            };

            let body = search.data.replace("%s", keyword);
            let pairs: Vec<(String, String)> = body.trim_matches(|c| c == '{' || c == '}')
                .split(',')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, ':');
                    let key = parts.next()?.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
                    let val = parts.next()?.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
                    Some((key, val))
                })
                .collect();

            let mut req = self.http.post(&url)
                .header("User-Agent", random_ua())
                .header("Referer", &referer);
            if !search.cookies.is_empty() {
                req = req.header("Cookie", &search.cookies);
            }
            for (k, v) in &pairs {
                req = req.form(&[(k.as_str(), v.as_str())]);
            }
            req.send().await.map_err(|e| AppError::internal(format!("Search request failed: {}", e)))?
        } else {
            let url = search.url.replace("%s", keyword);
            let mut req = self.http.get(&url)
                .header("User-Agent", random_ua())
                .header("Referer", &referer);
            if !search.cookies.is_empty() {
                req = req.header("Cookie", &search.cookies);
            }
            req.send().await.map_err(|e| AppError::internal(format!("Search request failed: {}", e)))?
        };

        let html = resp.text().await
            .map(|h| clean_invisible_chars(&h))
            .map_err(|e| AppError::internal(format!("Failed to read response: {}", e)))?;
        let document = Html::parse_document(&html);

        let result_sel = Selector::parse(&search.result)
            .map_err(|e| AppError::internal(format!("Invalid result selector: {}", e)))?;

        let mut results = Vec::new();
        for element in document.select(&result_sel) {
            let book_name = extract_text(&element, &search.book_name);
            let author = extract_text(&element, &search.author);
            let link = extract_href(&element, &search.book_name);

            if book_name.is_empty() || link.is_empty() {
                continue;
            }

            let base_url = source.url.trim_end_matches('/');
            let full_url = if link.starts_with("http") {
                link
            } else if link.starts_with("/") {
                format!("{}{}", base_url, link)
            } else {
                format!("{}/{}", base_url, link)
            };

            results.push(SearchBookResult {
                book_name,
                author,
                url: full_url,
                category: extract_text(&element, &search.category),
                word_count: extract_text(&element, &search.word_count),
                status: extract_text(&element, &search.status),
                latest_chapter: extract_text(&element, &search.latest_chapter),
                last_update_time: extract_text(&element, &search.last_update_time),
                source_name: source.name.clone(),
                source_url: source.url.clone(),
            });
        }

        Ok(results)
    }

    pub async fn get_toc(&self, source: &BookSource, book_url: &str) -> Result<Vec<ChapterInfo>, AppError> {
        let toc = source.toc.as_ref()
            .ok_or_else(|| AppError::invalid_input(format!("Source '{}' has no TOC rules", source.name)))?;

        let url = if !toc.url.is_empty() {
            let id = extract_book_id(book_url, &toc.url)?;
            toc.url.replace("%s", &id)
        } else {
            book_url.to_string()
        };

        let html = self.fetch_html(&url).await?;
        let document = Html::parse_document(&html);

        let item_sel = Selector::parse(&toc.item)
            .map_err(|e| AppError::internal(format!("Invalid TOC item selector: {}", e)))?;

        let mut chapters = Vec::new();
        for (index, element) in document.select(&item_sel).enumerate() {
            let title = element.text().collect::<String>().trim().to_string();
            let link = extract_href_raw(&element);

            if title.is_empty() || link.is_empty() {
                continue;
            }

            let base_url = if !toc.base_uri.is_empty() {
                toc.base_uri.trim_end_matches('/')
            } else {
                url.trim_end_matches('/')
            };

            let full_url = if link.starts_with("http") {
                link
            } else if link.starts_with("/") {
                format!("{}{}", base_url, link)
            } else {
                format!("{}/{}", base_url, link.trim_start_matches('/'))
            };

            chapters.push(ChapterInfo {
                title,
                url: full_url,
                index,
            });
        }

        if toc.is_desc {
            chapters.reverse();
            for (i, ch) in chapters.iter_mut().enumerate() {
                ch.index = i;
            }
        }

        Ok(chapters)
    }

    pub async fn get_chapter_content(&self, source: &BookSource, chapter_url: &str) -> Result<ChapterContent, AppError> {
        let chapter_rule = source.chapter.as_ref()
            .ok_or_else(|| AppError::invalid_input(format!("Source '{}' has no chapter rules", source.name)))?;

        let html = self.fetch_html(chapter_url).await?;
        let document = Html::parse_document(&html);

        let title = extract_text(&document.root_element(), &chapter_rule.title);

        let content_sel = Selector::parse(&chapter_rule.content)
            .map_err(|e| AppError::internal(format!("Invalid content selector: {}", e)))?;

        let mut content = String::new();
        if let Some(content_el) = document.select(&content_sel).next() {
            if chapter_rule.paragraph_tag_closed {
                for child in content_el.children() {
                    if let Some(text) = child.value().as_text() {
                        let t = text.trim();
                        if !t.is_empty() {
                            content.push_str(t);
                            content.push('\n');
                        }
                    }
                }
            } else {
                content = content_el.text().collect::<String>();
                // Handle <br> tags as paragraph separators
                if !chapter_rule.paragraph_tag.is_empty() {
                    let re = regex::Regex::new(&chapter_rule.paragraph_tag).unwrap_or_else(|_| regex::Regex::new("<br>").unwrap());
                    content = re.replace_all(&content, "\n").to_string();
                }
            }
        }

        // Apply filter patterns
        if !chapter_rule.filter_txt.is_empty() {
            let patterns: Vec<&str> = chapter_rule.filter_txt.split('|').collect();
            for pattern in patterns {
                let pattern = pattern.trim();
                if !pattern.is_empty() {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        content = re.replace_all(&content, "").to_string();
                    }
                }
            }
        }

        // Remove HTML tags if filter_tag is specified
        if !chapter_rule.filter_tag.is_empty() {
            let re = regex::Regex::new(r"<[^>]+>").unwrap();
            content = re.replace_all(&content, "").to_string();
        }

        // Clean invisible characters
        content = clean_invisible_chars(&content);
        let content = content.trim().to_string();

        Ok(ChapterContent {
            title,
            content,
            index: 0,
        })
    }

    async fn fetch_html(&self, url: &str) -> Result<String, AppError> {
        let referer = Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| format!("https://{}", h)))
            .unwrap_or_default();

        // Add random delay to avoid rate limiting
        tokio::time::sleep(Duration::from_millis(random_interval())).await;

        let resp = self.http.get(url)
            .header("User-Agent", random_ua())
            .header("Referer", &referer)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;
        let html = resp.text().await
            .map(|h| clean_invisible_chars(&h))
            .map_err(|e| AppError::internal(format!("Failed to read response: {}", e)))?;
        Ok(html)
    }
}

fn extract_text(el: &scraper::ElementRef, selector: &str) -> String {
    if selector.is_empty() {
        return String::new();
    }
    if let Ok(sel) = Selector::parse(selector) {
        if let Some(target) = el.select(&sel).next() {
            return target.text().collect::<String>().trim().to_string();
        }
    }
    String::new()
}

fn extract_href(el: &scraper::ElementRef, selector: &str) -> String {
    if selector.is_empty() {
        return String::new();
    }
    if let Ok(sel) = Selector::parse(selector) {
        if let Some(target) = el.select(&sel).next() {
            return extract_href_raw(&target);
        }
    }
    String::new()
}

fn extract_href_raw(el: &scraper::ElementRef) -> String {
    el.value().attr("href").unwrap_or("").to_string()
}

fn extract_book_id(url: &str, pattern: &str) -> Result<String, AppError> {
    if let Ok(re) = regex::Regex::new(pattern) {
        if let Some(caps) = re.captures(url) {
            if let Some(m) = caps.get(1) {
                return Ok(m.as_str().to_string());
            }
        }
    }
    // If pattern doesn't match, try to extract ID from the URL path
    if let Some(id) = url.split('/').last() {
        if !id.is_empty() {
            return Ok(id.to_string());
        }
    }
    Err(AppError::invalid_input(format!("Failed to extract book ID from URL: {}", url)))
}
