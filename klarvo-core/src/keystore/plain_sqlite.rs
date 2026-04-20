#![cfg(feature = "dev-plain-keystore")]

//! `PlainSqliteKeyStore` stores API-keys in plaintext within a local SQLite file. This is
//! **Security-Theater** (NFR4): a Windows-ACL-restriction on current-user read/write mitigates
//! casual-access by other OS-users, but does **not** protect against privileged-process-read,
//! disk-backup-extraction, or malware running as the same user. This implementation exists
//! **only** behind the `dev-plain-keystore` Cargo-feature and is **never** compiled into
//! release-builds. Real API-key-protection comes via the OS-Keystore-Impl (Phase-4
//! release-default per FR46, ref `memory/project_api_key_os_keystore_mvp`).
//!
//! `rusqlite` calls are inline-blocking inside async-methods — acceptable for Phase-1-dev-only-
//! scope. Phase-2+ OS-Keystore implementations (1C.3) may have non-trivial I/O latency (Windows
//! Credential Manager roundtrips, Android IPC) and should evaluate `tokio::task::spawn_blocking`
//! wrapping at that time.

use std::path::Path;

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::Mutex;

use crate::error::{AppError, AppErrorKind};
use crate::i18n;
use crate::keystore::{keys, KeyStore};

/// Plain-SQLite KeyStore backend. See module-level doc for NFR4 security-disclosure.
///
/// Use only in `dev-plain-keystore`-gated builds.
#[derive(Debug)]
pub struct PlainSqliteKeyStore {
    conn: Mutex<rusqlite::Connection>,
}

impl PlainSqliteKeyStore {
    /// Open a SQLite file at `path` and initialize schema. Fresh file is created if absent.
    ///
    /// # Contract
    ///
    /// Returns `AppError::kind::KeyMissing` with `user_message = keys::BACKEND_UNAVAILABLE`
    /// when SQLite-connection-init or table-CREATE fails.
    ///
    /// See module-level documentation for the NFR4 security-disclosure. Use only in
    /// `dev-plain-keystore`-gated builds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use klarvo_core::keystore::{KeyStore, PlainSqliteKeyStore};
    ///
    /// let store = PlainSqliteKeyStore::open("/tmp/keystore.db").expect("open");
    /// let ks: Arc<dyn KeyStore> = Arc::new(store);
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let conn = rusqlite::Connection::open(path).map_err(schema_init_err)?;
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// In-memory SQLite backend — no file-I/O, no cleanup-ritual. Use in tests.
    ///
    /// # Contract
    ///
    /// Returns `AppError::kind::KeyMissing` with `user_message = keys::BACKEND_UNAVAILABLE`
    /// when SQLite-connection-init or table-CREATE fails.
    ///
    /// See module-level documentation for the NFR4 security-disclosure. Use only in
    /// `dev-plain-keystore`-gated builds.
    pub fn in_memory() -> Result<Self, AppError> {
        let conn = rusqlite::Connection::open_in_memory().map_err(schema_init_err)?;
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn init_schema(conn: &rusqlite::Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                 name  TEXT PRIMARY KEY NOT NULL,
                 value TEXT NOT NULL
             )",
            [],
        )
        .map_err(schema_init_err)?;
        Ok(())
    }
}

fn schema_init_err(e: rusqlite::Error) -> AppError {
    debug_assert!(i18n::is_key(keys::BACKEND_UNAVAILABLE));
    AppError {
        kind: AppErrorKind::KeyMissing,
        message: format!("plain-sqlite: schema init failed: {e}"),
        user_message: Some(keys::BACKEND_UNAVAILABLE.to_string()),
        retryable: false,
    }
}

#[async_trait]
impl KeyStore for PlainSqliteKeyStore {
    async fn get(&self, key: &str) -> Result<SecretString, AppError> {
        let conn = self.conn.lock().await;
        let result: Result<String, rusqlite::Error> = conn.query_row(
            "SELECT value FROM api_keys WHERE name = ?1",
            [key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(value) => Ok(SecretString::new(value.into())),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug_assert!(i18n::is_key(keys::KEY_NOT_FOUND));
                Err(AppError {
                    kind: AppErrorKind::KeyMissing,
                    message: format!("plain-sqlite: key '{key}' not found"),
                    user_message: Some(keys::KEY_NOT_FOUND.to_string()),
                    retryable: false,
                })
            }
            Err(e) => {
                debug_assert!(i18n::is_key(keys::BACKEND_UNAVAILABLE));
                Err(AppError {
                    kind: AppErrorKind::KeyMissing,
                    message: format!("plain-sqlite: query failed for key '{key}': {e}"),
                    user_message: Some(keys::BACKEND_UNAVAILABLE.to_string()),
                    retryable: false,
                })
            }
        }
    }

    async fn set(&self, key: &str, value: SecretString) -> Result<(), AppError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO api_keys (name, value) VALUES (?1, ?2)",
            rusqlite::params![key, value.expose_secret()],
        )
        .map_err(|e| {
            debug_assert!(i18n::is_key(keys::BACKEND_UNAVAILABLE));
            AppError {
                kind: AppErrorKind::KeyMissing,
                message: format!("plain-sqlite: insert failed for key '{key}': {e}"),
                user_message: Some(keys::BACKEND_UNAVAILABLE.to_string()),
                retryable: false,
            }
        })?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM api_keys WHERE name = ?1", [key])
            .map_err(|e| {
                debug_assert!(i18n::is_key(keys::BACKEND_UNAVAILABLE));
                AppError {
                    kind: AppErrorKind::KeyMissing,
                    message: format!("plain-sqlite: delete failed for key '{key}': {e}"),
                    user_message: Some(keys::BACKEND_UNAVAILABLE.to_string()),
                    retryable: false,
                }
            })?;
        Ok(()) // idempotent — Ok(()) whether rows_affected > 0 or == 0
    }
}
