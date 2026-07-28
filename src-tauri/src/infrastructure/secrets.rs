//! ═══════════════════════════════════════════════════════════════════════════
//! 密钥管理 - API 密钥存储与访问
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, State};

use crate::infrastructure::db::state::DbState;
use crate::shared::error::{AppError, IpcResponse};

/// 占位符 token 常见字面量(小写匹配)。
const PLACEHOLDER_LITERALS: &[&str] = &[
    "placeholder",
    "your-api-key",
    "your_api_key",
    "your-api-key-here",
    "your_api_key_here",
    "your-secret-key",
    "your_secret_key",
    "xxx",
    "xxxx",
    "test",
    "example",
    "dummy",
    "todo",
    "tbd",
    "n/a",
    "none",
    "null",
];

/// 检测 token 是否为占位符/示例值(而非真实密钥)。
///
/// 用于在加载 secrets 时拒绝明显未配置的占位值,避免误用模板/示例 key 发起 API 请求。
/// 大小写不敏感,并忽略首尾空白。简化版检测:覆盖常见字面量、`sk-xxx` 模板与 `<...>`/`{...}` 占位符。
pub fn is_placeholder_token(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_lowercase();

    if PLACEHOLDER_LITERALS.contains(&lower.as_str()) {
        return true;
    }

    // 形如 sk-xxx / sk-... / sk-placeholder 等模板
    if let Some(rest) = lower.strip_prefix("sk-") {
        if rest.is_empty() {
            return true;
        }
        // rest 全为占位字符(x / . / - / _)或命中字面量
        if rest.chars().all(|c| matches!(c, 'x' | '.' | '-' | '_'))
            || PLACEHOLDER_LITERALS.contains(&rest)
        {
            return true;
        }
    }

    // 模板占位符 <...> / {...}
    if (trimmed.starts_with('<') && trimmed.ends_with('>'))
        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
    {
        return true;
    }

    false
}

#[derive(Default)]
pub struct SecretsStore {
    data_dir: PathBuf,
}

impl SecretsStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
        }
    }

    fn secrets_file(&self) -> PathBuf {
        self.data_dir.join("config").join("secrets.json")
    }

    pub fn get_all(&self) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
        let path = self.secrets_file();
        if !path.exists() {
            return Ok(serde_json::Map::new());
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| {
                tracing::error!(path = %path.display(), error = %e, "[secrets] failed to read file");
                AppError::internal(format!("Failed to read secrets: {}", e))
            })?;
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&raw)
            .map_err(|e| {
                tracing::error!(path = %path.display(), error = %e, "[secrets] failed to parse JSON");
                AppError::internal(format!("Failed to parse secrets: {}", e))
            })?;
        Ok(map)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let all = self.get_all()?;
        let value = all.get(key).and_then(|v| v.as_str()).map(|s| s.to_string());
        Ok(value)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), AppError> {
        tracing::info!(key, "[secrets] set: started");
        let mut all = self.get_all()?;
        all.insert(key.to_string(), serde_json::json!(value));
        let path = self.secrets_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| {
                    tracing::error!(path = %parent.display(), error = %e, "[secrets] failed to create directory");
                    AppError::internal(format!("Failed to create directory: {}", e))
                })?;
        }
        let json = serde_json::to_string_pretty(&all)
            .map_err(|e| {
                tracing::error!(error = %e, "[secrets] failed to serialize");
                AppError::internal(format!("Failed to serialize secrets: {}", e))
            })?;
        std::fs::write(&path, &json)
            .map_err(|e| {
                tracing::error!(path = %path.display(), error = %e, "[secrets] failed to write file");
                AppError::internal(format!("Failed to write secrets: {}", e))
            })?;
        tracing::info!(key, "[secrets] set: completed");
        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> Result<bool, AppError> {
        tracing::info!(key, "[secrets] delete: started");
        let mut all = self.get_all()?;
        let removed = all.remove(key).is_some();
        if removed {
            let path = self.secrets_file();
            let json = serde_json::to_string_pretty(&all)
                .map_err(|e| {
                    tracing::error!(error = %e, "[secrets] failed to serialize");
                    AppError::internal(format!("Failed to serialize secrets: {}", e))
                })?;
            std::fs::write(&path, &json)
                .map_err(|e| {
                    tracing::error!(path = %path.display(), error = %e, "[secrets] failed to write file");
                    AppError::internal(format!("Failed to write secrets: {}", e))
                })?;
        }
        tracing::info!(key, removed, "[secrets] delete: completed");
        Ok(removed)
    }

    pub fn exists(&self, key: &str) -> Result<bool, AppError> {
        let all = self.get_all()?;
        Ok(all.contains_key(key))
    }
}

#[derive(Default, Clone)]
pub struct SecretsState {
    pub inner: Arc<std::sync::Mutex<SecretsStore>>,
}

impl SecretsState {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(SecretsStore::new(data_dir))),
        }
    }
}

pub fn get_secret(
    _app: &AppHandle,
    state: &State<'_, SecretsState>,
    service: &str,
    account: &str,
) -> Result<Option<String>, AppError> {
    let key = format!("{}_{}", service, account);
    let store = state.inner.lock().unwrap();
    store.get(&key)
}

#[tauri::command]
pub async fn secrets_get(
    state: State<'_, DbState>,
    key: String,
) -> Result<IpcResponse<Option<String>>, AppError> {
    let start = Instant::now();
    tracing::info!(key = %key, "[secrets_get] started");

    let store = SecretsStore::new(state.data_dir.root());
    let value = store.get(&key)?;

    tracing::info!(key = %key, found = value.is_some(), duration_ms = start.elapsed().as_millis() as u64, "[secrets_get] completed");
    Ok(IpcResponse::ok(value))
}

#[tauri::command]
pub async fn secrets_set(
    state: State<'_, DbState>,
    key: String,
    value: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(key = %key, "[secrets_set] started");

    let mut store = SecretsStore::new(state.data_dir.root());
    store.set(&key, &value)?;

    tracing::info!(key = %key, duration_ms = start.elapsed().as_millis() as u64, "[secrets_set] completed");
    Ok(IpcResponse::ok(true))
}

#[tauri::command]
pub async fn secrets_delete(
    state: State<'_, DbState>,
    key: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(key = %key, "[secrets_delete] started");

    let mut store = SecretsStore::new(state.data_dir.root());
    let removed = store.delete(&key)?;

    tracing::info!(key = %key, removed, duration_ms = start.elapsed().as_millis() as u64, "[secrets_delete] completed");
    Ok(IpcResponse::ok(removed))
}

#[tauri::command]
pub async fn secrets_get_all(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<String>>, AppError> {
    let start = Instant::now();
    tracing::info!("[secrets_get_all] started");

    let store = SecretsStore::new(state.data_dir.root());
    let all = store.get_all()?;
    let keys: Vec<String> = all.keys().cloned().collect();

    tracing::info!(count = keys.len(), duration_ms = start.elapsed().as_millis() as u64, "[secrets_get_all] completed");
    Ok(IpcResponse::ok(keys))
}

#[tauri::command]
pub async fn secrets_exists(
    state: State<'_, DbState>,
    key: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(key = %key, "[secrets_exists] started");

    let store = SecretsStore::new(state.data_dir.root());
    let exists = store.exists(&key)?;

    tracing::info!(key = %key, exists, duration_ms = start.elapsed().as_millis() as u64, "[secrets_exists] completed");
    Ok(IpcResponse::ok(exists))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_placeholder_token() {
        // 空字符串与纯空白
        assert!(is_placeholder_token(""));
        assert!(is_placeholder_token("   "));
        assert!(is_placeholder_token("\t\n"));

        // 常见占位符字面量(大小写不敏感)
        assert!(is_placeholder_token("placeholder"));
        assert!(is_placeholder_token("PLACEHOLDER"));
        assert!(is_placeholder_token("your-api-key-here"));
        assert!(is_placeholder_token("YOUR_API_KEY_HERE"));
        assert!(is_placeholder_token("your-api-key"));
        assert!(is_placeholder_token("your-secret-key"));
        assert!(is_placeholder_token("xxx"));
        assert!(is_placeholder_token("XXXX"));
        assert!(is_placeholder_token("test"));
        assert!(is_placeholder_token("example"));
        assert!(is_placeholder_token("dummy"));
        assert!(is_placeholder_token("todo"));
        assert!(is_placeholder_token("n/a"));
        assert!(is_placeholder_token("none"));
        assert!(is_placeholder_token("null"));

        // sk- 模板形式
        assert!(is_placeholder_token("sk-xxx"));
        assert!(is_placeholder_token("sk-..."));
        assert!(is_placeholder_token("sk-"));
        assert!(is_placeholder_token("SK-XXX"));
        assert!(is_placeholder_token("sk-test"));

        // 模板占位符 <...> / {...}
        assert!(is_placeholder_token("<your-api-key>"));
        assert!(is_placeholder_token("{YOUR_KEY}"));

        // 首尾空白仍判定为占位符
        assert!(is_placeholder_token("  placeholder  "));

        // 真实 token 不应被误判
        assert!(!is_placeholder_token("sk-proj-abc123XYZdef456"));
        assert!(!is_placeholder_token("sk-ant-api03-RealKeyValue999"));
        assert!(!is_placeholder_token("abcdef0123456789"));
        assert!(!is_placeholder_token("sk-live-9f8e7d6c5b4a3210fedcba"));
    }
}