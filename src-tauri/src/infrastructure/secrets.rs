
use std::sync::Mutex;

use tauri::AppHandle;

use crate::shared::error::{AppError, IpcResponse};

#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use tauri::Manager;

/// 平台密钥存储状态。
/// Linux: 文件回退 + 内存缓存; 其他平台: 无状态 (keyring crate 自管)。
#[derive(Default)]
pub struct SecretsState {
    #[cfg(target_os = "linux")]
    cache: Mutex<Option<HashMap<String, String>>>,
    #[cfg(not(target_os = "linux"))]
    _phantom: Mutex<()>,
}

// ── Linux 文件回退实现 ──────────────────────────────────────────

#[cfg(target_os = "linux")]
fn key(service: &str, account: &str) -> String {
    format!("{}::{}", service, account)
}

#[cfg(target_os = "linux")]
fn store_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| AppError::internal(format!("failed to resolve data dir: {}", e)))?;
    fs::create_dir_all(&dir).map_err(|e| AppError::internal(format!("mkdir failed: {}", e)))?;
    Ok(dir.join("secrets.json"))
}

#[cfg(target_os = "linux")]
fn read_store(app: &AppHandle) -> Result<HashMap<String, String>, AppError> {
    read_store_at(&store_path(app)?)
}

#[cfg(target_os = "linux")]
fn read_store_at(path: &std::path::Path) -> Result<HashMap<String, String>, AppError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = fs::read(path).map_err(|e| AppError::file_read_error(path.display().to_string()))?;
    serde_json::from_slice::<HashMap<String, String>>(&bytes).map_err(|e| {
        AppError::invalid_format(format!("secrets file corrupt: {}", e))
    })
}

#[cfg(target_os = "linux")]
fn write_store(app: &AppHandle, map: &HashMap<String, String>) -> Result<(), AppError> {
    write_store_at(&store_path(app)?, map)
}

#[cfg(target_os = "linux")]
fn write_store_at(path: &std::path::Path, map: &HashMap<String, String>) -> Result<(), AppError> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(map).map_err(|e| AppError::internal(format!("serialize: {}", e)))?;

    // 0600: 仅文件属主可读写
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| AppError::file_write_error(tmp.display().to_string()))?;
    f.write_all(&bytes)
        .map_err(|e| AppError::file_write_error(tmp.display().to_string()))?;
    f.sync_all()
        .map_err(|e| AppError::file_write_error(tmp.display().to_string()))?;
    fs::rename(&tmp, path).map_err(|e| AppError::internal(format!("rename: {}", e)))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn with_store<F, R>(app: &AppHandle, state: &SecretsState, f: F) -> Result<R, AppError>
where
    F: FnOnce(&mut HashMap<String, String>) -> R,
{
    let mut guard = state
        .cache
        .lock()
        .map_err(|e| AppError::internal(format!("cache lock poisoned: {}", e)))?;
    if guard.is_none() {
        *guard = Some(read_store(app)?);
    }
    let map = guard.as_mut().expect("cache initialized above");
    Ok(f(map))
}

// ── 非 Linux keyring 实现 ───────────────────────────────────────

#[cfg(not(target_os = "linux"))]
fn entry(service: &str, account: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(service, account)
        .map_err(|e| AppError::internal(format!("keyring entry: {}", e)))
}

// ═══════════════════════════════════════════════════════════════
// Tauri 命令
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn secrets_get(
    app: AppHandle,
    state: tauri::State<'_, SecretsState>,
    service: String,
    account: String,
) -> Result<IpcResponse<Option<String>>, AppError> {
    #[cfg(target_os = "linux")]
    {
        let _ = state;
        let val = with_store(&app, &state, |m| m.get(&key(&service, &account)).cloned())?;
        Ok(IpcResponse::ok(val))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, state);
        let e = entry(&service, &account)?;
        match e.get_password() {
            Ok(v) => Ok(IpcResponse::ok(Some(v))),
            Err(keyring::Error::NoEntry) => Ok(IpcResponse::ok(None)),
            Err(err) => Err(AppError::internal(format!("keyring get: {}", err))),
        }
    }
}

#[tauri::command]
pub async fn secrets_set(
    app: AppHandle,
    state: tauri::State<'_, SecretsState>,
    service: String,
    account: String,
    password: String,
) -> Result<IpcResponse<()>, AppError> {
    #[cfg(target_os = "linux")]
    {
        let k = key(&service, &account);
        with_store(&app, &state, |m| {
            m.insert(k, password);
        })?;
        let snapshot = {
            let guard = state
                .cache
                .lock()
                .map_err(|e| AppError::internal(format!("cache lock poisoned: {}", e)))?;
            guard.as_ref().cloned().unwrap_or_default()
        };
        write_store(&app, &snapshot)?;
        Ok(IpcResponse::no_content())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, state);
        let e = entry(&service, &account)?;
        e.set_password(&password)
            .map_err(|e| AppError::internal(format!("keyring set: {}", e)))?;
        Ok(IpcResponse::no_content())
    }
}

#[tauri::command]
pub async fn secrets_delete(
    app: AppHandle,
    state: tauri::State<'_, SecretsState>,
    service: String,
    account: String,
) -> Result<IpcResponse<()>, AppError> {
    #[cfg(target_os = "linux")]
    {
        let k = key(&service, &account);
        with_store(&app, &state, |m| {
            m.remove(&k);
        })?;
        let snapshot = {
            let guard = state
                .cache
                .lock()
                .map_err(|e| AppError::internal(format!("cache lock poisoned: {}", e)))?;
            guard.as_ref().cloned().unwrap_or_default()
        };
        write_store(&app, &snapshot)?;
        Ok(IpcResponse::no_content())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, state);
        let e = entry(&service, &account)?;
        match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(IpcResponse::no_content()),
            Err(err) => Err(AppError::internal(format!("keyring delete: {}", err))),
        }
    }
}

/// 批量读 — 单次 IPC roundtrip, 适合冷启动 fan-out。
#[tauri::command]
pub async fn secrets_get_all(
    app: AppHandle,
    state: tauri::State<'_, SecretsState>,
    service: String,
    accounts: Vec<String>,
) -> Result<IpcResponse<Vec<Option<String>>>, AppError> {
    #[cfg(target_os = "linux")]
    {
        let vals = with_store(&app, &state, |m| {
            accounts
                .iter()
                .map(|a| m.get(&key(&service, a)).cloned())
                .collect::<Vec<_>>()
        })?;
        Ok(IpcResponse::ok(vals))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, state);
        let vals = accounts
            .into_iter()
            .map(|a| {
                keyring::Entry::new(&service, &a)
                    .ok()
                    .and_then(|e| e.get_password().ok())
            })
            .collect::<Vec<_>>();
        Ok(IpcResponse::ok(vals))
    }
}

// ── Linux 单测 (文件回退逻辑) ───────────────────────────────────

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    use tempfile::TempDir;

    #[test]
    fn key_format_is_service_double_colon_account() {
        assert_eq!(key("openai", "alice"), "openai::alice");
        assert_eq!(key("", ""), "::");
    }

    #[test]
    fn read_store_at_missing_path_is_empty() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("nope.json");
        let map = read_store_at(&p).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn write_then_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("secrets.json");
        let mut m = HashMap::new();
        m.insert(key("svc", "alice"), "p1".into());
        m.insert(key("svc", "bob"), "p2".into());

        write_store_at(&p, &m).unwrap();
        let loaded = read_store_at(&p).unwrap();
        assert_eq!(loaded, m);
    }

    #[test]
    fn write_uses_mode_0600() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("secrets.json");
        write_store_at(&p, &HashMap::new()).unwrap();

        let mode = fs::metadata(&p).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o600, "secrets file must be user-only readable");
    }

    #[test]
    fn write_does_not_leave_tmp_file_on_success() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("secrets.json");
        write_store_at(&p, &HashMap::new()).unwrap();

        let tmp_path = p.with_extension("json.tmp");
        assert!(!tmp_path.exists(), "tmp file must be renamed away on success");
    }

    #[test]
    fn write_overwrites_existing_atomically() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("secrets.json");

        let mut first = HashMap::new();
        first.insert("a".into(), "1".into());
        write_store_at(&p, &first).unwrap();

        let mut second = HashMap::new();
        second.insert("b".into(), "2".into());
        write_store_at(&p, &second).unwrap();

        let loaded = read_store_at(&p).unwrap();
        assert_eq!(loaded, second);
        assert!(!loaded.contains_key("a"));
    }

    #[test]
    fn read_store_at_garbage_file_errors() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("secrets.json");
        fs::write(&p, b"not json").unwrap();
        assert!(read_store_at(&p).is_err());
    }
}
