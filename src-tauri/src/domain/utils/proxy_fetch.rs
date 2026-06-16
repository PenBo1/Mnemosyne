//! Proxy fetch utilities for HTTP requests through proxies.

use crate::errors::AppError;

/// Fetch a URL with optional proxy support
pub async fn fetch_with_proxy(
    url: &str,
    options: FetchOptions,
) -> Result<FetchResponse, AppError> {
    let mut client_builder = reqwest::Client::builder();

    if let Some(proxy) = &options.proxy_url {
        client_builder = client_builder.proxy(reqwest::Proxy::all(proxy)
            .map_err(|e| AppError::internal(format!("Invalid proxy: {}", e)))?);
    }

    if let Some(timeout) = options.timeout_ms {
        client_builder = client_builder.timeout(std::time::Duration::from_millis(timeout));
    }

    let client = client_builder.build()
        .map_err(|e| AppError::internal(format!("Failed to create client: {}", e)))?;

    let mut request = client.get(url);

    if let Some(headers) = &options.headers {
        for (key, value) in headers {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    if let Some(user_agent) = &options.user_agent {
        request = request.header("User-Agent", user_agent.as_str());
    }

    let response = request.send().await
        .map_err(|e| AppError::internal(format!("Request failed: {}", e)))?;

    let status = response.status().as_u16();
    let body = response.text().await
        .map_err(|e| AppError::internal(format!("Failed to read response: {}", e)))?;

    Ok(FetchResponse { status, body })
}

pub struct FetchOptions {
    pub proxy_url: Option<String>,
    pub timeout_ms: Option<u64>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub user_agent: Option<String>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            proxy_url: None,
            timeout_ms: Some(10_000),
            headers: None,
            user_agent: Some("Mnemosyne/0.1".to_string()),
        }
    }
}

pub struct FetchResponse {
    pub status: u16,
    pub body: String,
}
