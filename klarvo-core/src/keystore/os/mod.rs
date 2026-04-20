//! Platform-native `KeyStore` implementations. Phase-1 provides `WindowsKeystore` (real,
//! via `windows-rs` Credential-Manager) and `AndroidKeystore` (Phase-3-deferred
//! scaffold-stub; all methods return `AppError::kind::KeyMissing` with
//! `keys::BACKEND_UNAVAILABLE`). Phase-4-Release-Default-Swap: disabling the
//! `dev-plain-keystore` Cargo-feature removes `PlainSqliteKeyStore` from the compile,
//! leaving the platform-native impl as the only `KeyStore`-provider on-target. No
//! `KeyStore`-Trait-Signature change is required — the swap is purely a
//! compile-feature-flag toggle. macOS and Linux are explicitly out-of-scope for Phase 1;
//! adding them is a Phase-5+ story with its own ADR.

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsKeystore;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub use android::AndroidKeystore;
