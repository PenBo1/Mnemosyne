//! ═══════════════════════════════════════════════════════════════════════════
//! 密钥管理 - API 密钥安全存储与访问
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use keyring::Entry;
use tauri::{AppHandle, State};

use crate::infrastructure::db::state::DbState;
use crate::shared::error::{AppError, IpcResponse};

// ── 常量 ────────────────────────────────────────────────────────────────────

/// keyring 服务名（与应用 ID 一致）
const KEYRING_SERVICE: &str = "com.admin.mnemosyne";

/// keyring 账户前缀
const KEYRING_PREFIX: &str = "api_key";

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

// ── 占位符检测 ──────────────────────────────────────────────────────────────

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

// ── 密钥存储（系统密钥库优先）──────────────────────────────────────────────

/// 密钥存储
///
/// 优先使用系统密钥库（Windows DPAPI/macOS Keychain/Linux Secret Service）。
/// 仅在密钥库不可用时回退到文件存储（明文，发出警告）。
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

    /// 获取 keyring entry
    fn keyring_entry(&self, key: &str) -> Result<Entry, AppError> {
        let account = format!("{}:{}", KEYRING_PREFIX, key);
        Entry::new(KEYRING_SERVICE, &account)
            .map_err(|e| AppError::internal(format!("Failed to create keyring entry: {}", e)))
    }

    /// 获取所有密钥列表
    ///
    /// 优先从系统密钥库读取，不可用时回退到文件。
    /// 注意：keyring 不支持枚举，此处从文件读取 keys 列表（不含值）。
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

    /// 获取密钥值
    ///
    /// 优先从系统密钥库读取，不可用时回退到文件。
    pub fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        // 优先尝试系统密钥库
        match self.get_from_keyring(key) {
            Ok(Some(value)) => return Ok(Some(value)),
            Ok(None) => {
                tracing::debug!(key, "[secrets] not found in keyring, trying file");
            }
            Err(e) => {
                tracing::warn!(key, error = %e, "[secrets] keyring read failed, falling back to file");
            }
        }

        // 回退到文件存储
        self.get_from_file(key)
    }

    /// 从系统密钥库获取
    fn get_from_keyring(&self, key: &str) -> Result<Option<String>, AppError> {
        let entry = self.keyring_entry(key)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::internal(format!("Keyring get failed: {}", e))),
        }
    }

    /// 从文件获取
    fn get_from_file(&self, key: &str) -> Result<Option<String>, AppError> {
        let all = self.get_all()?;
        let value = all.get(key).and_then(|v| v.as_str()).map(|s| s.to_string());
        Ok(value)
    }

    /// 设置密钥
    ///
    /// 优先存储到系统密钥库，失败时回退到文件（发出警告）。
    /// 同时更新文件中的 key 列表（用于 get_all 枚举）。
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), AppError> {
        tracing::info!(key, "[secrets] set: started");

        // 尝试存储到系统密钥库
        match self.set_to_keyring(key, value) {
            Ok(()) => {
                tracing::info!(key, "[secrets] stored to keyring successfully");
            }
            Err(e) => {
                tracing::warn!(
                    key,
                    error = %e,
                    "[secrets] keyring write failed, falling back to file (WARNING: plaintext storage)"
                );
                // 回退到文件存储（明文）
                self.set_to_file(key, value)?;
            }
        }

        // 更新文件中的 key 列表（用于枚举）
        self.update_key_list(key)?;

        tracing::info!(key, "[secrets] set: completed");
        Ok(())
    }

    /// 存储到系统密钥库
    fn set_to_keyring(&self, key: &str, value: &str) -> Result<(), AppError> {
        let entry = self.keyring_entry(key)?;
        entry.set_password(value)
            .map_err(|e| AppError::internal(format!("Keyring set failed: {}", e)))?;
        Ok(())
    }

    /// 存储到文件（明文，仅作为回退）
    fn set_to_file(&self, key: &str, value: &str) -> Result<(), AppError> {
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
        Ok(())
    }

    /// 更新文件中的 key 列表
    fn update_key_list(&self, key: &str) -> Result<(), AppError> {
        let mut all = self.get_all()?;
        if !all.contains_key(key) {
            // 仅记录 key 存在，值为占位符（真实值在 keyring）
            all.insert(key.to_string(), serde_json::json!("__keyring__"));
            let path = self.secrets_file();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let json = serde_json::to_string_pretty(&all)
                .map_err(|e| AppError::internal(format!("Failed to serialize: {}", e)))?;
            std::fs::write(&path, &json)
                .map_err(|e| AppError::internal(format!("Failed to write: {}", e)))?;
        }
        Ok(())
    }

    /// 删除密钥
    ///
    /// 同时从系统密钥库和文件中删除。
    pub fn delete(&mut self, key: &str) -> Result<bool, AppError> {
        tracing::info!(key, "[secrets] delete: started");

        let mut removed = false;

        // 从系统密钥库删除
        match self.delete_from_keyring(key) {
            Ok(true) => removed = true,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(key, error = %e, "[secrets] keyring delete failed");
            }
        }

        // 从文件删除
        match self.delete_from_file(key) {
            Ok(true) => removed = true,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(key, error = %e, "[secrets] file delete failed");
            }
        }

        tracing::info!(key, removed, "[secrets] delete: completed");
        Ok(removed)
    }

    /// 从系统密钥库删除
    fn delete_from_keyring(&self, key: &str) -> Result<bool, AppError> {
        let entry = self.keyring_entry(key)?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(AppError::internal(format!("Keyring delete failed: {}", e))),
        }
    }

    /// 从文件删除
    fn delete_from_file(&self, key: &str) -> Result<bool, AppError> {
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
        Ok(removed)
    }

    /// 检查密钥是否存在
    pub fn exists(&self, key: &str) -> Result<bool, AppError> {
        // 优先检查系统密钥库
        if self.get_from_keyring(key)?.is_some() {
            return Ok(true);
        }
        // 回退检查文件
        let all = self.get_all()?;
        Ok(all.contains_key(key))
    }
}

// ── 状态管理 ────────────────────────────────────────────────────────────────

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

// ── IPC 命令 ────────────────────────────────────────────────────────────────

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

// ── 测试模块 ────────────────────────────────────────────────────────────────

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