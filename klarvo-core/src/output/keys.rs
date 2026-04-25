//! i18n-key constants for `OutputTarget` errors.
//!
//! Phase-2+ may extend with additional error keys as new `OutputTarget` implementations are
//! introduced (e.g., `error.output.paste_injection_blocked` for Epic-3 Win32-SendInput failures).
//! Keys are additive; no existing key changes meaning once introduced.

pub const TARGET_NOT_FOUND: &str = "error.output.target_not_found";
pub const CLIPBOARD_UNAVAILABLE: &str = "error.output.clipboard_unavailable";
