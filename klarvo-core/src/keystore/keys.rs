//! i18n keys emitted by `KeyStore` trait implementations.
//!
//! # Phase-2+ Extension Note
//!
//! Phase-2+ may extend with `PERMISSION_DENIED` (OS-Keystore user-ACL-dialog-denied,
//! distinct from backend-unavailable) and `INIT_FAILED` (backend-initialization-failure
//! distinct from runtime-unavailability). These are deliberately deferred in Phase 1 to
//! keep the key-inventory minimal.

/// Emitted when the requested key identifier does not exist in the backend.
pub const KEY_NOT_FOUND: &str = "error.keystore.not_found";

/// Emitted when the KeyStore backend is unavailable (not reachable, not compiled-in,
/// or failed to initialize at runtime).
pub const BACKEND_UNAVAILABLE: &str = "error.keystore.backend_unavailable";
