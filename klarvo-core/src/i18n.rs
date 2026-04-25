//! i18n key format contract for `klarvo-core`.
//!
//! # Key Format Contract
//!
//! All i18n keys emitted by core MUST match:
//!
//! ```text
//! ^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$
//! ```
//!
//! Rules:
//! - ASCII lowercase letters, digits, and underscores only.
//! - Dot-notation required: at least one `.` separating namespace segments.
//! - First character must be `[a-z]` (no leading digit or underscore).
//! - Each segment after a dot must be non-empty.
//!
//! Examples of valid keys: `"recording.started"`, `"error.pipeline.unknown_stage_type"`, `"app.ready"`.
//!
//! # snake_case vs kebab-case
//!
//! Key segments use `snake_case` (underscores), **not** kebab-case.
//! This is distinct from Tauri event names, which default to kebab-case per
//! ADR-0002 Amendment 1. i18n keys are internal identifiers resolved by shells;
//! they never appear as Tauri event names.
//!
//! # Usage
//!
//! ```rust,ignore
//! use klarvo_core::i18n::assert_is_key;
//!
//! // At an emission site — panics in debug + release if the key is malformed:
//! assert_is_key("error.pipeline.unknown_stage_type");
//! ```
//!
//! Use [`is_key`] for non-panicking validation (e.g., in tests or conditional logic).
//!
//! # References
//!
//! - FR34 (Epic 5): `cargo xtask lint-events` static gate will import [`KEY_REGEX`] as
//!   single-source-of-truth; no format-regex duplication between runtime assertion and linter.
//! - NFR G3: Core must never emit user-facing strings; keys are the approved shell boundary.

/// Key-format regex — single source of truth shared with `cargo xtask lint-events` (FR34).
///
/// Pattern: `^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$`
pub const KEY_REGEX: &str = r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$";

/// Returns `true` if `s` is a valid i18n key per [`KEY_REGEX`].
pub fn is_key(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    let mut dot_count: usize = 0;
    let mut segment_len_after_dot: usize = 0;
    for &b in &bytes[1..] {
        if b == b'.' {
            if dot_count > 0 && segment_len_after_dot == 0 {
                return false; // consecutive or trailing dots
            }
            dot_count += 1;
            segment_len_after_dot = 0;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' {
            if dot_count > 0 {
                segment_len_after_dot += 1;
            }
        } else {
            return false; // non-ASCII, uppercase, whitespace, disallowed punctuation
        }
    }
    dot_count >= 1 && segment_len_after_dot > 0
}

/// Asserts that `s` is a valid i18n key, panicking with a diagnostic that contains
/// the input value if validation fails.
///
/// Keys are structurally public identifiers; the panic message exposes only the key
/// string, never surrounding payload data (NFR5 compliance).
pub fn assert_is_key(s: &str) {
    assert!(is_key(s), "invalid i18n key: {s:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_keys_pass() {
        assert!(is_key("error.pipeline.unknown_stage_type"));
        assert!(is_key("recording.started"));
        assert!(is_key("app.ready"));
    }

    #[test]
    #[should_panic(expected = "error pipeline")]
    fn invalid_whitespace_panics() {
        assert_is_key("error pipeline");
    }

    #[test]
    #[should_panic(expected = "fehler")]
    fn invalid_non_ascii_panics() {
        assert_is_key("fehler.löschen");
    }

    #[test]
    #[should_panic(expected = "error,key")]
    fn invalid_punctuation_panics() {
        assert_is_key("error,key");
    }

    #[test]
    #[should_panic(expected = "invalid i18n key")]
    fn invalid_empty_string_panics() {
        assert_is_key("");
    }

    #[test]
    #[should_panic(expected = "Error.Key")]
    fn invalid_uppercase_panics() {
        assert_is_key("Error.Key");
    }

    #[test]
    #[should_panic(expected = "0.foo")]
    fn invalid_leading_digit_panics() {
        assert_is_key("0.foo");
    }

    #[test]
    #[should_panic(expected = "recording")]
    fn invalid_no_dot_panics() {
        assert_is_key("recording");
    }
}
