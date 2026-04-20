#![cfg(target_os = "windows")]

//! Windows Credential Manager-backed `KeyStore` implementation.
//! `TargetName` convention is `<app_id>/<key>`. Persistence-mode is
//! `CRED_PERSIST_LOCAL_MACHINE` (machine-local, not roaming, not session-only).

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use windows::{
    core::{HRESULT, PCWSTR, PWSTR},
    Win32::{
        Foundation::ERROR_NOT_FOUND,
        Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CRED_PERSIST_LOCAL_MACHINE,
            CRED_TYPE_GENERIC, CREDENTIALW,
        },
    },
};

use crate::error::{AppError, AppErrorKind};
use crate::i18n;
use crate::keystore::{keys, KeyStore};

/// Windows Credential Manager-backed `KeyStore`.
///
/// # Intent
///
/// Stores API-keys as per-user Credential Manager entries namespaced by `app_id`.
/// TargetName format: `"<app_id>/<key>"`. See module-level documentation for
/// Phase-4-Release-Default-Swap semantics.
///
/// # Contract
///
/// Error mapping: `ERROR_NOT_FOUND (1168)` → `KEY_NOT_FOUND`;
/// all other Win32 errors → `BACKEND_UNAVAILABLE`.
/// `delete` is idempotent: `ERROR_NOT_FOUND` is treated as success.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use klarvo_core::keystore::{KeyStore, os::WindowsKeystore};
///
/// # async fn run() -> Result<(), klarvo_core::error::AppError> {
/// let store = WindowsKeystore::new("klarvo");
/// let key = store.get("groq_api_key").await?;
/// // use key.expose_secret() at the call-site only
/// # Ok(())
/// # }
/// ```
///
/// See module-level documentation for Phase-4-Release-Default-Swap semantics.
#[derive(Debug)]
pub struct WindowsKeystore {
    app_id: String,
}

impl WindowsKeystore {
    /// Create a Credential Manager keystore scoped to `app_id`.
    ///
    /// `app_id` is used as a TargetName prefix: `"<app_id>/<key>"`.
    /// Empty or whitespace `app_id` is a programming error (`debug_assert!`);
    /// the constructor is otherwise infallible.
    ///
    /// See module-level documentation for the NFR4 swap semantics.
    pub fn new(app_id: impl Into<String>) -> Self {
        let id = app_id.into();
        debug_assert!(!id.trim().is_empty(), "WindowsKeystore: app_id must not be empty");
        Self { app_id: id }
    }

    fn target_name(&self, key: &str) -> String {
        format!("{}/{}", self.app_id, key)
    }
}

/// Null-terminated UTF-16 encoding helper.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn key_not_found_err(key: &str) -> AppError {
    debug_assert!(i18n::is_key(keys::KEY_NOT_FOUND));
    AppError {
        kind: AppErrorKind::KeyMissing,
        message: format!("windows-keystore: key '{key}' not found"),
        user_message: Some(keys::KEY_NOT_FOUND.to_string()),
        retryable: false,
    }
}

fn backend_unavailable_err(detail: impl std::fmt::Display) -> AppError {
    debug_assert!(i18n::is_key(keys::BACKEND_UNAVAILABLE));
    AppError {
        kind: AppErrorKind::KeyMissing,
        message: format!("windows-keystore: backend unavailable: {detail}"),
        user_message: Some(keys::BACKEND_UNAVAILABLE.to_string()),
        retryable: false,
    }
}

fn is_not_found(e: &windows::core::Error) -> bool {
    e.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0)
}

#[async_trait]
impl KeyStore for WindowsKeystore {
    /// Retrieve a key from the Windows Credential Manager.
    ///
    /// # Contract
    ///
    /// `ERROR_NOT_FOUND` → `KEY_NOT_FOUND`; other errors → `BACKEND_UNAVAILABLE`.
    async fn get(&self, key: &str) -> Result<SecretString, AppError> {
        let target = self.target_name(key);
        let target_wide = to_wide(&target);
        let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

        match unsafe { CredReadW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0, &mut cred_ptr) } {
            Err(e) if is_not_found(&e) => Err(key_not_found_err(key)),
            Err(e) => Err(backend_unavailable_err(e)),
            Ok(()) => {
                let value = unsafe {
                    let cred = &*cred_ptr;
                    let blob = std::slice::from_raw_parts(
                        cred.CredentialBlob,
                        cred.CredentialBlobSize as usize,
                    );
                    let s = String::from_utf8_lossy(blob).into_owned();
                    CredFree(cred_ptr as _);
                    s
                };
                Ok(SecretString::new(value.into()))
            }
        }
    }

    /// Store or overwrite a key in the Windows Credential Manager (upsert).
    ///
    /// # Contract
    ///
    /// `expose_secret()` is called inline in the CredWriteW binding only
    /// (narrow-expose per 1C.1-AC-2).
    async fn set(&self, key: &str, value: SecretString) -> Result<(), AppError> {
        let target = self.target_name(key);
        let mut target_wide = to_wide(&target);
        let blob = value.expose_secret().as_bytes();

        unsafe {
            let mut cred: CREDENTIALW = std::mem::zeroed();
            cred.Type = CRED_TYPE_GENERIC;
            cred.TargetName = PWSTR(target_wide.as_mut_ptr());
            cred.CredentialBlobSize = blob.len() as u32;
            cred.CredentialBlob = blob.as_ptr() as *mut u8;
            cred.Persist = CRED_PERSIST_LOCAL_MACHINE;
            CredWriteW(&cred, 0)
        }
        .map_err(|e| backend_unavailable_err(e))
    }

    /// Delete a key from the Windows Credential Manager.
    ///
    /// # Contract
    ///
    /// Idempotent: `ERROR_NOT_FOUND` is treated as success.
    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let target = self.target_name(key);
        let target_wide = to_wide(&target);

        match unsafe { CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0) } {
            Ok(()) => Ok(()),
            Err(e) if is_not_found(&e) => Ok(()), // idempotent — key already absent
            Err(e) => Err(backend_unavailable_err(e)),
        }
    }
}
