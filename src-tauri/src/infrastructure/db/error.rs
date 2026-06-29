use crate::shared::errors::AppError;

/// 反序列化 JSON 列失败时显式报错（替代 `unwrap_or_default` 反模式）。
///
/// 返回包含原始 JSON 字符串与解析错误的 internal 错误，便于排查数据损坏。
pub(super) fn json_decode<T: serde::de::DeserializeOwned>(raw: &str, column: &str) -> Result<T, AppError> {
    serde_json::from_str(raw).map_err(|e| {
        AppError::internal(format!(
            "Failed to decode JSON column `{}`: {} (raw: {})",
            column,
            e,
            raw.chars().take(200).collect::<String>()
        ))
    })
}

/// 序列化 JSON 列失败时显式报错。
pub(super) fn json_encode<T: serde::Serialize>(value: &T, column: &str) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|e| {
        AppError::internal(format!("Failed to encode JSON column `{}`: {}", column, e))
    })
}
