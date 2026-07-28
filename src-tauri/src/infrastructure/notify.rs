//! ═══════════════════════════════════════════════════════════════════════════
//! 多渠道通知模块 - Telegram/Feishu/WeCom/Webhook
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use crate::shared::error::{AppError, IpcResponse};

/// 通知消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct NotifyMessage {
    pub title: String,
    pub body: String,
    /// Webhook 事件类型（仅 Webhook 渠道使用；None 时 dispatch 用 "notification" 兜底）。
    ///
    /// 此前 WebhookPayload.event 被硬编码为 "pipeline-complete"，导致所有
    /// 通知走同一事件类型，webhook events 过滤失效。现由调用方按事件语义传入
    /// （如 chapter-written / chapter-audited）。
    #[serde(default)]
    pub event: Option<String>,
    /// 关联 book_id（仅 Webhook 渠道使用，用于 webhook 端按书聚合）。
    #[serde(default)]
    pub book_id: Option<String>,
    /// 关联章节号（仅 Webhook 渠道使用）。
    #[serde(default)]
    pub chapter_number: Option<u32>,
}


/// 输出格式
/// - "markdown": 保留 markdown 标记(适合 IM 平台富文本)
/// - "text": 剥离 markdown,纯文本(适合简单 webhook 消费者)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotifyFormat {
    Markdown,
    Text,
}

impl Default for NotifyFormat {
    fn default() -> Self { Self::Markdown }
}

/// 通知渠道(可扩展)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum NotifyChannel {
    /// Telegram Bot
    #[serde(rename_all = "camelCase")]
    Telegram {
        bot_token: String,
        chat_id: String,
        #[serde(default)]
        format: NotifyFormat,
    },
    /// 飞书机器人
    #[serde(rename_all = "camelCase")]
    Feishu {
        webhook_url: String,
        #[serde(default)]
        format: NotifyFormat,
    },
    /// 企业微信群机器人
    #[serde(rename_all = "camelCase")]
    WechatWork {
        webhook_url: String,
        #[serde(default)]
        format: NotifyFormat,
    },
    /// 通用 webhook(支持 HMAC-SHA256 签名)
    Webhook {
        url: String,
        /// HMAC 签名密钥(可选)
        #[serde(default)]
        secret: Option<String>,
        /// 订阅事件列表(空表示订阅全部)
        #[serde(default)]
        events: Vec<String>,
    },
}

/// Webhook 事件载荷
#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub book_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<u32>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// 分发通知到所有渠道(串行发送,失败不阻塞)。
///
/// 串行而非并行的取舍：channels 数量通常 1-3，串行实现简单且避免
/// `'static` 约束带来的 clone 开销；若后续单次 dispatch 延迟成为瓶颈，
/// 可再引入 `futures_util::future::join_all` 并 clone 入参。
pub async fn dispatch_notification(
    channels: &[NotifyChannel],
    message: &NotifyMessage,
) -> Vec<NotifyResult> {
    let markdown_text = format!("**{}**\n\n{}", message.title, message.body);
    let plain_body = strip_markdown_marks(&message.body);
    let plain_text = format!("{}\n\n{}", message.title, plain_body);
    // webhook 事件 / book_id / chapter_number 缺省兜底
    let webhook_event = message.event.clone().unwrap_or_else(|| "notification".to_string());
    let webhook_book_id = message.book_id.clone().unwrap_or_default();

    let mut results = Vec::with_capacity(channels.len());
    for channel in channels {
        let result = match channel {
            NotifyChannel::Telegram { bot_token, chat_id, format } => {
                let text = if *format == NotifyFormat::Text { &plain_text } else { &markdown_text };
                send_telegram(bot_token, chat_id, text, *format).await
            }
            NotifyChannel::Feishu { webhook_url, format } => {
                let content = if *format == NotifyFormat::Text { &plain_body } else { &message.body };
                send_feishu(webhook_url, &message.title, content, *format).await
            }
            NotifyChannel::WechatWork { webhook_url, format } => {
                let content = if *format == NotifyFormat::Text { &plain_text } else { &markdown_text };
                send_wechat_work(webhook_url, content, *format).await
            }
            NotifyChannel::Webhook { url, secret, events } => {
                let payload = WebhookPayload {
                    event: webhook_event.clone(),
                    book_id: webhook_book_id.clone(),
                    chapter_number: message.chapter_number,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    data: Some(serde_json::json!({
                        "title": message.title,
                        "body": message.body,
                    })),
                };
                send_webhook(url, secret.as_deref(), events, &payload).await
            }
        };
        // 失败只 log,不抛
        if let Err(ref e) = result {
            tracing::warn!(error = %e, "notify channel failed");
        }
        results.push(NotifyResult {
            channel_type: channel_type_name(channel).to_string(),
            success: result.is_ok(),
            error: result.err().map(|e| e.to_string()),
        });
    }
    results
}

/// 单渠道发送结果
#[derive(Debug, Clone, Serialize)]
pub struct NotifyResult {
    pub channel_type: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn channel_type_name(c: &NotifyChannel) -> &'static str {
    match c {
        NotifyChannel::Telegram { .. } => "telegram",
        NotifyChannel::Feishu { .. } => "feishu",
        NotifyChannel::WechatWork { .. } => "wechat-work",
        NotifyChannel::Webhook { .. } => "webhook",
    }
}

// === Telegram ===

/// 模块级共享 reqwest::Client —— 避免每次 dispatch 重建连接池。
///
/// reqwest::Client 内部已是 Arc，clone 廉价；OnceLock 保证只 build 一次。
/// 超时配置：连接 15s、整体 30s，覆盖所有渠道发送。
fn shared_http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

async fn send_telegram(
    bot_token: &str,
    chat_id: &str,
    text: &str,
    format: NotifyFormat,
) -> Result<(), AppError> {
    validate_token(bot_token)?;
    validate_chat_id(chat_id)?;
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let mut body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
    });
    if format == NotifyFormat::Markdown {
        body["parse_mode"] = serde_json::Value::String("Markdown".into());
    }
    let client = shared_http_client();
    let resp = client.post(&url).json(&body).send().await
        .map_err(|e| AppError::internal(format!("Telegram request failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("Telegram send failed: {} {}", status, body)));
    }
    Ok(())
}

fn validate_token(token: &str) -> Result<(), AppError> {
    if token.trim().is_empty() {
        return Err(AppError::invalid_input("Telegram bot_token is empty"));
    }
    // Telegram token 格式: 数字:随机串(A-Z a-z 0-9 _ -)
    if !token.contains(':') {
        return Err(AppError::invalid_input("Telegram bot_token format invalid (missing ':')"));
    }
    Ok(())
}

fn validate_chat_id(chat_id: &str) -> Result<(), AppError> {
    if chat_id.trim().is_empty() {
        return Err(AppError::invalid_input("Telegram chat_id is empty"));
    }
    Ok(())
}

// === Feishu ===

async fn send_feishu(
    webhook_url: &str,
    title: &str,
    content: &str,
    format: NotifyFormat,
) -> Result<(), AppError> {
    validate_webhook_url(webhook_url)?;
    let payload = if format == NotifyFormat::Text {
        serde_json::json!({
            "msg_type": "text",
            "content": { "text": format!("{}\n\n{}", title, content) },
        })
    } else {
        serde_json::json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": { "tag": "plain_text", "content": title },
                    "template": "blue",
                },
                "elements": [
                    { "tag": "markdown", "content": content },
                ],
            },
        })
    };
    let client = shared_http_client();
    let resp = client.post(webhook_url).json(&payload).send().await
        .map_err(|e| AppError::internal(format!("Feishu request failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("Feishu send failed: {} {}", status, body)));
    }
    Ok(())
}

// === WeCom (企业微信) ===

async fn send_wechat_work(
    webhook_url: &str,
    content: &str,
    format: NotifyFormat,
) -> Result<(), AppError> {
    validate_webhook_url(webhook_url)?;
    let payload = if format == NotifyFormat::Text {
        serde_json::json!({ "msgtype": "text", "text": { "content": content } })
    } else {
        serde_json::json!({ "msgtype": "markdown", "markdown": { "content": content } })
    };
    let client = shared_http_client();
    let resp = client.post(webhook_url).json(&payload).send().await
        .map_err(|e| AppError::internal(format!("WeCom request failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("WeCom send failed: {} {}", status, body)));
    }
    Ok(())
}

// === Webhook (HMAC 签名) ===

async fn send_webhook(
    url: &str,
    secret: Option<&str>,
    events: &[String],
    payload: &WebhookPayload,
) -> Result<(), AppError> {
    validate_webhook_url(url)?;
    // 事件过滤
    if !events.is_empty() && !events.iter().any(|e| e == &payload.event) {
        tracing::debug!(event = %payload.event, "webhook skipped (event not subscribed)");
        return Ok(());
    }
    let body = serde_json::to_string(payload)
        .map_err(|e| AppError::internal(format!("Webhook payload serialize failed: {}", e)))?;

    let client = shared_http_client();
    let mut req = client.post(url)
        .header("Content-Type", "application/json")
        .body(body.clone());
    // HMAC-SHA256 签名(如配置了 secret)
    if let Some(secret) = secret {
        if !secret.is_empty() {
            let signature = hmac_sha256_hex(secret.as_bytes(), body.as_bytes());
            req = req.header("X-Mnemosyne-Signature", format!("sha256={}", signature));
        }
    }
    let resp = req.send().await
        .map_err(|e| AppError::internal(format!("Webhook POST failed: {}", e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("Webhook POST to {} failed: {} {}", url, status, body)));
    }
    Ok(())
}

/// 计算 HMAC-SHA256,返回十六进制字符串
fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key)
        .expect("HMAC key length error (should never happen for any length)");
    mac.update(message);
    hex::encode(mac.finalize().into_bytes())
}

/// 校验 URL 仅 http/https(防 SSRF)
fn validate_webhook_url(url: &str) -> Result<(), AppError> {
    if url.trim().is_empty() {
        return Err(AppError::invalid_input("webhook URL is empty"));
    }
    let parsed = url::Url::parse(url)
        .map_err(|e| AppError::invalid_input(format!("Invalid URL: {}", e)))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        s => Err(AppError::invalid_input(format!("URL scheme '{}' not allowed (http/https only)", s))),
    }
}

/// 剥离 markdown 标记(转纯文本)
///
/// 正则通过 OnceLock 静态缓存，避免每次调用重新编译（原实现每次调用
/// 都 `Regex::new` 3 次，编译开销显著）。
fn strip_markdown_marks(text: &str) -> String {
    static CODE_BLOCK: OnceLock<regex::Regex> = OnceLock::new();
    static BOLD: OnceLock<regex::Regex> = OnceLock::new();
    static INLINE: OnceLock<regex::Regex> = OnceLock::new();
    let code_block = CODE_BLOCK.get_or_init(|| {
        regex::Regex::new(r"```[^\n]*\n?").expect("invalid code_block regex")
    });
    let bold = BOLD.get_or_init(|| {
        regex::Regex::new(r"\*\*([^*]+)\*\*").expect("invalid bold regex")
    });
    let inline = INLINE.get_or_init(|| {
        regex::Regex::new(r"`([^`]+)`").expect("invalid inline regex")
    });
    // 移除代码块 ```lang\n...```
    let no_code_blocks = code_block.replace_all(text, "");
    // 加粗 **text** → text
    let no_bold = bold.replace_all(&no_code_blocks, "$1");
    // 行内代码 `text` → text
    let no_inline = inline.replace_all(&no_bold, "$1");
    no_inline.into_owned()
}

// === IPC 命令 ===

/// 分发通知(POST 到所有渠道)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchNotifyParams {
    pub channels: Vec<NotifyChannel>,
    pub message: NotifyMessage,
}

#[tauri::command]
pub async fn notify_dispatch(
    params: DispatchNotifyParams,
) -> Result<IpcResponse<Vec<NotifyResult>>, AppError> {
    if params.message.title.trim().is_empty() {
        return Err(AppError::invalid_input("message.title cannot be empty"));
    }
    if params.message.title.len() > 255 {
        return Err(AppError::invalid_input("message.title too long (max 255 chars)"));
    }
    if params.message.body.len() > 10_000 {
        return Err(AppError::invalid_input("message.body too long (max 10000 chars)"));
    }
    if params.channels.is_empty() {
        return Err(AppError::invalid_input("channels cannot be empty"));
    }
    if params.channels.len() > 10 {
        return Err(AppError::invalid_input("too many channels (max 10)"));
    }
    tracing::info!(
        channel_count = params.channels.len(),
        title = %params.message.title,
        "Dispatching notification"
    );
    let results = dispatch_notification(&params.channels, &params.message).await;
    let success_count = results.iter().filter(|r| r.success).count();
    tracing::info!(
        success = success_count,
        total = results.len(),
        "Notification dispatch completed"
    );
    Ok(IpcResponse::ok(results))
}

// === 章节通知格式化 ===

/// 章节事件类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChapterEvent {
    /// 章节写作完成
    Written,
    /// 审核通过
    Audited,
    /// 修订完成
    Revised,
    /// 已发布
    Published,
}

impl ChapterEvent {
    /// 事件标识(用于 webhook payload 的 event 字段)。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Written => "chapter-written",
            Self::Audited => "chapter-audited",
            Self::Revised => "chapter-revised",
            Self::Published => "chapter-published",
        }
    }

    /// 中文标签(用于通知标题)。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Written => "写作完成",
            Self::Audited => "审核通过",
            Self::Revised => "修订完成",
            Self::Published => "已发布",
        }
    }
}

/// 格式化章节通知消息。
///
/// 生成统一的 NotifyMessage,可经 dispatch_notification 推送到所有渠道。
/// `summary` 可选,用于附加章节摘要或备注。
///
/// Webhook 字段（event/book_id/chapter_number）会被填充，使 webhook 端
/// 可按事件类型与书聚合过滤；其他渠道（Telegram/Feishu/WeCom）仅使用
/// title/body，忽略这些字段。
pub fn format_chapter_notification(
    book_id: &str,
    book_title: &str,
    chapter_number: u32,
    chapter_title: Option<&str>,
    event: ChapterEvent,
    summary: Option<&str>,
) -> NotifyMessage {
    let title = format!("[{}] 第 {} 章 {}", book_title, chapter_number, event.label());
    let mut body = String::new();
    if let Some(ct) = chapter_title {
        body.push_str(&format!("**章节标题**: {}\n", ct));
    }
    body.push_str(&format!("**事件**: {}", event.label()));
    if let Some(s) = summary {
        if !s.trim().is_empty() {
            body.push_str(&format!("\n**摘要**: {}", s));
        }
    }
    NotifyMessage {
        title,
        body,
        event: Some(event.as_str().to_string()),
        book_id: Some(book_id.to_string()),
        chapter_number: Some(chapter_number),
    }
}

/// 测试通知配置 —— 向给定渠道发送一条测试消息,返回每个渠道的发送结果。
#[tauri::command]
pub async fn notify_test(
    channels: Vec<NotifyChannel>,
) -> Result<IpcResponse<Vec<NotifyResult>>, AppError> {
    if channels.is_empty() {
        return Err(AppError::invalid_input("channels cannot be empty"));
    }
    if channels.len() > 10 {
        return Err(AppError::invalid_input("too many channels (max 10)"));
    }
    let message = NotifyMessage {
        title: "Mnemosyne 通知测试".to_string(),
        body: "这是一条测试通知,用于验证通知渠道配置是否正确。".to_string(),
        ..Default::default()
    };
    let results = dispatch_notification(&channels, &message).await;
    let success_count = results.iter().filter(|r| r.success).count();
    tracing::info!(
        success = success_count,
        total = results.len(),
        "Notify test completed"
    );
    Ok(IpcResponse::ok(results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_markdown_removes_code_blocks() {
        let input = "```python\nprint('hi')\n```\n**bold** and `inline`";
        let stripped = strip_markdown_marks(input);
        assert!(!stripped.contains("```"));
        assert!(stripped.contains("bold"));
        assert!(stripped.contains("inline"));
        assert!(!stripped.contains("**"));
        assert!(!stripped.contains("`"));
    }

    #[test]
    fn validate_webhook_url_rejects_file_scheme() {
        assert!(validate_webhook_url("file:///etc/passwd").is_err());
        assert!(validate_webhook_url("ftp://example.com").is_err());
    }

    #[test]
    fn validate_webhook_url_accepts_https() {
        assert!(validate_webhook_url("https://example.com/webhook").is_ok());
        assert!(validate_webhook_url("http://localhost:8080/hook").is_ok());
    }

    #[test]
    fn validate_webhook_url_rejects_empty() {
        assert!(validate_webhook_url("").is_err());
        assert!(validate_webhook_url("   ").is_err());
    }

    #[test]
    fn validate_token_rejects_missing_colon() {
        assert!(validate_token("invalidtoken").is_err());
        assert!(validate_token("").is_err());
    }

    #[test]
    fn validate_token_accepts_valid_format() {
        assert!(validate_token("123456:ABC-DEF_hash").is_ok());
    }

    #[test]
    fn hmac_sha256_matches_known_vector() {
        // RFC 4231 Test Case 1: key=0x0b*20, data="Hi There"
        // 实际值(RFC 文本跨两行拼接):b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
        let key = vec![0x0bu8; 20];
        let msg = b"Hi There";
        let result = hmac_sha256_hex(&key, msg);
        assert_eq!(result, "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    }

    #[test]
    fn hmac_sha256_empty_key_and_message() {
        let result = hmac_sha256_hex(&[], b"");
        // HMAC of empty key + empty message (SHA-256 inner/outer hash of ipad/opad)
        // Not a standard vector, just verify it doesn't panic and returns 64 hex chars
        assert_eq!(result.len(), 64);
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn channel_type_name_returns_correct_strings() {
        let tg = NotifyChannel::Telegram {
            bot_token: "x:y".into(), chat_id: "1".into(), format: NotifyFormat::Markdown,
        };
        assert_eq!(channel_type_name(&tg), "telegram");

        let wh = NotifyChannel::Webhook {
            url: "https://x.com".into(), secret: None, events: vec![],
        };
        assert_eq!(channel_type_name(&wh), "webhook");
    }

    #[test]
    fn notify_format_default_is_markdown() {
        assert_eq!(NotifyFormat::default(), NotifyFormat::Markdown);
    }

    #[test]
    fn notify_channel_deserializes_telegram() {
        let json = r#"{"type":"telegram","botToken":"123:abc","chatId":"456","format":"text"}"#;
        let channel: NotifyChannel = serde_json::from_str(json).unwrap();
        match channel {
            NotifyChannel::Telegram { bot_token, chat_id, format } => {
                assert_eq!(bot_token, "123:abc");
                assert_eq!(chat_id, "456");
                assert_eq!(format, NotifyFormat::Text);
            }
            _ => panic!("expected Telegram"),
        }
    }

    #[test]
    fn notify_channel_deserializes_webhook_with_optional_secret() {
        let json = r#"{"type":"webhook","url":"https://example.com/hook","events":["pipeline-complete"]}"#;
        let channel: NotifyChannel = serde_json::from_str(json).unwrap();
        match channel {
            NotifyChannel::Webhook { url, secret, events } => {
                assert_eq!(url, "https://example.com/hook");
                assert!(secret.is_none());
                assert_eq!(events, vec!["pipeline-complete".to_string()]);
            }
            _ => panic!("expected Webhook"),
        }
    }

    #[test]
    fn chapter_event_as_str_and_label() {
        assert_eq!(ChapterEvent::Written.as_str(), "chapter-written");
        assert_eq!(ChapterEvent::Audited.as_str(), "chapter-audited");
        assert_eq!(ChapterEvent::Revised.as_str(), "chapter-revised");
        assert_eq!(ChapterEvent::Published.as_str(), "chapter-published");
        assert_eq!(ChapterEvent::Written.label(), "写作完成");
        assert_eq!(ChapterEvent::Published.label(), "已发布");
    }

    #[test]
    fn format_chapter_notification_with_full_fields() {
        let msg = format_chapter_notification(
            "book-1",
            "测试书",
            3,
            Some("风起"),
            ChapterEvent::Written,
            Some("主角登场"),
        );
        assert_eq!(msg.title, "[测试书] 第 3 章 写作完成");
        assert!(msg.body.contains("**章节标题**: 风起"));
        assert!(msg.body.contains("**事件**: 写作完成"));
        assert!(msg.body.contains("**摘要**: 主角登场"));
        // webhook 字段
        assert_eq!(msg.event.as_deref(), Some("chapter-written"));
        assert_eq!(msg.book_id.as_deref(), Some("book-1"));
        assert_eq!(msg.chapter_number, Some(3));
    }

    #[test]
    fn format_chapter_notification_without_optional_fields() {
        let msg = format_chapter_notification("book-2", "书名", 1, None, ChapterEvent::Audited, None);
        assert_eq!(msg.title, "[书名] 第 1 章 审核通过");
        assert!(!msg.body.contains("章节标题"));
        assert!(!msg.body.contains("摘要"));
        assert!(msg.body.contains("**事件**: 审核通过"));
        // 即使无 summary/chapter_title，event/book_id/chapter_number 仍填充
        assert_eq!(msg.event.as_deref(), Some("chapter-audited"));
        assert_eq!(msg.book_id.as_deref(), Some("book-2"));
        assert_eq!(msg.chapter_number, Some(1));
    }

    #[test]
    fn format_chapter_notification_ignores_blank_summary() {
        let msg = format_chapter_notification("b", "书", 2, None, ChapterEvent::Revised, Some("   "));
        assert!(!msg.body.contains("摘要"));
    }
}
