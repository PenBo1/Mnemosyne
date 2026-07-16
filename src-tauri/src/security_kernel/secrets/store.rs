use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::shared::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretKey {
    pub namespace: String,
    pub name: String,
}

impl SecretKey {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }

    pub fn as_compound_key(&self) -> String {
        format!("{}::{}", self.namespace, self.name)
    }
}

#[derive(Debug, Clone)]
pub struct SecretEntry {
    pub key: SecretKey,
    pub encoded_value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct SecretStore {
    secrets: Arc<Mutex<HashMap<String, SecretEntry>>>,
}

impl SecretStore {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set(&self, key: &SecretKey, value: &str) -> Result<(), AppError> {
        let encoded = encode_value(value)?;
        let now = chrono::Utc::now().timestamp();
        let compound = key.as_compound_key();

        // C18: 单次加锁完成读+写,消除双锁 TOCTOU 窗口。
        let mut guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;

        let created_at = guard.get(&compound)
            .map(|existing| existing.created_at)
            .unwrap_or(now);

        let entry = SecretEntry {
            key: key.clone(),
            encoded_value: encoded,
            created_at,
            updated_at: now,
        };
        guard.insert(compound, entry);

        Ok(())
    }

    pub fn get(&self, key: &SecretKey) -> Result<Option<String>, AppError> {
        let guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;

        let compound = key.as_compound_key();
        match guard.get(&compound) {
            Some(entry) => {
                let decoded = decode_value(&entry.encoded_value)?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    pub fn delete(&self, key: &SecretKey) -> Result<bool, AppError> {
        let mut guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;

        let compound = key.as_compound_key();
        Ok(guard.remove(&compound).is_some())
    }

    pub fn exists(&self, key: &SecretKey) -> Result<bool, AppError> {
        let guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;
        Ok(guard.contains_key(&key.as_compound_key()))
    }

    pub fn list_namespace(&self, namespace: &str) -> Result<Vec<SecretKey>, AppError> {
        let guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;

        let keys = guard
            .values()
            .filter(|e| e.key.namespace == namespace)
            .map(|e| e.key.clone())
            .collect();

        Ok(keys)
    }

    pub fn count(&self) -> Result<usize, AppError> {
        let guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;
        Ok(guard.len())
    }

    pub fn clear(&self) -> Result<(), AppError> {
        let mut guard = self.secrets.lock().map_err(|e|
            AppError::internal(format!("store lock poisoned: {}", e))
        )?;
        guard.clear();
        Ok(())
    }
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_value(value: &str) -> Result<String, AppError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    Ok(STANDARD.encode(value.as_bytes()))
}

fn decode_value(encoded: &str) -> Result<String, AppError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let bytes = STANDARD.decode(encoded).map_err(|e|
        AppError::invalid_format(format!("base64 decode failed: {}", e))
    )?;
    String::from_utf8(bytes).map_err(|e|
        AppError::invalid_format(format!("utf8 decode failed: {}", e))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_key_compound_format() {
        let key = SecretKey::new("openai", "api_key");
        assert_eq!(key.as_compound_key(), "openai::api_key");
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original = "sk-test-123456";
        let encoded = encode_value(original).unwrap();
        let decoded = decode_value(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn store_set_get_delete() {
        let store = SecretStore::new();
        let key = SecretKey::new("anthropic", "api_key");

        store.set(&key, "sk-ant-test").unwrap();
        assert!(store.exists(&key).unwrap());

        let val = store.get(&key).unwrap();
        assert_eq!(val, Some("sk-ant-test".to_string()));

        let deleted = store.delete(&key).unwrap();
        assert!(deleted);
        assert!(!store.exists(&key).unwrap());
    }

    #[test]
    fn store_update_preserves_created_at() {
        let store = SecretStore::new();
        let key = SecretKey::new("test", "key");

        store.set(&key, "value1").unwrap();
        let guard = store.secrets.lock().unwrap();
        let first_created = guard.get(&key.as_compound_key()).unwrap().created_at;
        drop(guard);

        std::thread::sleep(std::time::Duration::from_millis(10));
        store.set(&key, "value2").unwrap();

        let guard = store.secrets.lock().unwrap();
        let entry = guard.get(&key.as_compound_key()).unwrap();
        assert_eq!(entry.created_at, first_created);
        assert!(entry.updated_at > entry.created_at);
    }

    #[test]
    fn list_namespace_filters_correctly() {
        let store = SecretStore::new();
        store.set(&SecretKey::new("ns1", "a"), "v1").unwrap();
        store.set(&SecretKey::new("ns1", "b"), "v2").unwrap();
        store.set(&SecretKey::new("ns2", "c"), "v3").unwrap();

        let ns1_keys = store.list_namespace("ns1").unwrap();
        assert_eq!(ns1_keys.len(), 2);

        let ns2_keys = store.list_namespace("ns2").unwrap();
        assert_eq!(ns2_keys.len(), 1);

        let ns3_keys = store.list_namespace("ns3").unwrap();
        assert_eq!(ns3_keys.len(), 0);
    }
}