//! LLM 环境变量管理。

use std::collections::HashMap;

/// 加载 LLM 配置相关的环境变量
pub fn load_llm_env() -> HashMap<String, String> {
    let mut env = HashMap::new();

    // 检查常见的 LLM 环境变量
    for key in &[
        "OPENAI_API_KEY", "OPENAI_BASE_URL",
        "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL",
        "DEEPSEEK_API_KEY",
        "MOONSHOT_API_KEY",
        "AGNES_API_KEY", "AGNES_BASE_URL",
        "TAVILY_API_KEY",
        "INKOS_AGENT_ALLOW_SYSTEM_READ",
        "LOG_FORMAT", "LOG_LEVEL",
    ] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                env.insert(key.to_string(), value);
            }
        }
    }

    env
}

/// 获取某个特定的 LLM 环境变量
pub fn get_llm_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// 合并多个环境变量 map
pub fn merge_env_maps(base: &HashMap<String, String>, overlay: &HashMap<String, String>) -> HashMap<String, String> {
    let mut merged = base.clone();
    for (k, v) in overlay {
        merged.insert(k.clone(), v.clone());
    }
    merged
}
