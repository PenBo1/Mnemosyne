//! ═══════════════════════════════════════════════════════════════════════════
//! HTTP 客户端 - 小说爬虫 HTTP 请求封装
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 单例 `reqwest::Client`,提供带 UA/Referer 头的 GET / POST 表单请求。
//! POST 数据使用 `{key: %s, key2: value2}` 简化格式，
//! 其中 `%s` 占位符在调用时按位置替换为参数。

use std::time::Duration;

use reqwest::{Client, header::{HeaderMap, HeaderValue, REFERER, USER_AGENT}};
use url::Url;

use crate::shared::error::AppError;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// 获取共享 HTTP 客户端单例。
fn client() -> Client {
    // 静态 OnceCell 风格的惰性初始化:用 leak 换取跨线程共享。
    // 一次构建后整个进程复用,避免每次请求都重建连接池。
    use std::sync::OnceLock;
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .expect("failed to build reqwest client")
        })
        .clone()
}

/// 构造请求头:UA + Referer(指向源站 host)。
fn build_headers(url: &str) -> Result<HeaderMap, AppError> {
    let parsed = Url::parse(url).map_err(|_| AppError::invalid_input(format!("invalid url: {}", url)))?;
    let host = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_UA));
    headers.insert(REFERER, HeaderValue::from_str(&host).unwrap_or_else(|_| HeaderValue::from_static("")));
    Ok(headers)
}

/// GET 请求,返回 HTML 文本(reqwest 自动按 charset 解码)。
pub async fn get_html(url: &str, cookies: &str) -> Result<String, AppError> {
    let mut req = client().get(url).headers(build_headers(url)?);
    if !cookies.is_empty() {
        req = req.header("Cookie", cookies);
    }
    let resp = req.send().await.map_err(map_reqwest_err)?;
    if !resp.status().is_success() {
        return Err(AppError::internal(format!(
            "HTTP {} for {}",
            resp.status(),
            url
        )));
    }
    resp.text().await.map_err(map_reqwest_err)
}

/// POST 表单请求。`data_pattern` 形如 `{searchkey: %s, searchtype: all}`,
/// `args` 按 `%s` 出现顺序替换。
pub async fn post_form_html(
    url: &str,
    cookies: &str,
    data_pattern: &str,
    args: &[&str],
) -> Result<String, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(url, "http_post: enter");
    
    let result = async {
        let form = parse_form_pattern(data_pattern, args);
        let body = form
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let mut req = client()
            .post(url)
            .headers(build_headers(url)?)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body);
        if !cookies.is_empty() {
            req = req.header("Cookie", cookies);
        }
        let resp = req.send().await.map_err(map_reqwest_err)?;
        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "HTTP {} for {}",
                resp.status(),
                url
            )));
        }
        resp.text().await.map_err(map_reqwest_err)
    }.await;
    
    match &result {
        Ok(_) => tracing::info!(
            duration_ms = start.elapsed().as_millis(),
            "http_post: exit"
        ),
        Err(e) => tracing::error!(
            error = %e,
            duration_ms = start.elapsed().as_millis(),
            "http_post: error"
        ),
    }
    result
}

/// 解析 `{key: value, key2: %s}` 形式的伪 JSON 为表单字段列表。
/// `%s` 占位符按 `args` 顺序替换;非 `%s` 的值原样使用。
fn parse_form_pattern(pattern: &str, args: &[&str]) -> Vec<(String, String)> {
    let trimmed = pattern.trim().trim_start_matches('{').trim_end_matches('}');
    let mut arg_idx = 0;
    let mut out = Vec::new();
    for pair in trimmed.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let Some((k, v)) = pair.split_once(':') else {
            continue;
        };
        let key = k.trim().trim_matches(|c: char| c == '"' || c == '\'').to_string();
        let raw_val = v.trim().trim_matches(|c: char| c == '"' || c == '\'');
        let value = if raw_val == "%s" {
            let arg = args.get(arg_idx).copied().unwrap_or("");
            arg_idx += 1;
            arg.to_string()
        } else {
            raw_val.to_string()
        };
        out.push((key, value));
    }
    out
}

fn map_reqwest_err(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::network_timeout()
    } else if e.is_connect() {
        AppError::network_unreachable()
    } else {
        AppError::internal(format!("request error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_pattern() {
        let f = parse_form_pattern("{searchkey: %s}", &["斗破苍穹"]);
        assert_eq!(f, vec![("searchkey".to_string(), "斗破苍穹".to_string())]);
    }

    #[test]
    fn parses_multi_field_pattern() {
        let f = parse_form_pattern("{searchkey: %s, searchtype: all}", &["天龙八部"]);
        assert_eq!(
            f,
            vec![
                ("searchkey".to_string(), "天龙八部".to_string()),
                ("searchtype".to_string(), "all".to_string()),
            ]
        );
    }

    #[test]
    fn parses_quoted_keys() {
        let f = parse_form_pattern(r#"{"q": "rust", "page": %s}"#, &["2"]);
        assert_eq!(
            f,
            vec![
                ("q".to_string(), "rust".to_string()),
                ("page".to_string(), "2".to_string()),
            ]
        );
    }

    #[test]
    fn handles_empty_pattern() {
        assert!(parse_form_pattern("{}", &[]).is_empty());
        assert!(parse_form_pattern("", &[]).is_empty());
    }
}
