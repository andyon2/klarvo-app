#![cfg(target_os = "android")]

//! Android-Keystore scaffold-stub. All trait-methods return `AppError::kind::KeyMissing`
//! with `keys::BACKEND_UNAVAILABLE` and a cause-chain explaining the Phase-3-Deferral.
//! Real Android-Keystore integration (JNI + Android-Keystore-System-API per
//! `memory/project_jni_dual_surface`) is Phase-3-scope, gated by
//! AccessibilityService-Policy-Audit (ref `memory/project_play_store_phase3_blocker`).
//! Trait-signature is stable across the Phase-3-swap — only method-bodies are replaced.
//!
//! Phase-1-Scaffold uses a structured error message as a lightweight detail-carrier.
//! Phase-3-real-impl may introduce a dedicated `KeystoreBackendError`-type if
//! Android-specific error-paths prove diverse enough to warrant dedicated taxonomy.

use async_trait::async_trait;
use secrecy::SecretString;

use crate::error::{AppError, AppErrorKind};
use crate::i18n;
use crate::keystore::{keys, KeyStore};

/// Android Keystore scaffold-stub. All methods unconditionally fail until Phase-3.
///
/// # Intent
///
/// Provides a stable `KeyStore`-compliant type for Android-target builds in Phase-1
/// without requiring JNI, NDK, or Android-OS-API access. See module-level documentation
/// for Phase-4-Release-Default-Swap semantics.
///
/// # Contract
///
/// All methods unconditionally return `AppError::kind::KeyMissing` with
/// `user_message = keys::BACKEND_UNAVAILABLE`. No successful get/set/delete path exists
/// in the Phase-1-scaffold; full impl lands in Phase-3.
///
/// # Example
///
/// ```no_run
/// use klarvo_core::keystore::os::AndroidKeystore;
/// use klarvo_core::error::AppErrorKind;
///
/// # async fn run() {
/// let store = AndroidKeystore::new("klarvo");
/// match store.get("groq_api_key").await {
///     Err(e) if matches!(e.kind, AppErrorKind::KeyMissing) => { /* Phase-3-fallback */ }
///     _ => unreachable!("Android scaffold returns KeyMissing uniformly"),
/// }
/// # }
/// ```
///
/// See module-level documentation for Phase-4-Release-Default-Swap semantics.
#[derive(Debug)]
pub struct AndroidKeystore {
    app_id: String,
}

impl AndroidKeystore {
    /// Create an Android Keystore placeholder scoped to `app_id`. Infallible.
    ///
    /// # Contract
    ///
    /// All subsequent `get`/`set`/`delete` calls will return
    /// `AppError::kind::KeyMissing` + `user_message = keys::BACKEND_UNAVAILABLE`.
    /// No I/O or JNI calls are made.
    pub fn new(app_id: impl Into<String>) -> Self {
        Self { app_id: app_id.into() }
    }

    fn phase3_scaffold_error(&self) -> AppError {
        debug_assert!(i18n::is_key(keys::BACKEND_UNAVAILABLE));
        AppError {
            kind: AppErrorKind::KeyMissing,
            message: format!(
                "KeyStore not available on Android in Phase-1 scaffold (app_id: '{}') — \
                 Phase-3 scope (AccessibilityService-Policy-Audit blocker, \
                 ref project_play_store_phase3_blocker)",
                self.app_id
            ),
            user_message: Some(keys::BACKEND_UNAVAILABLE.to_string()),
            retryable: false,
        }
    }
}

#[async_trait]
impl KeyStore for AndroidKeystore {
    async fn get(&self, _key: &str) -> Result<SecretString, AppError> {
        Err(self.phase3_scaffold_error())
    }

    async fn set(&self, _key: &str, _value: SecretString) -> Result<(), AppError> {
        Err(self.phase3_scaffold_error())
    }

    async fn delete(&self, _key: &str) -> Result<(), AppError> {
        Err(self.phase3_scaffold_error())
    }
}
