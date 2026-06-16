//! Model probing — discover available models from upstream providers.

/// Model info from upstream probe
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbedModel {
    pub id: String,
    pub name: String,
    pub context_window: u32,
}

/// Probe upstream /models endpoint for available models.
/// Returns empty array on any failure (network, timeout, non-JSON).
pub async fn probe_models_from_upstream(
    base_url: &str,
    api_key: &str,
    timeout_ms: u64,
) -> Vec<ProbedModel> {
    if base_url.is_empty() {
        return vec![];
    }

    let models_url = format!("{}/models", base_url.trim_end_matches('/'));

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut request = client.get(&models_url);
    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = match request.send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    if !resp.status().is_success() {
        return vec![];
    }

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let data = match json.get("data").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return vec![],
    };

    data.iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?;
            if id.is_empty() { return None; }
            Some(ProbedModel {
                id: id.to_string(),
                name: id.to_string(),
                context_window: 0,
            })
        })
        .collect()
}

/// Probe with default 10s timeout
pub async fn probe_models(base_url: &str, api_key: &str) -> Vec<ProbedModel> {
    probe_models_from_upstream(base_url, api_key, 10_000).await
}
