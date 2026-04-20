#![cfg(target_os = "windows")]

use secrecy::{ExposeSecret, SecretString};
use windows::Win32::Security::Credentials::{
    CredFree, CredReadW, CredDeleteW, CRED_TYPE_GENERIC, CREDENTIALW,
};
use windows::core::PCWSTR;

use klarvo_core::error::AppErrorKind;
use klarvo_core::keystore::{keys, KeyStore, os::WindowsKeystore};

fn secret(s: &str) -> SecretString {
    SecretString::new(s.into())
}

/// RAII cleanup for Credential Manager entries created during tests.
///
/// Guarantees cleanup even on test-panic (Drop fires unconditionally). Uses a
/// uuid-namespaced `app_id` so tests never collide with user or CI credentials.
struct TestKeystoreScope {
    app_id: String,
    created_keys: Vec<String>,
}

impl TestKeystoreScope {
    fn new(app_id: impl Into<String>) -> Self {
        Self { app_id: app_id.into(), created_keys: Vec::new() }
    }

    fn store(&self) -> WindowsKeystore {
        WindowsKeystore::new(self.app_id.clone())
    }

    fn register(&mut self, key: &str) {
        self.created_keys.push(key.to_string());
    }
}

impl Drop for TestKeystoreScope {
    fn drop(&mut self) {
        for key in &self.created_keys {
            let target = format!("{}/{}", self.app_id, key);
            let target_wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
            // ignore errors — Drop must never panic
            let _ = unsafe { CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0) };
        }
    }
}

#[tokio::test]
async fn set_get_roundtrip() {
    let mut scope = TestKeystoreScope::new(format!("klarvo-test-{}", uuid::Uuid::new_v4()));
    let store = scope.store();

    store.set("groq_api_key", secret("sk-test-123")).await.unwrap();
    scope.register("groq_api_key");

    let retrieved = store.get("groq_api_key").await.unwrap();
    assert_eq!(retrieved.expose_secret(), "sk-test-123");
}

#[tokio::test]
async fn get_missing_key_returns_key_not_found() {
    let scope = TestKeystoreScope::new(format!("klarvo-test-{}", uuid::Uuid::new_v4()));
    let store = scope.store();

    let err = store.get("absent").await.unwrap_err();
    assert!(matches!(err.kind, AppErrorKind::KeyMissing));
    assert_eq!(err.user_message.as_deref(), Some(keys::KEY_NOT_FOUND));
}

#[tokio::test]
async fn delete_existing_key_then_get_returns_key_not_found() {
    let mut scope = TestKeystoreScope::new(format!("klarvo-test-{}", uuid::Uuid::new_v4()));
    let store = scope.store();

    store.set("k", secret("v")).await.unwrap();
    scope.register("k");
    store.delete("k").await.unwrap();
    scope.created_keys.clear(); // already deleted; Drop would get ERROR_NOT_FOUND (harmless, but clean)

    let err = store.get("k").await.unwrap_err();
    assert_eq!(err.user_message.as_deref(), Some(keys::KEY_NOT_FOUND));
}

#[tokio::test]
async fn delete_non_existing_key_is_idempotent_ok() {
    let scope = TestKeystoreScope::new(format!("klarvo-test-{}", uuid::Uuid::new_v4()));
    let store = scope.store();
    store.delete("nonexistent").await.unwrap();
}

#[tokio::test]
async fn set_existing_key_upserts() {
    let mut scope = TestKeystoreScope::new(format!("klarvo-test-{}", uuid::Uuid::new_v4()));
    let store = scope.store();

    store.set("k", secret("first")).await.unwrap();
    scope.register("k");
    store.set("k", secret("second")).await.unwrap();

    let retrieved = store.get("k").await.unwrap();
    assert_eq!(retrieved.expose_secret(), "second");
}

#[tokio::test]
async fn target_name_prefix_convention() {
    let mut scope = TestKeystoreScope::new(format!("klarvo-test-{}", uuid::Uuid::new_v4()));
    let store = scope.store();
    let test_key = "groq_api_key";

    store.set(test_key, secret("sk-prefix-test")).await.unwrap();
    scope.register(test_key);

    // Verify TargetName format directly via raw CredReadW
    let target_name = format!("{}/{}", scope.app_id, test_key);
    let target_wide: Vec<u16> = target_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        CredReadW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0, &mut cred_ptr)
            .expect("direct CredReadW with {app_id}/{key} TargetName should succeed");
        CredFree(cred_ptr as _);
    }
}
