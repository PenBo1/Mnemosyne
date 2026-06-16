//! LLM endpoint authentication utilities.

/// Check if an API key is optional for a given endpoint
pub fn is_api_key_optional_for_endpoint(endpoint: &str) -> bool {
    let lower = endpoint.to_lowercase();
    // Ollama doesn't need an API key
    lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("11434")
}

/// Resolve API key for a provider
pub fn resolve_api_key(provider: &str, env_var: &str) -> String {
    // Check env var first
    if let Ok(key) = std::env::var(env_var) {
        if !key.is_empty() {
            return key;
        }
    }

    // Check provider-specific env vars
    let provider_env = format!("{}_API_KEY", provider.to_uppercase());
    if let Ok(key) = std::env::var(&provider_env) {
        if !key.is_empty() {
            return key;
        }
    }

    String::new()
}
