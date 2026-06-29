//! LLM 端点认证工具。

/// 检查给定端点是否可以不提供 API key
pub fn is_api_key_optional_for_endpoint(endpoint: &str) -> bool {
    let lower = endpoint.to_lowercase();
    // Ollama 不需要 API key
    lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("11434")
}

/// 为 provider 解析 API key
pub fn resolve_api_key(provider: &str, env_var: &str) -> String {
    // 优先检查指定环境变量
    if let Ok(key) = std::env::var(env_var) {
        if !key.is_empty() {
            return key;
        }
    }

    // 再检查 provider 专属环境变量
    let provider_env = format!("{}_API_KEY", provider.to_uppercase());
    if let Ok(key) = std::env::var(&provider_env) {
        if !key.is_empty() {
            return key;
        }
    }

    String::new()
}
