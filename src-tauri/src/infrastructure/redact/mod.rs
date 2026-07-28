//! ═══════════════════════════════════════════════════════════════════════════
//! 密钥脱敏 - 日志、工具输出、IPC 响应的统一脱敏
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 正则密钥脱敏 —— 日志、工具输出、IPC 响应的统一脱敏入口。
//!
//! 特性：
//! - 默认开启，启动时快照，防止运行时被关闭
//! - 短 token (<18 字符) 全掩码，长 token 保留首 6 + 末 4
//! - force=true 用于"绝不可返回原始 secret"的安全边界
//!
//! 支持的脱敏模式：
//! - 厂商前缀（OpenAI/GitHub/Slack/AWS/Stripe/Google/JWT 等）
//! - ENV 赋值（KEY=value，KEY 含 API_KEY/TOKEN/SECRET/PASSWORD/AUTH）
//! - JSON 字段（"apiKey": "..."、"token": "..." 等）
//! - Authorization: Bearer xxx
//! - 私钥块（-----BEGIN ... PRIVATE KEY-----）
//! - 数据库连接串（postgres://user:pass@host）
//! - URL userinfo（https://user:pass@host）
//! - Telegram bot token（<digits>:<token>）
//! - JWT（eyJ... 三段式）

use std::sync::OnceLock;

use regex::Regex;

/// 敏感 ENV 变量名片段(大小写不敏感)
const SECRET_ENV_NAMES: &str = r"(?:API_?KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL|AUTH)";

/// 敏感 JSON 字段名(大小写不敏感,精确匹配)
const SENSITIVE_JSON_KEYS: &str = r#"(?:api_?[Kk]ey|token|secret|password|access_token|refresh_token|auth_token|bearer|secret_value|raw_secret|secret_input|key_material)"#;

/// 预编译的全部模式
struct Patterns {
    /// 厂商前缀 + JWT 合并(jwt 与 prefix 均用 mask_secret(&caps[0]),
    /// 互斥不重叠,合并为单次扫描减少 replace_all 分配)
    token: Regex,
    /// ENV 赋值
    env_assign: Regex,
    /// JSON 字段
    json_field: Regex,
    /// Authorization 头
    auth_header: Regex,
    /// 私钥块
    private_key: Regex,
    /// 数据库连接串密码
    db_connstr: Regex,
    /// URL userinfo
    url_userinfo: Regex,
    /// Telegram bot token
    telegram: Regex,
}

fn compile_patterns() -> Patterns {
    // 厂商前缀列表
    let prefixes = [
        r"sk-[A-Za-z0-9_-]{10,}",              // OpenAI / OpenRouter / Anthropic (sk-ant-*)
        r"ghp_[A-Za-z0-9]{10,}",               // GitHub PAT (classic)
        r"github_pat_[A-Za-z0-9_]{10,}",       // GitHub PAT (fine-grained)
        r"gho_[A-Za-z0-9]{10,}",               // GitHub OAuth access token
        r"ghu_[A-Za-z0-9]{10,}",               // GitHub user-to-server token
        r"ghs_[A-Za-z0-9]{10,}",               // GitHub server-to-server token
        r"ghr_[A-Za-z0-9]{10,}",               // GitHub refresh token
        r"xox[baprs]-[A-Za-z0-9-]{10,}",       // Slack tokens
        r"AIza[A-Za-z0-9_-]{30,}",             // Google API keys
        r"pplx-[A-Za-z0-9]{10,}",             // Perplexity
        r"fal_[A-Za-z0-9_-]{10,}",             // Fal.ai
        r"fc-[A-Za-z0-9]{10,}",                // Firecrawl
        r"bb_live_[A-Za-z0-9_-]{10,}",         // BrowserBase
        r"gAAAA[A-Za-z0-9_=-]{20,}",           // Encrypted tokens
        r"AKIA[A-Z0-9]{16}",                   // AWS Access Key ID
        r"sk_live_[A-Za-z0-9]{10,}",           // Stripe secret key (live)
        r"sk_test_[A-Za-z0-9]{10,}",           // Stripe secret key (test)
        r"rk_live_[A-Za-z0-9]{10,}",           // Stripe restricted key
        r"SG\.[A-Za-z0-9_-]{10,}",             // SendGrid API key
        r"hf_[A-Za-z0-9]{10,}",                // HuggingFace token
        r"r8_[A-Za-z0-9]{10,}",                // Replicate API token
        r"npm_[A-Za-z0-9]{10,}",               // npm access token
        r"pypi-[A-Za-z0-9_-]{10,}",            // PyPI API token
        r"dop_v1_[A-Za-z0-9]{10,}",            // DigitalOcean PAT
        r"doo_v1_[A-Za-z0-9]{10,}",            // DigitalOcean OAuth
        r"am_[A-Za-z0-9_-]{10,}",              // AgentMail API key
        r"sk_[A-Za-z0-9_]{10,}",               // ElevenLabs TTS key
        r"tvly-[A-Za-z0-9]{10,}",              // Tavily search API key
        r"exa_[A-Za-z0-9]{10,}",               // Exa search API key
        r"gsk_[A-Za-z0-9]{10,}",               // Groq Cloud API key
        r"syt_[A-Za-z0-9]{10,}",               // Matrix access token
        r"hsk-[A-Za-z0-9]{10,}",               // Hindsight API key
        r"mem0_[A-Za-z0-9]{10,}",              // Mem0 Platform API key
        r"brv_[A-Za-z0-9]{10,}",               // ByteRover API key
        r"xai-[A-Za-z0-9]{30,}",               // xAI (Grok) API key
        r"ntn_[A-Za-z0-9]{10,}",               // Notion internal integration token
    ];
    let prefix_alt = prefixes
        .iter()
        .map(|p| format!("(?:{})", p))
        .collect::<Vec<_>>()
        .join("|");
    // 不使用 look-around(Rust regex crate 不支持);
    // 前缀模式本身已足够特异(sk-/ghp_/AKIA 等),误报率低。

    // JWT + 厂商前缀合并为单 alternation(两者均用 mask_secret(&caps[0]),
    // 且互斥:eyJ 与 sk-/ghp_/AKIA 等不会同时匹配同一子串)
    let jwt_pat = r"eyJ[A-Za-z0-9_-]{10,}(?:\.[A-Za-z0-9_=-]{4,}){0,2}";
    let token_alt = format!("(?:{})|(?:{})", jwt_pat, prefix_alt);
    let token_re = Regex::new(&token_alt).expect("invalid token regex");

    // ENV 赋值:KEY=value(value 是非空白字符序列,可能含引号)
    // 不使用反向引用(Rust regex crate 不支持 \1/\2),
    // 简化为捕获整个非空白 value 并掩码。
    let env_assign_re = Regex::new(&format!(
        r"([A-Z0-9_]{{0,50}}{}[A-Z0-9_]{{0,50}})\s*=\s*(\S+)",
        SECRET_ENV_NAMES
    ))
    .expect("invalid env_assign regex");

    let json_field_re = Regex::new(&format!(
        r#""((?:{}))"\s*:\s*"([^"]+)""#,
        SENSITIVE_JSON_KEYS
    ))
    .expect("invalid json_field regex");

    Patterns {
        token: token_re,
        env_assign: env_assign_re,
        json_field: json_field_re,
        auth_header: Regex::new(r"(?i)(authorization:\s*bearer\s+)(\S+)")
            .expect("invalid auth_header regex"),
        private_key: Regex::new(
            r"-----BEGIN[A-Z ]*PRIVATE KEY-----[\s\S]*?-----END[A-Z ]*PRIVATE KEY-----",
        )
        .expect("invalid private_key regex"),
        db_connstr: Regex::new(
            r"(?i)((?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqp)://[^:]+:)([^@]+)(@)",
        )
        .expect("invalid db_connstr regex"),
        url_userinfo: Regex::new(r"(?i)((?:https?|wss?|ftp)://)([^/\s:@]+):([^/\s@]+)@")
            .expect("invalid url_userinfo regex"),
        telegram: Regex::new(r"(bot)?(\d{8,}):([-A-Za-z0-9_]{30,})")
            .expect("invalid telegram regex"),
    }
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(compile_patterns)
}

/// 掩码单个 secret 值。
///
/// - 空值返回 "empty"
/// - 长度 < 8 全掩码 "***"
/// - 长度 8..=18 保留首 2 + 末 2,中间 `***`
/// - 长度 > 18 保留首 6 + 末 4,中间 `***`
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "empty".to_string();
    }
    let len = value.chars().count();
    if len < 8 {
        return "***".to_string();
    }
    let (head, tail) = if len <= 18 {
        (2, 2)
    } else {
        (6, 4)
    };
    let chars: Vec<char> = value.chars().collect();
    let head_str: String = chars[..head].iter().collect();
    let tail_str: String = chars[len - tail..].iter().collect();
    format!("{}***{}", head_str, tail_str)
}

/// 对文本执行全量脱敏(默认场景)。
///
/// 顺序:私钥块 → 厂商前缀/JWT(合并) → Authorization 头 →
///       DB 连接串 → URL userinfo → Telegram → ENV 赋值 → JSON 字段
///
/// 后续模式不会破坏前序模式的输出(因为 mask_secret 输出含 `***`,
/// 不会被后续模式误识别为 token)。
pub fn redact_text(text: &str) -> String {
    let p = patterns();

    // 1. 私钥块(整体替换)
    let s = p.private_key.replace_all(text, "[REDACTED PRIVATE KEY]");

    // 2. JWT + 厂商前缀(合并为单次扫描,两者均用 mask_secret(&caps[0]),
    //    且互斥不重叠,减少一次 replace_all 分配)
    let s = p.token.replace_all(&s, |caps: &regex::Captures| {
        mask_secret(&caps[0])
    });

    // 3. Authorization: Bearer xxx
    let s = p.auth_header.replace_all(&s, |caps: &regex::Captures| {
        format!("{}{}", &caps[1], mask_secret(&caps[2]))
    });

    // 4. DB 连接串 postgres://user:PASSWORD@host
    let s = p.db_connstr.replace_all(&s, |caps: &regex::Captures| {
        format!("{}{}{}", &caps[1], mask_secret(&caps[2]), &caps[3])
    });

    // 5. URL userinfo https://user:pass@host
    let s = p.url_userinfo.replace_all(&s, |caps: &regex::Captures| {
        format!("{}{}:***@", &caps[1], &caps[2])
    });

    // 6. Telegram bot token
    let s = p.telegram.replace_all(&s, |caps: &regex::Captures| {
        format!(
            "{}{}:***",
            caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            &caps[2]
        )
    });

    // 7. ENV 赋值 KEY=value(新正则只有两组:KEY 和 value)
    let s = p.env_assign.replace_all(&s, |caps: &regex::Captures| {
        format!("{}={}", &caps[1], mask_secret(&caps[2]))
    });

    // 8. JSON 字段 "apiKey": "value"(新正则两组:key 名和 value)
    let s = p.json_field.replace_all(&s, |caps: &regex::Captures| {
        format!("\"{}\": \"{}\"", &caps[1], mask_secret(&caps[2]))
    });

    s.into_owned()
}

/// 强制脱敏:无视全局关闭偏好。
///
/// 当前实现中 redact_text 始终开启,
/// 此函数保留作为未来"用户可选关闭"时的强制边界入口。
pub fn redact_force(text: &str) -> String {
    redact_text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_short_returns_stars() {
        assert_eq!(mask_secret("abc"), "***");
        assert_eq!(mask_secret(""), "empty");
    }

    #[test]
    fn mask_secret_medium_preserves_2_2() {
        // 长度 8..=18:首 2 + 末 2
        let v = "abcdefghijklmnop"; // 16 chars
        let masked = mask_secret(v);
        assert!(masked.starts_with("ab"));
        assert!(masked.ends_with("op"));
        assert!(masked.contains("***"));
    }

    #[test]
    fn mask_secret_long_preserves_6_4() {
        // 长度 > 18:首 6 + 末 4
        let v = "sk-ant-api03-verylongtokenvaluewithmorethan18characters";
        let masked = mask_secret(v);
        assert!(masked.starts_with("sk-ant"));
        assert!(masked.ends_with("ters"));
        assert!(masked.contains("***"));
    }

    #[test]
    fn redact_openai_key() {
        let input = "OPENAI_API_KEY=sk-1234567890abcdefghijklmnopqrstuvwxyz";
        let out = redact_text(input);
        assert!(out.contains("***"));
        assert!(!out.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn redact_github_pat() {
        let input = "token ghp_1234567890abcdefghij";
        let out = redact_text(input);
        assert!(out.contains("***"));
        // mask_secret 保留首 6 + 末 4,所以 ghp_12 和 ghij 会出现,
        // 但中间部分 34567890abcdef 不应出现
        assert!(!out.contains("34567890abcdef"));
    }

    #[test]
    fn redact_authorization_bearer() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature";
        let out = redact_text(input);
        // Bearer 部分被掩码
        assert!(out.contains("Authorization: Bearer"));
        assert!(out.contains("***"));
        // 原 token 不应完整出现
        assert!(!out.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature"));
    }

    #[test]
    fn redact_private_key_block() {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let out = redact_text(input);
        assert!(out.contains("[REDACTED PRIVATE KEY]"));
        assert!(!out.contains("MIIEpAIBAAKCAQEA"));
    }

    #[test]
    fn redact_postgres_connstr() {
        let input = "postgres://user:secretpassword@localhost:5432/db";
        let out = redact_text(input);
        assert!(out.starts_with("postgres://user:"));
        assert!(out.contains("***"));
        assert!(out.ends_with("@localhost:5432/db"));
        assert!(!out.contains("secretpassword"));
    }

    #[test]
    fn redact_url_userinfo() {
        let input = "https://alice:secret@example.com/path";
        let out = redact_text(input);
        assert!(out.starts_with("https://alice:"));
        assert!(out.contains("***@example.com/path"));
        assert!(!out.contains(":secret@"));
    }

    #[test]
    fn redact_json_field() {
        let input = r#"{"apiKey": "sk-1234567890abcdefghij", "name": "test"}"#;
        let out = redact_text(input);
        assert!(out.contains("\"apiKey\":"));
        assert!(out.contains("***"));
        // 原 token 不应完整出现
        assert!(!out.contains("sk-1234567890abcdefghij"));
        // 非敏感字段保留
        assert!(out.contains("\"test\""));
    }

    #[test]
    fn redact_telegram_bot_token() {
        let input = "bot12345678:AAEx7G6R4J9o0g1k2m3p4q5r6s7t8u9v0w1x2y3z4";
        let out = redact_text(input);
        assert!(out.contains("***"));
        assert!(!out.contains("AAEx7G6R4J9o0g1k2m3p4q5r6s7t8u9v0w1x2y3z4"));
    }

    #[test]
    fn redact_preserves_non_secret_text() {
        let input = "The quick brown fox jumps over the lazy dog. token_count=42";
        let out = redact_text(input);
        // token_count 不应被误识别为 token=...(因为我们的正则要求 KEY 含 SECRET_NAMES)
        // 但 "token_count=42" 中 KEY="TOKEN_COUNT" 含 "TOKEN",会匹配 ENV_ASSIGN_RE
        // 我们的实现大小写敏感(只匹配大写),所以 "token_count=42" 不会被匹配(小写 t)
        assert!(out.contains("token_count=42"));
        assert!(out.contains("The quick brown fox"));
    }

    #[test]
    fn redact_aws_access_key() {
        let input = "AWS_KEY=AKIAIOSFODNN7EXAMPLE";
        let out = redact_text(input);
        assert!(out.contains("***"));
        // AKIA 前缀模式应被匹配
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn redact_jwt_standalone() {
        let input = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature";
        let out = redact_text(input);
        assert!(out.contains("***"));
        // 不应保留完整 JWT
        assert!(!out.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature"));
    }
}
