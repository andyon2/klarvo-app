use std::collections::HashMap;

use async_trait::async_trait;
use secrecy::SecretString;
use tokio::sync::Mutex;

use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::keystore::{keys, KeyStore};

/// In-memory `KeyStore` fixture for unit and integration tests.
///
/// Backed by a `tokio::sync::Mutex<HashMap>`. Thread-safe, async, and
/// `Arc<dyn KeyStore>`-compatible. Use `empty()` for a blank store or
/// `with_pairs()` to pre-populate keys.
pub struct InMemoryKeyStore {
    store: Mutex<HashMap<String, SecretString>>,
}

impl InMemoryKeyStore {
    /// Create an empty store.
    pub fn empty() -> Self {
        Self { store: Mutex::new(HashMap::new()) }
    }

    /// Create a store pre-populated with the given key-value pairs.
    pub fn with_pairs(pairs: impl IntoIterator<Item = (&'static str, SecretString)>) -> Self {
        let map = pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        Self { store: Mutex::new(map) }
    }
}

#[async_trait]
impl KeyStore for InMemoryKeyStore {
    async fn get(&self, key: &str) -> Result<SecretString, AppError> {
        let guard = self.store.lock().await;
        guard.get(key).cloned().ok_or_else(|| {
            debug_assert!(klarvo_core::i18n::is_key(keys::KEY_NOT_FOUND));
            AppError {
                kind: AppErrorKind::KeyMissing,
                message: format!("key '{key}' not found"),
                user_message: Some(keys::KEY_NOT_FOUND.to_string()),
                retryable: false,
            }
        })
    }

    async fn set(&self, key: &str, value: SecretString) -> Result<(), AppError> {
        self.store.lock().await.insert(key.to_string(), value);
        Ok(()) // upsert — last-write-wins
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.store.lock().await.remove(key);
        Ok(()) // idempotent — Ok(()) whether present or not
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    fn secret(s: &str) -> SecretString {
        SecretString::new(s.into())
    }

    #[tokio::test]
    async fn get_missing_key_returns_key_missing_error() {
        let store = InMemoryKeyStore::empty();
        let err = store.get("missing_key").await.unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::KeyMissing));
        assert_eq!(err.user_message.as_deref(), Some(keys::KEY_NOT_FOUND));
        assert!(!err.retryable);
    }

    #[tokio::test]
    async fn set_then_get_roundtrip() {
        let store = InMemoryKeyStore::empty();
        store.set("groq_api_key", secret("sk-test-123")).await.unwrap();
        let retrieved = store.get("groq_api_key").await.unwrap();
        assert_eq!(retrieved.expose_secret(), "sk-test-123");
    }

    #[tokio::test]
    async fn delete_missing_key_is_idempotent_ok() {
        let store = InMemoryKeyStore::empty();
        store.delete("nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn delete_existing_then_get_returns_key_missing() {
        let store = InMemoryKeyStore::empty();
        store.set("to_delete", secret("value")).await.unwrap();
        store.delete("to_delete").await.unwrap();
        let err = store.get("to_delete").await.unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::KeyMissing));
    }

    #[tokio::test]
    async fn set_overwrites_existing_key_upsert() {
        let store = InMemoryKeyStore::empty();
        store.set("api_key", secret("first")).await.unwrap();
        store.set("api_key", secret("second")).await.unwrap();
        let retrieved = store.get("api_key").await.unwrap();
        assert_eq!(retrieved.expose_secret(), "second");
    }

    #[tokio::test]
    async fn with_pairs_constructor_prepopulates_store() {
        let store = InMemoryKeyStore::with_pairs([
            ("groq_api_key", secret("sk-groq")),
            ("deepseek_api_key", secret("sk-ds")),
        ]);
        assert_eq!(store.get("groq_api_key").await.unwrap().expose_secret(), "sk-groq");
        assert_eq!(store.get("deepseek_api_key").await.unwrap().expose_secret(), "sk-ds");
    }

    #[test]
    fn in_memory_keystore_is_arc_dyn_compatible() {
        fn assert_arc_dyn<T: KeyStore>(_: std::sync::Arc<T>) {}
        assert_arc_dyn(std::sync::Arc::new(InMemoryKeyStore::empty()));
    }
}
