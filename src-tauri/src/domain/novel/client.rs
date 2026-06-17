use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use crate::errors::AppError;
use super::types::*;

pub struct NovelClient {
    http: Client,
}

impl NovelClient {
    pub fn new() -> Result<Self, AppError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
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

        let url = search.url.clone();
        let method = search.method.to_uppercase();

        let resp = if method == "POST" {
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

            let mut req = self.http.post(&url);
            for (k, v) in &pairs {
                req = req.form(&[(k.as_str(), v.as_str())]);
            }
            req.send().await.map_err(|e| AppError::internal(format!("Search request failed: {}", e)))?
        } else {
            let url = url.replace("%s", keyword);
            self.http.get(&url).send().await.map_err(|e| AppError::internal(format!("Search request failed: {}", e)))?
        };

        let html = resp.text().await.map_err(|e| AppError::internal(format!("Failed to read response: {}", e)))?;
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

            let full_url = if link.starts_with("http") {
                link
            } else {
                format!("{}{}", source.url.trim_end_matches('/'), link)
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

            let full_url = if link.starts_with("http") {
                link
            } else if !toc.base_uri.is_empty() {
                format!("{}{}", toc.base_uri.trim_end_matches('/'), link)
            } else {
                let base = url.trim_end_matches('/');
                format!("{}/{}", base, link.trim_start_matches('/'))
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
                if !chapter_rule.paragraph_tag.is_empty() {
                    let parts: Vec<&str> = content.split("<br>").collect();
                    content = parts.join("\n");
                }
            }
        }

        if !chapter_rule.filter_txt.is_empty() {
            if let Ok(re) = regex::Regex::new(&chapter_rule.filter_txt) {
                content = re.replace_all(&content, "").to_string();
            }
        }

        let content = content.trim().to_string();

        Ok(ChapterContent {
            title,
            content,
            index: 0,
        })
    }

    async fn fetch_html(&self, url: &str) -> Result<String, AppError> {
        let resp = self.http.get(url).send().await
            .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;
        let html = resp.text().await
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
    Err(AppError::invalid_input(format!("Failed to extract book ID from URL: {}", url)))
}
