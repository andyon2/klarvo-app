#![cfg(feature = "dev-plain-keystore")]

use secrecy::{ExposeSecret, SecretString};

use klarvo_core::error::AppErrorKind;
use klarvo_core::keystore::{keys, KeyStore, PlainSqliteKeyStore};

fn secret(s: &str) -> SecretString {
    SecretString::new(s.into())
}

#[tokio::test]
async fn set_get_roundtrip() {
    let store = PlainSqliteKeyStore::in_memory().expect("in_memory");
    store.set("groq_api_key", secret("sk-test-123")).await.unwrap();
    let retrieved = store.get("groq_api_key").await.unwrap();
    assert_eq!(retrieved.expose_secret(), "sk-test-123");
}

#[tokio::test]
async fn get_missing_key_returns_key_not_found() {
    let store = PlainSqliteKeyStore::in_memory().expect("in_memory");
    let err = store.get("absent").await.unwrap_err();
    assert!(matches!(err.kind, AppErrorKind::KeyMissing));
    assert_eq!(err.user_message.as_deref(), Some(keys::KEY_NOT_FOUND));
}

#[tokio::test]
async fn delete_existing_key_then_get_returns_key_not_found() {
    let store = PlainSqliteKeyStore::in_memory().expect("in_memory");
    store.set("k", secret("v")).await.unwrap();
    store.delete("k").await.unwrap();
    let err = store.get("k").await.unwrap_err();
    assert_eq!(err.user_message.as_deref(), Some(keys::KEY_NOT_FOUND));
}

#[tokio::test]
async fn delete_non_existing_key_is_idempotent_ok() {
    let store = PlainSqliteKeyStore::in_memory().expect("in_memory");
    store.delete("nonexistent").await.unwrap();
}

#[tokio::test]
async fn set_existing_key_upserts() {
    let store = PlainSqliteKeyStore::in_memory().expect("in_memory");
    store.set("k", secret("first")).await.unwrap();
    store.set("k", secret("second")).await.unwrap();
    let retrieved = store.get("k").await.unwrap();
    assert_eq!(retrieved.expose_secret(), "second");
}

#[tokio::test]
async fn in_memory_constructor_no_file_io() {
    // Marker-test: verifies in_memory() is usable without a filesystem path.
    let store = PlainSqliteKeyStore::in_memory().expect("in_memory");
    store.set("k", secret("v")).await.unwrap();
    assert_eq!(store.get("k").await.unwrap().expose_secret(), "v");
}

#[tokio::test]
async fn open_on_non_existent_parent_dir_returns_backend_unavailable() {
    let path = "/tmp/klarvo-test-nonexistent-dir-xyz-1c2/keystore.db";
    let err = PlainSqliteKeyStore::open(path).unwrap_err();
    assert!(matches!(err.kind, AppErrorKind::KeyMissing));
    assert_eq!(err.user_message.as_deref(), Some(keys::BACKEND_UNAVAILABLE));
    // Divergenz 2: AppError has no source-field; assert message contains error-context.
    assert!(
        err.message.contains("plain-sqlite:"),
        "message should contain plain-sqlite error-context: {}",
        err.message
    );
}
