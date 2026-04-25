use async_trait::async_trait;
use secrecy::SecretString;

use crate::error::AppError;

/// Async interface for API-key storage and retrieval across pluggable backends.
///
/// Callers must invoke `SecretString::expose_secret()` only at their immediate use-site
/// (e.g., HTTP Bearer-header construction, platform-API-call). Never log, persist,
/// forward, or clone the exposed value. Keep exposure scope as narrow as syntactically
/// possible.
#[async_trait]
pub trait KeyStore: Send + Sync + 'static {
    /// Retrieve an API-key-secret from the configured backend by identifier.
    ///
    /// # Contract
    ///
    /// Returns `AppError` with `kind = AppErrorKind::KeyMissing` and
    /// `user_message = keys::KEY_NOT_FOUND` when no entry exists for `key`. Returns
    /// `AppError` with `kind = AppErrorKind::KeyMissing` and
    /// `user_message = keys::BACKEND_UNAVAILABLE` when the backend is unreachable or
    /// not compiled-in (e.g. OS-Keystore-Android-Scaffold-Stub pre-Phase-3).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let api_key = store.get("groq_api_key").await?;
    /// let response = client
    ///     .post(endpoint)
    ///     .header("Authorization", format!("Bearer {}", api_key.expose_secret()))
    ///     .send()
    ///     .await?;
    /// ```
    async fn get(&self, key: &str) -> Result<SecretString, AppError>;

    /// Store or overwrite an API-key-secret in the configured backend.
    ///
    /// # Contract
    ///
    /// Upsert semantics: if `key` already exists, the value is overwritten
    /// (last-write-wins). Returns `AppError` with `kind = AppErrorKind::KeyMissing` and
    /// `user_message = keys::BACKEND_UNAVAILABLE` when the backend is unreachable or
    /// not compiled-in.
    async fn set(&self, key: &str, value: SecretString) -> Result<(), AppError>;

    /// Delete an API-key-secret from the configured backend by identifier.
    ///
    /// # Contract
    ///
    /// `delete` is idempotent: returns `Ok(())` whether the key existed before the call
    /// or not. Use `get` to verify pre-existence if a caller needs that semantic.
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}
