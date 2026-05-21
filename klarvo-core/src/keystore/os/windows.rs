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

/// Decode a Credential Manager blob to a String.
///
/// Heuristic for encoding detection:
/// - If blob length is even AND any second-byte-of-pair is `\0`, treat as UTF-16-LE.
///   This is the encoding Windows-native tools (`cmdkey`, `New-StoredCredential`, etc.)
///   use when writing generic credentials.
/// - Otherwise decode as UTF-8 (Klarvo's own `set()` writes UTF-8 via `String::as_bytes()`).
///
/// Trailing whitespace and NUL bytes (from UTF-16 terminator or copy-paste artifacts)
/// are stripped — API-key style values never contain leading/trailing whitespace.
fn decode_credential_blob(blob: &[u8]) -> String {
    let looks_utf16 = !blob.is_empty()
        && blob.len() % 2 == 0
        && blob.chunks_exact(2).any(|chunk| chunk[1] == 0);

    let raw = if looks_utf16 {
        let u16s: Vec<u16> = blob
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(blob).into_owned()
    };

    raw.trim_matches(|c: char| c.is_whitespace() || c == '\0')
        .to_string()
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

        match unsafe { CredReadW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, Some(0), &mut cred_ptr) } {
            Err(e) if is_not_found(&e) => Err(key_not_found_err(key)),
            Err(e) => Err(backend_unavailable_err(e)),
            Ok(()) => {
                let value = unsafe {
                    let cred = &*cred_ptr;
                    let blob = std::slice::from_raw_parts(
                        cred.CredentialBlob,
                        cred.CredentialBlobSize as usize,
                    );
                    let s = decode_credential_blob(blob);
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
        .map_err(backend_unavailable_err)
    }

    /// Delete a key from the Windows Credential Manager.
    ///
    /// # Contract
    ///
    /// Idempotent: `ERROR_NOT_FOUND` is treated as success.
    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let target = self.target_name(key);
        let target_wide = to_wide(&target);

        match unsafe { CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, Some(0)) } {
            Ok(()) => Ok(()),
            Err(e) if is_not_found(&e) => Ok(()), // idempotent — key already absent
            Err(e) => Err(backend_unavailable_err(e)),
        }
    }
}

#[cfg(test)]
mod decode_tests {
    use super::decode_credential_blob;

    #[test]
    fn utf8_ascii_roundtrip() {
        // Klarvo's own set() writes UTF-8. ASCII-only blob → UTF-8 decode path.
        let blob = b"gsk_abcdef1234";
        assert_eq!(decode_credential_blob(blob), "gsk_abcdef1234");
    }

    #[test]
    fn utf16_le_from_cmdkey() {
        // cmdkey on Windows writes UTF-16-LE. "gsk_" → 0x67 0x00 0x73 0x00 0x6B 0x00 0x5F 0x00
        let blob: &[u8] = &[0x67, 0x00, 0x73, 0x00, 0x6B, 0x00, 0x5F, 0x00];
        assert_eq!(decode_credential_blob(blob), "gsk_");
    }

    #[test]
    fn utf16_le_with_trailing_nul_terminator() {
        // cmdkey often stores a NUL-terminator at the end. "gsk\0" in UTF-16-LE.
        let blob: &[u8] = &[0x67, 0x00, 0x73, 0x00, 0x6B, 0x00, 0x00, 0x00];
        assert_eq!(decode_credential_blob(blob), "gsk");
    }

    #[test]
    fn strips_trailing_newline_utf8() {
        // Copy-paste artifact: trailing \n in UTF-8-stored value.
        let blob = b"gsk_abcdef\n";
        assert_eq!(decode_credential_blob(blob), "gsk_abcdef");
    }

    #[test]
    fn strips_trailing_whitespace_utf16() {
        // UTF-16-LE encoding of "gsk_x " (trailing space).
        let blob: &[u8] = &[
            0x67, 0x00, 0x73, 0x00, 0x6B, 0x00, 0x5F, 0x00, 0x78, 0x00, 0x20, 0x00,
        ];
        assert_eq!(decode_credential_blob(blob), "gsk_x");
    }

    #[test]
    fn empty_blob() {
        assert_eq!(decode_credential_blob(&[]), "");
    }

    #[test]
    fn odd_length_blob_falls_back_to_utf8() {
        // Odd length cannot be UTF-16. Should decode as UTF-8.
        let blob = b"abc";
        assert_eq!(decode_credential_blob(blob), "abc");
    }
}
