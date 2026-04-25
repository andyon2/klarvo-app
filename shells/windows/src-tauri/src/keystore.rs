//! Windows-shell keystore factory and boot-readiness probe.
//!
//! Exposes `make_keystore()` and `verify_keystore_ready()` used by the
//! bootstrap sequence (Story 3.10).

use std::sync::Arc;

use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::keystore::KeyStore;

/// Resolve the default path for the dev-only SQLite keystore (`%APPDATA%\Klarvo\keystore.db`).
#[cfg(feature = "dev-plain-keystore")]
fn default_keystore_path() -> std::path::PathBuf {
    std::env::var("APPDATA")
        .map(|p| std::path::PathBuf::from(p).join("Klarvo").join("keystore.db"))
        .unwrap_or_else(|_| std::path::PathBuf::from("keystore.db"))
}

/// Construct the `KeyStore` instance for this build variant.
///
/// Returns `PlainSqliteKeyStore` when `dev-plain-keystore` feature is active
/// (dev/test builds). Returns `WindowsKeystore` otherwise (release default).
/// Phase-4-Release-Default-Swap semantics: see klarvo-core/src/keystore/mod.rs.
///
/// Story 3.10 (Bootstrap-Integration) calls:
///   let keystore = make_keystore();
///   if let Err(e) = verify_keystore_ready(keystore.as_ref()).await {
///       error_emitter.emit_error(&e.user_message.unwrap_or_default(), clock.now_ms()).await;
///   }
///   app.manage(keystore);
#[cfg(feature = "dev-plain-keystore")]
pub fn make_keystore() -> Arc<dyn KeyStore> {
    Arc::new(
        klarvo_core::keystore::PlainSqliteKeyStore::open(default_keystore_path())
            .expect("PlainSqliteKeyStore init failed in dev mode"),
    )
}

#[cfg(not(feature = "dev-plain-keystore"))]
pub fn make_keystore() -> Arc<dyn KeyStore> {
    Arc::new(klarvo_core::keystore::os::WindowsKeystore::new("klarvo"))
}

/// Check that the `KeyStore` backend is accessible before the session loop starts.
///
/// Performs a probe lookup for `"klarvo_bootstrap_probe"` — a reserved identifier that
/// is never registered as a real API-key. `KEY_NOT_FOUND` is the expected happy-path
/// response (keystore reachable, probe key absent). Any other error signals that the
/// Credential Manager (or SQLite file) is inaccessible.
///
/// # Errors
///
/// Returns `AppError { kind: Io, user_message: "error.keystore.read_failed" }` when the
/// backend is unreachable. The caller (Story 3.10) emits this via `ErrorEmitter` as a
/// transient Toast (see `docs/shell-error-mapping.md` Io-Kind row for the Toast treatment).
///
/// // Checks infrastructure readiness only. Per-plugin key presence is Plugin-Init-scope.
pub async fn verify_keystore_ready(keystore: &dyn KeyStore) -> Result<(), AppError> {
    match keystore.get("klarvo_bootstrap_probe").await {
        Ok(_) => Ok(()),
        Err(e) if e.user_message.as_deref() == Some(klarvo_core::keystore::keys::KEY_NOT_FOUND) => {
            Ok(()) // Expected: probe key absent → keystore accessible but key not stored
        }
        Err(_) => Err(AppError {
            kind: AppErrorKind::Io,
            message: "keystore boot-readiness probe failed".to_string(),
            user_message: Some("error.keystore.read_failed".to_string()),
            retryable: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use klarvo_core::error::{AppError, AppErrorKind};

    use super::*;

    /// Compile-check: `make_keystore()` returns an `Arc<dyn KeyStore>`-compatible value.
    /// Runtime branch is feature-dependent (dev-plain-keystore vs. release).
    #[cfg(any(feature = "dev-plain-keystore", target_os = "windows"))]
    #[test]
    fn make_keystore_returns_arc_dyn_keystore() {
        use std::sync::Arc;
        use klarvo_core::keystore::KeyStore;
        let _ks: Arc<dyn KeyStore> = make_keystore();
    }

    #[tokio::test]
    async fn verify_keystore_ready_happy_path_probe_key_absent() {
        let ks = klarvo_test_fixtures::InMemoryKeyStore::empty();
        let result = verify_keystore_ready(&ks).await;
        assert!(result.is_ok(), "empty InMemoryKeyStore should be 'ready'");
    }

    #[tokio::test]
    async fn verify_keystore_ready_io_failure_maps_to_apperror_io() {
        let ks = klarvo_test_fixtures::FailingKeyStore::with_error(AppError {
            kind: AppErrorKind::Io,
            message: "backend unreachable".to_string(),
            user_message: Some(klarvo_core::keystore::keys::BACKEND_UNAVAILABLE.to_string()),
            retryable: false,
        });
        let err = verify_keystore_ready(&ks).await.unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Io));
        assert_eq!(err.user_message.as_deref(), Some("error.keystore.read_failed"));
    }
}
