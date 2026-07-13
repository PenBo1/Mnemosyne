pub mod store;
pub mod keyring;

pub use store::{SecretKey, SecretEntry, SecretStore};
pub use keyring::KeyringBackend;

use std::sync::{Arc, Mutex};
use crate::shared::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Memory,
    Keyring,
    Both,
}

pub struct SecretManager {
    store: SecretStore,
    keyring: Option<KeyringBackend>,
    backend_preference: StorageBackend,
}

impl SecretManager {
    pub fn new(service_name: impl Into<String>) -> Self {
        let keyring = if KeyringBackend::is_available() {
            Some(KeyringBackend::new(service_name))
        } else {
            None
        };

        Self {
            store: SecretStore::new(),
            keyring,
            backend_preference: StorageBackend::Both,
        }
    }

    pub fn with_backend_preference(mut self, preference: StorageBackend) -> Self {
        self.backend_preference = preference;
        self
    }

    pub fn memory_only() -> Self {
        Self {
            store: SecretStore::new(),
            keyring: None,
            backend_preference: StorageBackend::Memory,
        }
    }

    pub fn set(&self, key: &SecretKey, value: &str) -> Result<(), AppError> {
        match self.backend_preference {
            StorageBackend::Memory => {
                self.store.set(key, value)?;
            }
            StorageBackend::Keyring => {
                if let Some(ref keyring) = self.keyring {
                    keyring.set(key, value)?;
                } else {
                    return Err(AppError::unavailable("Keyring backend not available"));
                }
            }
            StorageBackend::Both => {
                self.store.set(key, value)?;
                if let Some(ref keyring) = self.keyring {
                    if let Err(e) = keyring.set(key, value) {
                        tracing::warn!(error = %e, key = %key.as_compound_key(), "keyring set failed (memory fallback succeeded)");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &SecretKey) -> Result<Option<String>, AppError> {
        match self.backend_preference {
            StorageBackend::Memory => {
                self.store.get(key)
            }
            StorageBackend::Keyring => {
                if let Some(ref keyring) = self.keyring {
                    keyring.get(key)
                } else {
                    Err(AppError::unavailable("Keyring backend not available"))
                }
            }
            StorageBackend::Both => {
                if let Some(ref keyring) = self.keyring {
                    if let Some(v) = keyring.get(key)? {
                        return Ok(Some(v));
                    }
                }
                self.store.get(key)
            }
        }
    }

    pub fn delete(&self, key: &SecretKey) -> Result<bool, AppError> {
        let mut deleted_any = false;

        match self.backend_preference {
            StorageBackend::Memory => {
                deleted_any = self.store.delete(key)?;
            }
            StorageBackend::Keyring => {
                if let Some(ref keyring) = self.keyring {
                    deleted_any = keyring.delete(key)?;
                } else {
                    return Err(AppError::unavailable("Keyring backend not available"));
                }
            }
            StorageBackend::Both => {
                if self.store.delete(key)? {
                    deleted_any = true;
                }
                if let Some(ref keyring) = self.keyring {
                    if keyring.delete(key)? {
                        deleted_any = true;
                    }
                }
            }
        }

        Ok(deleted_any)
    }

    pub fn exists(&self, key: &SecretKey) -> Result<bool, AppError> {
        match self.backend_preference {
            StorageBackend::Memory => {
                self.store.exists(key)
            }
            StorageBackend::Keyring => {
                if let Some(ref keyring) = self.keyring {
                    keyring.get(key)?.map(|_| true).ok_or_else(|| 
                        AppError::unavailable("Keyring backend not available")
                    )
                } else {
                    Err(AppError::unavailable("Keyring backend not available"))
                }
            }
            StorageBackend::Both => {
                if let Some(ref keyring) = self.keyring {
                    if keyring.get(key)?.is_some() {
                        return Ok(true);
                    }
                }
                self.store.exists(key)
            }
        }
    }

    pub fn list_namespace(&self, namespace: &str) -> Result<Vec<SecretKey>, AppError> {
        self.store.list_namespace(namespace)
    }

    pub fn count(&self) -> Result<usize, AppError> {
        self.store.count()
    }

    pub fn clear(&self) -> Result<(), AppError> {
        self.store.clear()?;
        Ok(())
    }

    pub fn backend_preference(&self) -> StorageBackend {
        self.backend_preference
    }

    pub fn keyring_available(&self) -> bool {
        self.keyring.is_some()
    }
}

pub type SharedSecretManager = Arc<Mutex<SecretManager>>;

pub fn create_shared_manager(service_name: impl Into<String>) -> SharedSecretManager {
    Arc::new(Mutex::new(SecretManager::new(service_name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_only_backend() {
        let manager = SecretManager::memory_only();
        assert_eq!(manager.backend_preference(), StorageBackend::Memory);
        assert!(!manager.keyring_available());
    }

    #[test]
    fn set_get_delete_cycle() {
        let manager = SecretManager::memory_only();
        let key = SecretKey::new("test", "api_key");

        manager.set(&key, "sk-test-123").unwrap();
        assert!(manager.exists(&key).unwrap());

        let val = manager.get(&key).unwrap();
        assert_eq!(val, Some("sk-test-123".to_string()));

        assert!(manager.delete(&key).unwrap());
        assert!(!manager.exists(&key).unwrap());
    }

    #[test]
    fn list_namespace() {
        let manager = SecretManager::memory_only();
        manager.set(&SecretKey::new("ns1", "a"), "v1").unwrap();
        manager.set(&SecretKey::new("ns1", "b"), "v2").unwrap();
        manager.set(&SecretKey::new("ns2", "c"), "v3").unwrap();

        let keys = manager.list_namespace("ns1").unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn shared_manager_creation() {
        let shared = create_shared_manager("mnemosyne");
        let guard = shared.lock().unwrap();
        assert!(guard.keyring_available() || guard.backend_preference() == StorageBackend::Memory);
    }
}