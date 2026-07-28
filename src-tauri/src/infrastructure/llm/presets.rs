//! ═══════════════════════════════════════════════════════════════════════════
//! Provider 预设表 - OpenAI/Anthropic 兼容 Provider 元数据
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 设计目标:
//! 1. 零新增依赖 —— 所有 OpenAI 兼容 provider 复用 OpenAiProvider,
//!    Anthropic 兼容复用 AnthropicProvider。
//! 2. 二进制体积可控 —— 预设是静态表,只存必要字段。
//! 3. 环境变量自动注册 —— ProviderRegistry::new() 遍历 presets,
//!    有 env var 就自动注册。
//! 4. UI 可枚举 —— 通过 list_provider_presets 命令暴露给前端。
//!
//! 收录 40+ 主流 provider，涵盖国产/海外/聚合器三类。

use super::types::ModelInfo;

/// Provider 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetProtocol {
    /// OpenAI Chat Completions 兼容协议(绝大多数 provider 用这个)
    OpenAi,
    /// Anthropic Messages 协议(百炼 Anthropic 通道 / Claude 官方)
    Anthropic,
}

/// 单个 provider 的预设元数据
#[derive(Debug, Clone)]
pub struct ProviderPreset {
    /// provider id(注册到 registry 的 key,如 "moonshot")
    pub id: &'static str,
    /// 显示名称(给 UI 用,如 "Moonshot (Kimi)")
    pub label: &'static str,
    /// 分组(overseas/china/aggregator)
    pub group: &'static str,
    /// 协议类型
    pub protocol: PresetProtocol,
    /// 默认 base_url(env var 没设 BASE_URL 时用这个)
    pub base_url: &'static str,
    /// 环境变量名(如 "MOONSHOT_API_KEY")
    pub env_var: &'static str,
    /// 环境变量 base_url 名(如 "MOONSHOT_BASE_URL",可选)
    pub env_base_url: &'static str,
    /// 测试连接时用的 model id(便宜/快速的模型)
    pub check_model: &'static str,
    /// 代表性模型列表(不全量,只挑主流)
    pub models: &'static [PresetModel],
}

/// 预设模型定义
#[derive(Debug, Clone)]
pub struct PresetModel {
    pub id: &'static str,
    pub name: &'static str,
    pub context_window: usize,
    pub supports_tools: bool,
}

// 静态模型表 —— 用 const &[] 避免运行时分配。
// 每个 provider 只挑 5-15 个主流模型,避免撑体积。

const MOONSHOT_MODELS: &[PresetModel] = &[
    PresetModel { id: "kimi-k2.6", name: "Kimi K2.6", context_window: 262144, supports_tools: true },
    PresetModel { id: "kimi-k2.5", name: "Kimi K2.5", context_window: 262144, supports_tools: true },
    PresetModel { id: "kimi-k2-thinking", name: "Kimi K2 Thinking", context_window: 262144, supports_tools: true },
    PresetModel { id: "moonshot-v1-auto", name: "Moonshot v1 Auto", context_window: 131072, supports_tools: true },
    PresetModel { id: "moonshot-v1-128k", name: "Moonshot v1 128K", context_window: 131072, supports_tools: true },
];

const ZHIPU_MODELS: &[PresetModel] = &[
    PresetModel { id: "glm-5.1", name: "GLM-5.1", context_window: 200000, supports_tools: true },
    PresetModel { id: "glm-5", name: "GLM-5", context_window: 200000, supports_tools: true },
    PresetModel { id: "glm-4.7", name: "GLM-4.7", context_window: 200000, supports_tools: true },
    PresetModel { id: "glm-4.7-flash", name: "GLM-4.7 Flash", context_window: 200000, supports_tools: true },
    PresetModel { id: "glm-4-flash", name: "GLM-4 Flash", context_window: 131072, supports_tools: true },
    PresetModel { id: "glm-4-long", name: "GLM-4 Long", context_window: 1024000, supports_tools: true },
];

const QWEN_MODELS: &[PresetModel] = &[
    PresetModel { id: "qwen3.6-max-preview", name: "Qwen3.6 Max Preview", context_window: 262144, supports_tools: true },
    PresetModel { id: "qwen3.6-plus", name: "Qwen3.6 Plus", context_window: 1000000, supports_tools: true },
    PresetModel { id: "qwen3.6-flash", name: "Qwen3.6 Flash", context_window: 1000000, supports_tools: true },
    PresetModel { id: "qwen3-max", name: "Qwen3 Max", context_window: 262144, supports_tools: true },
    PresetModel { id: "qwen-plus", name: "Qwen Plus", context_window: 1000000, supports_tools: true },
    PresetModel { id: "qwen-turbo", name: "Qwen Turbo", context_window: 1000000, supports_tools: true },
];

const DEEPSEEK_MODELS: &[PresetModel] = &[
    PresetModel { id: "deepseek-chat", name: "DeepSeek V3", context_window: 64000, supports_tools: true },
    PresetModel { id: "deepseek-reasoner", name: "DeepSeek R1", context_window: 64000, supports_tools: false },
];

const BAICHUAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "Baichuan4-Turbo", name: "Baichuan4 Turbo", context_window: 192000, supports_tools: true },
    PresetModel { id: "Baichuan4-Air", name: "Baichuan4 Air", context_window: 32000, supports_tools: true },
    PresetModel { id: "Baichuan3-Turbo", name: "Baichuan3 Turbo", context_window: 128000, supports_tools: true },
];

const MINIMAX_MODELS: &[PresetModel] = &[
    PresetModel { id: "MiniMax-M2.5", name: "MiniMax M2.5", context_window: 196608, supports_tools: true },
    PresetModel { id: "MiniMax-M2.1", name: "MiniMax M2.1", context_window: 204800, supports_tools: true },
    PresetModel { id: "abab6.5s-chat", name: "ABAB 6.5s", context_window: 245760, supports_tools: true },
];

const YI_MODELS: &[PresetModel] = &[
    PresetModel { id: "yi-large", name: "Yi Large", context_window: 32768, supports_tools: true },
    PresetModel { id: "yi-medium", name: "Yi Medium", context_window: 16384, supports_tools: true },
    PresetModel { id: "yi-lightning", name: "Yi Lightning", context_window: 16384, supports_tools: true },
];

const STEPFUN_MODELS: &[PresetModel] = &[
    PresetModel { id: "step-2-16k", name: "Step 2 16K", context_window: 16384, supports_tools: true },
    PresetModel { id: "step-2-mini", name: "Step 2 Mini", context_window: 32768, supports_tools: true },
    PresetModel { id: "step-1-8k", name: "Step 1 8K", context_window: 8192, supports_tools: true },
];

const SPARK_MODELS: &[PresetModel] = &[
    PresetModel { id: "4.0Ultra", name: "Spark 4.0 Ultra", context_window: 8192, supports_tools: true },
    PresetModel { id: "generalv3.5", name: "Spark 3.5", context_window: 8192, supports_tools: true },
    PresetModel { id: "general", name: "Spark 3", context_window: 8192, supports_tools: true },
];

const HUNYUAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "hunyuan-turbos-latest", name: "Hunyuan Turbo S", context_window: 28000, supports_tools: true },
    PresetModel { id: "hunyuan-large", name: "Hunyuan Large", context_window: 28000, supports_tools: true },
    PresetModel { id: "hunyuan-pro", name: "Hunyuan Pro", context_window: 28000, supports_tools: true },
];

const WENXIN_MODELS: &[PresetModel] = &[
    PresetModel { id: "ernie-4.0-turbo-8k", name: "ERNIE 4.0 Turbo", context_window: 8192, supports_tools: true },
    PresetModel { id: "ernie-4.0-8k-latest", name: "ERNIE 4.0", context_window: 8192, supports_tools: true },
    PresetModel { id: "ernie-3.5-8k", name: "ERNIE 3.5", context_window: 8192, supports_tools: true },
];

const SENSENOVA_MODELS: &[PresetModel] = &[
    PresetModel { id: "SenseChat-5", name: "SenseChat 5", context_window: 32000, supports_tools: true },
    PresetModel { id: "SenseChat-Turbo", name: "SenseChat Turbo", context_window: 32000, supports_tools: true },
];

const VOLCENGINE_MODELS: &[PresetModel] = &[
    PresetModel { id: "doubao-1.5-pro-256k", name: "Doubao 1.5 Pro 256K", context_window: 256000, supports_tools: true },
    PresetModel { id: "doubao-1.5-pro-32k", name: "Doubao 1.5 Pro 32K", context_window: 32000, supports_tools: true },
    PresetModel { id: "doubao-1.5-lite-32k", name: "Doubao 1.5 Lite 32K", context_window: 32000, supports_tools: true },
];

const LONGCAT_MODELS: &[PresetModel] = &[
    PresetModel { id: "longcat-flash", name: "LongCat Flash", context_window: 128000, supports_tools: true },
    PresetModel { id: "longcat-chat", name: "LongCat Chat", context_window: 128000, supports_tools: true },
];

const INTERNLM_MODELS: &[PresetModel] = &[
    PresetModel { id: "internlm2.5-latest", name: "InternLM 2.5", context_window: 32000, supports_tools: true },
];

const XIAOMI_MODELS: &[PresetModel] = &[
    PresetModel { id: "mimo-7b-rl", name: "MiMo 7B RL", context_window: 32768, supports_tools: false },
    PresetModel { id: "mimo-7b-base", name: "MiMo 7B Base", context_window: 32768, supports_tools: false },
];

// --- 聚合器 ---

const SILICONCLOUD_MODELS: &[PresetModel] = &[
    PresetModel { id: "deepseek-ai/DeepSeek-V3", name: "DeepSeek V3 (SiliconCloud)", context_window: 64000, supports_tools: true },
    PresetModel { id: "Qwen/Qwen2.5-72B-Instruct", name: "Qwen2.5 72B (SiliconCloud)", context_window: 131072, supports_tools: true },
    PresetModel { id: "meta-llama/Llama-3.3-70B-Instruct", name: "Llama 3.3 70B (SiliconCloud)", context_window: 131072, supports_tools: true },
];

const OPENROUTER_MODELS: &[PresetModel] = &[
    PresetModel { id: "anthropic/claude-3.5-sonnet", name: "Claude 3.5 Sonnet (OpenRouter)", context_window: 200000, supports_tools: true },
    PresetModel { id: "openai/gpt-4o", name: "GPT-4o (OpenRouter)", context_window: 128000, supports_tools: true },
    PresetModel { id: "google/gemini-2.0-flash", name: "Gemini 2.0 Flash (OpenRouter)", context_window: 1000000, supports_tools: true },
];

const PPIO_MODELS: &[PresetModel] = &[
    PresetModel { id: "deepseek/deepseek-v3", name: "DeepSeek V3 (PPIO)", context_window: 64000, supports_tools: true },
    PresetModel { id: "qwen/qwen2.5-72b-instruct", name: "Qwen2.5 72B (PPIO)", context_window: 131072, supports_tools: true },
];

const QINIU_MODELS: &[PresetModel] = &[
    PresetModel { id: "deepseek-v3", name: "DeepSeek V3 (Qiniu)", context_window: 64000, supports_tools: true },
    PresetModel { id: "qwen2.5-72b-instruct", name: "Qwen2.5 72B (Qiniu)", context_window: 131072, supports_tools: true },
];

const MODELSCOPE_MODELS: &[PresetModel] = &[
    PresetModel { id: "Qwen/Qwen2.5-72B-Instruct", name: "Qwen2.5 72B (ModelScope)", context_window: 131072, supports_tools: true },
    PresetModel { id: "deepseek-ai/DeepSeek-V3", name: "DeepSeek V3 (ModelScope)", context_window: 64000, supports_tools: true },
];

const GITEEAI_MODELS: &[PresetModel] = &[
    PresetModel { id: "DeepSeek/V3", name: "DeepSeek V3 (Gitee AI)", context_window: 64000, supports_tools: true },
    PresetModel { id: "Qwen/Qwen2.5-72B-Instruct", name: "Qwen2.5 72B (Gitee AI)", context_window: 131072, supports_tools: true },
];

const AI360_MODELS: &[PresetModel] = &[
    PresetModel { id: "360gpt2-pro", name: "360GPT2 Pro", context_window: 32768, supports_tools: true },
];

const INFINIAI_MODELS: &[PresetModel] = &[
    PresetModel { id: "gpt-4o", name: "GPT-4o (InfiniAI)", context_window: 128000, supports_tools: true },
    PresetModel { id: "claude-3-5-sonnet", name: "Claude 3.5 Sonnet (InfiniAI)", context_window: 200000, supports_tools: true },
];

const TENCENTCLOUD_MODELS: &[PresetModel] = &[
    PresetModel { id: "deepseek-v3", name: "DeepSeek V3 (Tencent Cloud)", context_window: 64000, supports_tools: true },
    PresetModel { id: "qwen2.5-72b-instruct", name: "Qwen2.5 72B (Tencent Cloud)", context_window: 131072, supports_tools: true },
];

// --- 海外 ---

const MISTRAL_MODELS: &[PresetModel] = &[
    PresetModel { id: "mistral-large-latest", name: "Mistral Large", context_window: 128000, supports_tools: true },
    PresetModel { id: "mistral-small-latest", name: "Mistral Small", context_window: 32000, supports_tools: true },
    PresetModel { id: "codestral-latest", name: "Codestral", context_window: 32000, supports_tools: true },
];

const XAI_MODELS: &[PresetModel] = &[
    PresetModel { id: "grok-3", name: "Grok 3", context_window: 131072, supports_tools: true },
    PresetModel { id: "grok-3-mini", name: "Grok 3 Mini", context_window: 131072, supports_tools: true },
    PresetModel { id: "grok-2-vision", name: "Grok 2 Vision", context_window: 32768, supports_tools: true },
];

const GOOGLE_MODELS: &[PresetModel] = &[
    PresetModel { id: "gemini-2.0-flash", name: "Gemini 2.0 Flash", context_window: 1000000, supports_tools: true },
    PresetModel { id: "gemini-2.0-pro", name: "Gemini 2.0 Pro", context_window: 2000000, supports_tools: true },
    PresetModel { id: "gemini-1.5-pro", name: "Gemini 1.5 Pro", context_window: 2000000, supports_tools: true },
];

const GROQ_MODELS: &[PresetModel] = &[
    PresetModel { id: "llama-3.3-70b-versatile", name: "Llama 3.3 70B (Groq)", context_window: 128000, supports_tools: true },
    PresetModel { id: "llama-3.1-8b-instant", name: "Llama 3.1 8B Instant (Groq)", context_window: 128000, supports_tools: true },
];

const TOGETHER_MODELS: &[PresetModel] = &[
    PresetModel { id: "meta-llama/Llama-3.3-70B-Instruct-Turbo", name: "Llama 3.3 70B (Together)", context_window: 131072, supports_tools: true },
    PresetModel { id: "Qwen/Qwen2.5-72B-Instruct-Turbo", name: "Qwen2.5 72B (Together)", context_window: 131072, supports_tools: true },
];

// --- 聚合器 / 编程订阅包 ---

/// kkaiapi 聚合器(挑 enabled 的代表性模型,排除 image/codex/disabled)
const KKAIAPI_MODELS: &[PresetModel] = &[
    PresetModel { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash (kkaiapi)", context_window: 1000000, supports_tools: true },
    PresetModel { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro (kkaiapi)", context_window: 1000000, supports_tools: true },
    PresetModel { id: "gpt-5.5", name: "GPT-5.5 (kkaiapi)", context_window: 1050000, supports_tools: true },
    PresetModel { id: "gpt-5.4", name: "GPT-5.4 (kkaiapi)", context_window: 1050000, supports_tools: true },
    PresetModel { id: "gpt-5.4-mini", name: "GPT-5.4 Mini (kkaiapi)", context_window: 400000, supports_tools: true },
    PresetModel { id: "claude-opus-4-7", name: "Claude Opus 4.7 (kkaiapi)", context_window: 1000000, supports_tools: true },
    PresetModel { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6 (kkaiapi)", context_window: 1000000, supports_tools: true },
    PresetModel { id: "claude-haiku-4-5", name: "Claude Haiku 4.5 (kkaiapi)", context_window: 200000, supports_tools: true },
    PresetModel { id: "gemini-3.1-pro-preview", name: "Gemini 3.1 Pro Preview (kkaiapi)", context_window: 1048576, supports_tools: true },
    PresetModel { id: "glm-5.1", name: "GLM-5.1 (kkaiapi)", context_window: 128000, supports_tools: true },
    PresetModel { id: "kimi-k2.6", name: "Kimi K2.6 (kkaiapi)", context_window: 256000, supports_tools: true },
    PresetModel { id: "qwen3.6-plus", name: "Qwen3.6 Plus (kkaiapi)", context_window: 128000, supports_tools: true },
    PresetModel { id: "mimo-v2.5-pro", name: "MiMo V2.5 Pro (kkaiapi)", context_window: 128000, supports_tools: true },
];

/// NewAPI 网关 —— 模型列表由用户部署方决定,预设留空
const NEWAPI_MODELS: &[PresetModel] = &[];

/// Kimi Code(Anthropic 协议)
const KIMICODE_MODELS: &[PresetModel] = &[
    PresetModel { id: "kimi-for-coding", name: "Kimi for Coding", context_window: 262144, supports_tools: true },
];

/// Kimi Coding Plan(Anthropic 协议)
const KIMI_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "kimi-k2.5", name: "Kimi K2.5 (Coding Plan)", context_window: 262144, supports_tools: true },
    PresetModel { id: "kimi-k2-thinking", name: "Kimi K2 Thinking (Coding Plan)", context_window: 262144, supports_tools: true },
];

/// MiniMax Coding Plan(Anthropic 协议)
const MINIMAX_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "MiniMax-M2.7", name: "MiniMax M2.7 (Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "MiniMax-M2.7-highspeed", name: "MiniMax M2.7 Highspeed (Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "MiniMax-M2.5", name: "MiniMax M2.5 (Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "MiniMax-M2.5-highspeed", name: "MiniMax M2.5 Highspeed (Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "MiniMax-M2.1", name: "MiniMax M2.1 (Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "MiniMax-M2", name: "MiniMax M2 (Coding Plan)", context_window: 204800, supports_tools: true },
];

/// 百炼 Coding Plan(Anthropic 协议)
const BAILIAN_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "qwen3.5-plus", name: "Qwen3.5 Plus (百炼 Coding Plan)", context_window: 1000000, supports_tools: true },
    PresetModel { id: "qwen3-coder-plus", name: "Qwen3 Coder Plus (百炼 Coding Plan)", context_window: 1000000, supports_tools: true },
    PresetModel { id: "qwen3-max-2026-01-23", name: "Qwen3 Max 2026-01-23 (百炼 Coding Plan)", context_window: 262144, supports_tools: true },
    PresetModel { id: "qwen3-coder-next", name: "Qwen3 Coder Next (百炼 Coding Plan)", context_window: 262144, supports_tools: true },
    PresetModel { id: "glm-5", name: "GLM-5 (百炼 Coding Plan)", context_window: 200000, supports_tools: true },
    PresetModel { id: "glm-4.7", name: "GLM-4.7 (百炼 Coding Plan)", context_window: 200000, supports_tools: true },
    PresetModel { id: "kimi-k2.5", name: "Kimi K2.5 (百炼 Coding Plan)", context_window: 262144, supports_tools: true },
    PresetModel { id: "MiniMax-M2.5", name: "MiniMax M2.5 (百炼 Coding Plan)", context_window: 204800, supports_tools: true },
];

/// GLM Coding Plan(Anthropic 协议)
const GLM_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "GLM-5.1", name: "GLM-5.1 (Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "GLM-5", name: "GLM-5 (Coding Plan)", context_window: 200000, supports_tools: true },
    PresetModel { id: "GLM-5-Turbo", name: "GLM-5 Turbo (Coding Plan)", context_window: 200000, supports_tools: true },
    PresetModel { id: "GLM-4.7", name: "GLM-4.7 (Coding Plan)", context_window: 200000, supports_tools: true },
    PresetModel { id: "GLM-4.6", name: "GLM-4.6 (Coding Plan)", context_window: 202752, supports_tools: true },
    PresetModel { id: "GLM-4.5", name: "GLM-4.5 (Coding Plan)", context_window: 202752, supports_tools: true },
    PresetModel { id: "GLM-4.5-Air", name: "GLM-4.5 Air (Coding Plan)", context_window: 202752, supports_tools: true },
];

/// 火山方舟 Coding Plan(Anthropic 协议)
const VOLCENGINE_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "doubao-seed-2.0-code", name: "Doubao Seed 2.0 Code (Coding Plan)", context_window: 256000, supports_tools: true },
    PresetModel { id: "doubao-seed-2.0-pro", name: "Doubao Seed 2.0 Pro (Coding Plan)", context_window: 256000, supports_tools: true },
    PresetModel { id: "doubao-seed-2.0-lite", name: "Doubao Seed 2.0 Lite (Coding Plan)", context_window: 256000, supports_tools: true },
    PresetModel { id: "doubao-seed-code", name: "Doubao Seed Code (Coding Plan)", context_window: 256000, supports_tools: true },
    PresetModel { id: "minimax-m2.5", name: "MiniMax M2.5 (火山 Coding Plan)", context_window: 204800, supports_tools: true },
    PresetModel { id: "glm-4.7", name: "GLM-4.7 (火山 Coding Plan)", context_window: 200000, supports_tools: true },
    PresetModel { id: "deepseek-v3.2", name: "DeepSeek V3.2 (火山 Coding Plan)", context_window: 262144, supports_tools: true },
    PresetModel { id: "kimi-k2.5", name: "Kimi K2.5 (火山 Coding Plan)", context_window: 262144, supports_tools: true },
];

/// OpenCode Coding Plan(Anthropic 协议)
const OPENCODE_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "glm-5.1", name: "GLM-5.1 (OpenCode)", context_window: 204800, supports_tools: true },
    PresetModel { id: "glm-5", name: "GLM-5 (OpenCode)", context_window: 204800, supports_tools: true },
    PresetModel { id: "kimi-k2.5", name: "Kimi K2.5 (OpenCode)", context_window: 262144, supports_tools: true },
    PresetModel { id: "mimo-v2-omni", name: "MiMo V2 Omni (OpenCode)", context_window: 262144, supports_tools: true },
    PresetModel { id: "qwen3.6-plus", name: "Qwen3.6 Plus (OpenCode)", context_window: 262144, supports_tools: true },
    PresetModel { id: "minimax-m2.5", name: "MiniMax M2.5 (OpenCode)", context_window: 204800, supports_tools: true },
    PresetModel { id: "minimax-m2.7", name: "MiniMax M2.7 (OpenCode)", context_window: 204800, supports_tools: true },
    PresetModel { id: "mimo-v2-pro", name: "MiMo V2 Pro (OpenCode)", context_window: 1048576, supports_tools: true },
    PresetModel { id: "qwen3.5-plus", name: "Qwen3.5 Plus (OpenCode)", context_window: 262144, supports_tools: true },
];

/// 讯飞星辰 Astron Coding Plan(Anthropic 协议)
const ASTRON_CODING_PLAN_MODELS: &[PresetModel] = &[
    PresetModel { id: "astron-code-latest", name: "Astron Code Latest", context_window: 131072, supports_tools: true },
];

// --- 预设主表 ---

pub const PRESETS: &[ProviderPreset] = &[
    // === 国产 ===
    ProviderPreset {
        id: "moonshot",
        label: "Moonshot (Kimi)",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.moonshot.cn/v1",
        env_var: "MOONSHOT_API_KEY",
        env_base_url: "MOONSHOT_BASE_URL",
        check_model: "moonshot-v1-8k",
        models: MOONSHOT_MODELS,
    },
    ProviderPreset {
        id: "zhipu",
        label: "智谱 GLM",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        env_var: "ZHIPU_API_KEY",
        env_base_url: "ZHIPU_BASE_URL",
        check_model: "glm-4-flash",
        models: ZHIPU_MODELS,
    },
    ProviderPreset {
        id: "qwen",
        label: "通义千问 (DashScope)",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        env_var: "QWEN_API_KEY",
        env_base_url: "QWEN_BASE_URL",
        check_model: "qwen-turbo",
        models: QWEN_MODELS,
    },
    ProviderPreset {
        id: "deepseek",
        label: "DeepSeek",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.deepseek.com",
        env_var: "DEEPSEEK_API_KEY",
        env_base_url: "DEEPSEEK_BASE_URL",
        check_model: "deepseek-chat",
        models: DEEPSEEK_MODELS,
    },
    ProviderPreset {
        id: "baichuan",
        label: "百川 Baichuan",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.baichuan-ai.com/v1",
        env_var: "BAICHUAN_API_KEY",
        env_base_url: "BAICHUAN_BASE_URL",
        check_model: "Baichuan4-Turbo",
        models: BAICHUAN_MODELS,
    },
    ProviderPreset {
        id: "minimax",
        label: "MiniMax",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.minimax.chat/v1",
        env_var: "MINIMAX_API_KEY",
        env_base_url: "MINIMAX_BASE_URL",
        check_model: "abab6.5s-chat",
        models: MINIMAX_MODELS,
    },
    ProviderPreset {
        id: "yi",
        label: "零一万物 Yi",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.lingyiwanwu.com/v1",
        env_var: "YI_API_KEY",
        env_base_url: "YI_BASE_URL",
        check_model: "yi-lightning",
        models: YI_MODELS,
    },
    ProviderPreset {
        id: "stepfun",
        label: "阶跃星辰 StepFun",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.stepfun.com/v1",
        env_var: "STEPFUN_API_KEY",
        env_base_url: "STEPFUN_BASE_URL",
        check_model: "step-1-8k",
        models: STEPFUN_MODELS,
    },
    ProviderPreset {
        id: "spark",
        label: "讯飞星火 Spark",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://spark-api-open.xf-yun.com/v1",
        env_var: "SPARK_API_KEY",
        env_base_url: "SPARK_BASE_URL",
        check_model: "generalv3.5",
        models: SPARK_MODELS,
    },
    ProviderPreset {
        id: "hunyuan",
        label: "腾讯混元 Hunyuan",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.hunyuan.cloud.tencent.com/v1",
        env_var: "HUNYUAN_API_KEY",
        env_base_url: "HUNYUAN_BASE_URL",
        check_model: "hunyuan-pro",
        models: HUNYUAN_MODELS,
    },
    ProviderPreset {
        id: "wenxin",
        label: "百度文心 ERNIE",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://qianfan.baidubce.com/v2",
        env_var: "WENXIN_API_KEY",
        env_base_url: "WENXIN_BASE_URL",
        check_model: "ernie-3.5-8k",
        models: WENXIN_MODELS,
    },
    ProviderPreset {
        id: "sensenova",
        label: "商汤 SenseNova",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.sensenova.cn/compatible-mode/v1",
        env_var: "SENSENOVA_API_KEY",
        env_base_url: "SENSENOVA_BASE_URL",
        check_model: "SenseChat-Turbo",
        models: SENSENOVA_MODELS,
    },
    ProviderPreset {
        id: "volcengine",
        label: "火山豆包 Doubao",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        env_var: "VOLCENGINE_API_KEY",
        env_base_url: "VOLCENGINE_BASE_URL",
        check_model: "doubao-1.5-lite-32k",
        models: VOLCENGINE_MODELS,
    },
    ProviderPreset {
        id: "longcat",
        label: "美团 LongCat",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.longcat.chat/openai/v1",
        env_var: "LONGCAT_API_KEY",
        env_base_url: "LONGCAT_BASE_URL",
        check_model: "longcat-flash",
        models: LONGCAT_MODELS,
    },
    ProviderPreset {
        id: "internlm",
        label: "书生 InternLM",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://internlm-chat.intern-ai.org.cn/puyu/api/v1",
        env_var: "INTERNLM_API_KEY",
        env_base_url: "INTERNLM_BASE_URL",
        check_model: "internlm2.5-latest",
        models: INTERNLM_MODELS,
    },
    ProviderPreset {
        id: "xiaomi-mimo",
        label: "小米 MiMo",
        group: "china",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.xiaoaiapi.com/v1",
        env_var: "XIAOMI_API_KEY",
        env_base_url: "XIAOMI_BASE_URL",
        check_model: "mimo-7b-rl",
        models: XIAOMI_MODELS,
    },
    // === 聚合器 ===
    ProviderPreset {
        id: "siliconcloud",
        label: "SiliconCloud 硅基流动",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.siliconflow.cn/v1",
        env_var: "SILICONCLOUD_API_KEY",
        env_base_url: "SILICONCLOUD_BASE_URL",
        check_model: "Qwen/Qwen2.5-72B-Instruct",
        models: SILICONCLOUD_MODELS,
    },
    ProviderPreset {
        id: "openrouter",
        label: "OpenRouter",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://openrouter.ai/api/v1",
        env_var: "OPENROUTER_API_KEY",
        env_base_url: "OPENROUTER_BASE_URL",
        check_model: "openai/gpt-4o",
        models: OPENROUTER_MODELS,
    },
    ProviderPreset {
        id: "ppio",
        label: "PPIO 派欧云",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.ppinfra.com/v3/openai",
        env_var: "PPIO_API_KEY",
        env_base_url: "PPIO_BASE_URL",
        check_model: "deepseek/deepseek-v3",
        models: PPIO_MODELS,
    },
    ProviderPreset {
        id: "qiniu",
        label: "七牛云",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.qnaigc.com/v1",
        env_var: "QINIU_API_KEY",
        env_base_url: "QINIU_BASE_URL",
        check_model: "deepseek-v3",
        models: QINIU_MODELS,
    },
    ProviderPreset {
        id: "modelscope",
        label: "ModelScope 魔搭",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api-inference.modelscope.cn/v1",
        env_var: "MODELSCOPE_API_KEY",
        env_base_url: "MODELSCOPE_BASE_URL",
        check_model: "Qwen/Qwen2.5-72B-Instruct",
        models: MODELSCOPE_MODELS,
    },
    ProviderPreset {
        id: "giteeai",
        label: "Gitee AI",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://ai.gitee.com/serverless-api/v1",
        env_var: "GITEEAI_API_KEY",
        env_base_url: "GITEEAI_BASE_URL",
        check_model: "Qwen/Qwen2.5-72B-Instruct",
        models: GITEEAI_MODELS,
    },
    ProviderPreset {
        id: "ai360",
        label: "360 智脑",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.360.cn/v1",
        env_var: "AI360_API_KEY",
        env_base_url: "AI360_BASE_URL",
        check_model: "360gpt2-pro",
        models: AI360_MODELS,
    },
    ProviderPreset {
        id: "infiniai",
        label: "InfiniAI",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://cloud.infini-ai.com/maas/v1",
        env_var: "INFINIAI_API_KEY",
        env_base_url: "INFINIAI_BASE_URL",
        check_model: "gpt-4o",
        models: INFINIAI_MODELS,
    },
    ProviderPreset {
        id: "tencentcloud",
        label: "腾讯云 LKE",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.lkeap.cloud.tencent.com/v1",
        env_var: "TENCENTCLOUD_API_KEY",
        env_base_url: "TENCENTCLOUD_BASE_URL",
        check_model: "deepseek-v3",
        models: TENCENTCLOUD_MODELS,
    },
    // === 海外 ===
    ProviderPreset {
        id: "mistral",
        label: "Mistral AI",
        group: "overseas",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.mistral.ai/v1",
        env_var: "MISTRAL_API_KEY",
        env_base_url: "MISTRAL_BASE_URL",
        check_model: "mistral-small-latest",
        models: MISTRAL_MODELS,
    },
    ProviderPreset {
        id: "xai",
        label: "xAI Grok",
        group: "overseas",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.x.ai/v1",
        env_var: "XAI_API_KEY",
        env_base_url: "XAI_BASE_URL",
        check_model: "grok-3-mini",
        models: XAI_MODELS,
    },
    ProviderPreset {
        id: "google",
        label: "Google Gemini (OpenAI 兼容)",
        group: "overseas",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        env_var: "GOOGLE_API_KEY",
        env_base_url: "GOOGLE_BASE_URL",
        check_model: "gemini-1.5-pro",
        models: GOOGLE_MODELS,
    },
    ProviderPreset {
        id: "groq",
        label: "Groq",
        group: "overseas",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.groq.com/openai/v1",
        env_var: "GROQ_API_KEY",
        env_base_url: "GROQ_BASE_URL",
        check_model: "llama-3.1-8b-instant",
        models: GROQ_MODELS,
    },
    ProviderPreset {
        id: "together",
        label: "Together AI",
        group: "overseas",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.together.xyz/v1",
        env_var: "TOGETHER_API_KEY",
        env_base_url: "TOGETHER_BASE_URL",
        check_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        models: TOGETHER_MODELS,
    },
    // === 编程订阅包(CodingPlan,Anthropic 协议) ===
    ProviderPreset {
        id: "kimicode",
        label: "Kimi Code",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://api.kimi.com/coding",
        env_var: "KIMI_CODE_API_KEY",
        env_base_url: "KIMI_CODE_BASE_URL",
        check_model: "kimi-for-coding",
        models: KIMICODE_MODELS,
    },
    ProviderPreset {
        id: "kimiCodingPlan",
        label: "Kimi Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://api.moonshot.cn/anthropic",
        env_var: "KIMI_CODING_PLAN_API_KEY",
        env_base_url: "KIMI_CODING_PLAN_BASE_URL",
        check_model: "kimi-k2.5",
        models: KIMI_CODING_PLAN_MODELS,
    },
    ProviderPreset {
        id: "minimaxCodingPlan",
        label: "MiniMax Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://api.minimaxi.com/anthropic",
        env_var: "MINIMAX_CODING_PLAN_API_KEY",
        env_base_url: "MINIMAX_CODING_PLAN_BASE_URL",
        check_model: "MiniMax-M2.7",
        models: MINIMAX_CODING_PLAN_MODELS,
    },
    ProviderPreset {
        id: "bailianCodingPlan",
        label: "百炼 Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://dashscope.aliyuncs.com/apps/anthropic",
        env_var: "BAILIAN_CODING_PLAN_API_KEY",
        env_base_url: "BAILIAN_CODING_PLAN_BASE_URL",
        check_model: "qwen-max",
        models: BAILIAN_CODING_PLAN_MODELS,
    },
    ProviderPreset {
        id: "glmCodingPlan",
        label: "GLM Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://api.z.ai/api/anthropic",
        env_var: "GLM_CODING_PLAN_API_KEY",
        env_base_url: "GLM_CODING_PLAN_BASE_URL",
        check_model: "glm-5.1",
        models: GLM_CODING_PLAN_MODELS,
    },
    ProviderPreset {
        id: "volcengineCodingPlan",
        label: "火山 Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://ark.cn-beijing.volces.com/api/coding",
        env_var: "VOLCENGINE_CODING_PLAN_API_KEY",
        env_base_url: "VOLCENGINE_CODING_PLAN_BASE_URL",
        check_model: "doubao-seed-2.0-code",
        models: VOLCENGINE_CODING_PLAN_MODELS,
    },
    ProviderPreset {
        id: "opencodeCodingPlan",
        label: "OpenCode Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://opencode.ai/api/anthropic",
        env_var: "OPENCODE_CODING_PLAN_API_KEY",
        env_base_url: "OPENCODE_CODING_PLAN_BASE_URL",
        check_model: "glm-5.1",
        models: OPENCODE_CODING_PLAN_MODELS,
    },
    ProviderPreset {
        id: "astronCodingPlan",
        label: "讯飞星辰 Astron Coding Plan",
        group: "codingPlan",
        protocol: PresetProtocol::Anthropic,
        base_url: "https://maas-coding-api.cn-huabei-1.xf-yun.com/anthropic",
        env_var: "ASTRON_CODING_PLAN_API_KEY",
        env_base_url: "ASTRON_CODING_PLAN_BASE_URL",
        check_model: "astron-code-latest",
        models: ASTRON_CODING_PLAN_MODELS,
    },
    // === 聚合器补充 ===
    ProviderPreset {
        id: "kkaiapi",
        label: "kkaiapi",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "https://api.kkaiapi.com/v1",
        env_var: "KKAIAPI_API_KEY",
        env_base_url: "KKAIAPI_BASE_URL",
        check_model: "deepseek-v4-flash",
        models: KKAIAPI_MODELS,
    },
    ProviderPreset {
        id: "newapi",
        label: "New API (中转网关)",
        group: "aggregator",
        protocol: PresetProtocol::OpenAi,
        base_url: "",
        env_var: "NEWAPI_API_KEY",
        env_base_url: "NEWAPI_BASE_URL",
        check_model: "",
        models: NEWAPI_MODELS,
    },
];

impl ProviderPreset {
    /// 按 id 查找预设
    pub fn find(id: &str) -> Option<&'static ProviderPreset> {
        PRESETS.iter().find(|p| p.id == id)
    }

    /// 将预设模型列表转为 ModelInfo
    pub fn to_model_infos(&self) -> Vec<ModelInfo> {
        self.models.iter().map(|m| ModelInfo {
            id: m.id.to_string(),
            provider: self.id.to_string(),
            name: m.name.to_string(),
            context_window: m.context_window,
            supports_tools: m.supports_tools,
            supports_streaming: true,
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_non_empty() {
        assert!(PRESETS.len() >= 40, "应至少有 40 个预设,实际: {}", PRESETS.len());
    }

    #[test]
    fn preset_ids_unique() {
        let mut ids: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
        ids.sort();
        let dupes: Vec<&str> = ids.windows(2).filter(|w| w[0] == w[1]).map(|w| w[0]).collect();
        assert!(dupes.is_empty(), "重复的 preset id: {:?}", dupes);
    }

    #[test]
    fn find_returns_known_preset() {
        let moonshot = ProviderPreset::find("moonshot").expect("moonshot 应存在");
        assert_eq!(moonshot.label, "Moonshot (Kimi)");
        assert_eq!(moonshot.protocol, PresetProtocol::OpenAi);
        assert!(!moonshot.models.is_empty());
    }

    #[test]
    fn find_returns_none_for_unknown() {
        assert!(ProviderPreset::find("nonexistent-provider").is_none());
    }

    #[test]
    fn to_model_infos_preserves_provider() {
        let zhipu = ProviderPreset::find("zhipu").unwrap();
        let infos = zhipu.to_model_infos();
        assert!(infos.iter().all(|m| m.provider == "zhipu"));
        assert!(!infos.is_empty());
    }

    #[test]
    fn deepseek_preset_matches_existing_registry_id() {
        // 现有 registry.rs 已经把 deepseek 写死,预设里 id 必须一致
        let deepseek = ProviderPreset::find("deepseek").expect("deepseek 预设应存在");
        assert_eq!(deepseek.base_url, "https://api.deepseek.com");
    }

    #[test]
    fn coding_plan_presets_use_anthropic_protocol() {
        // 8 个 CodingPlan 预设全部使用 Anthropic 协议
        for id in [
            "kimicode",
            "kimiCodingPlan",
            "minimaxCodingPlan",
            "bailianCodingPlan",
            "glmCodingPlan",
            "volcengineCodingPlan",
            "opencodeCodingPlan",
            "astronCodingPlan",
        ] {
            let p = ProviderPreset::find(id).unwrap_or_else(|| panic!("{id} 预设应存在"));
            assert_eq!(p.protocol, PresetProtocol::Anthropic, "{id} 应为 Anthropic 协议");
            assert_eq!(p.group, "codingPlan", "{id} group 应为 codingPlan");
            assert!(!p.base_url.is_empty(), "{id} base_url 不应为空");
        }
    }

    #[test]
    fn kkaiapi_preset_has_representative_models() {
        let p = ProviderPreset::find("kkaiapi").expect("kkaiapi 预设应存在");
        assert_eq!(p.protocol, PresetProtocol::OpenAi);
        assert!(!p.models.is_empty(), "kkaiapi 应有代表性模型");
        // 全部模型都应支持工具调用(聚合器走 OpenAI 协议)
        assert!(p.models.iter().all(|m| m.supports_tools));
    }

    #[test]
    fn newapi_preset_has_empty_models_and_base_url() {
        // newapi 是用户自建网关,预设不预填模型和 baseUrl
        let p = ProviderPreset::find("newapi").expect("newapi 预设应存在");
        assert_eq!(p.protocol, PresetProtocol::OpenAi);
        assert!(p.models.is_empty(), "newapi 模型列表应为空");
        assert!(p.base_url.is_empty(), "newapi base_url 应为空");
        assert!(p.check_model.is_empty(), "newapi check_model 应为空");
    }
}
