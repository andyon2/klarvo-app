//! License validation module for the Dikta open-core monetization model.
//!
//! ## Key format
//!
//! `DIKTA-XXXX-XXXX-XXXX-XXXX`
//!
//! The payload is Base32-encoded (RFC 4648, no padding) with groups of 4
//! characters separated by hyphens.  Decoded it is 20 bytes:
//! - bytes 0..12 : 96-bit payload (version byte + 11 bytes of random data)
//! - bytes 12..20: first 8 bytes of HMAC-SHA256(secret, payload[0..12])
//!
//! The `DIKTA-` prefix + 4 groups of 4 Base32 chars = 4 * 4 = 16 chars of
//! Base32 = 10 bytes of data per group... actually let's be precise:
//!
//! Base32: 5 bits per char. 20 bytes = 160 bits = 32 chars.
//! We display as `DIKTA-` + 4 groups of 4 + the remaining chars.
//! To keep it simple: 20 bytes → 32 Base32 chars → split as 8-8-8-8.
//! Display: `DIKTA-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX` is too long.
//!
//! Revised: 12 bytes payload + 4 bytes HMAC-truncated = 16 bytes total.
//! 16 bytes = 128 bits → 26 Base32 chars (padded to 32, but we strip padding).
//! We use 20 chars split 4-4-4-4 with one more group -- simpler:
//!
//! Final design: 12-byte payload + 8-byte HMAC = 20 bytes.
//! 20 bytes in Base32 = 32 chars (with padding). Strip `=`.
//! Display as: `DIKTA-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX` (too long).
//!
//! Practical approach matching the briefing `DIKTA-XXXX-XXXX-XXXX-XXXX`:
//! - 4 groups of 4 Base32 chars = 16 chars = 80 bits = 10 bytes.
//! - Split: 6 bytes payload + 4 bytes HMAC-truncated = 10 bytes.
//! - Regex: `^DIKTA-[A-Z2-7]{4}-[A-Z2-7]{4}-[A-Z2-7]{4}-[A-Z2-7]{4}$`
//!
//! ## HMAC secret
//!
//! Two constants are concatenated at runtime so a simple `strings` binary
//! scan won't find the full secret.
//!
//! ## Caching
//!
//! After first validation the key + timestamp are persisted in config.json.
//! `compute_status_from_cache` recalculates the status on every app start
//! without network access.

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

// ---------------------------------------------------------------------------
// HMAC secret -- split across two constants so `strings` won't find it whole
// ---------------------------------------------------------------------------

/// First half of the embedded HMAC secret.
const SECRET_PART_A: &[u8] = b"dikta-license-v1";
/// Second half of the embedded HMAC secret.
const SECRET_PART_B: &[u8] = b"-2025-open-core!";

/// Combines the two secret parts into a single key used for HMAC operations.
/// The result is dropped after use -- not stored as a static.
fn build_secret() -> Vec<u8> {
    let mut s = Vec::with_capacity(SECRET_PART_A.len() + SECRET_PART_B.len());
    s.extend_from_slice(SECRET_PART_A);
    s.extend_from_slice(SECRET_PART_B);
    s
}

// ---------------------------------------------------------------------------
// Grace period durations (in seconds)
// ---------------------------------------------------------------------------

/// A validated license key is considered fully licensed for 30 days.
const LICENSED_DURATION_SECS: u64 = 30 * 24 * 60 * 60;

/// After 30 days, a 48-hour grace period is granted before downgrading.
const GRACE_PERIOD_SECS: u64 = 48 * 60 * 60;

/// Early-adopter migration: existing users get a 60-day grace period.
pub const EARLY_ADOPTER_GRACE_SECS: u64 = 60 * 24 * 60 * 60;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Current license state of the application.
///
/// This is derived from the cached key + timestamp on every app start.
/// It is stored in `AppState` and checked by `is_feature_allowed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LicenseStatus {
    /// No license key configured. Free tier applies.
    Unlicensed,
    /// License was validated but the 30-day offline window has elapsed.
    /// A 48-hour grace period is active; `until` is the Unix timestamp
    /// at which the grace period expires.
    GracePeriod { until: u64 },
    /// Fully licensed. All paid features are unlocked.
    Licensed,
}

/// Paid features that require a valid license.
///
/// Each variant maps to a specific capability gated behind the paid tier.
/// Free-tier features are implicitly allowed and not listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LicensedFeature {
    /// Non-default STT providers (OpenAI Whisper) and non-default LLM
    /// providers (OpenAI, Anthropic, Groq LLM).
    AlternativeProviders,
    /// Verbatim and Chat cleanup styles (Polished is free).
    AllCleanupStyles,
    /// User-supplied custom LLM system prompts.
    CustomPrompts,
    /// Command Mode: voice-edit selected text.
    CommandMode,
    /// User-defined reusable text snippets.
    Snippets,
    /// Per-application recording profiles.
    AppProfiles,
    /// History entries beyond the free-tier limit + full-text search.
    UnlimitedHistory,
    /// Voice Notes Mode: save transcription as a note instead of pasting.
    VoiceNotes,
    /// Cross-device history sync via Turso.
    Sync,
    /// Offline transcription via whisper.cpp (when implemented).
    OfflineMode,
    /// Whisper Mode: amplified mic for quiet dictation.
    WhisperMode,
    /// Filler-word frequency analysis.
    FillerAnalysis,
    /// API cost tracking dashboard.
    CostTracking,
}

// ---------------------------------------------------------------------------
// Key validation
// ---------------------------------------------------------------------------

/// Validates a license key string.
///
/// Returns `LicenseStatus::Licensed` if the key is correctly formatted and
/// its HMAC matches. Returns `Err` with a human-readable message otherwise.
///
/// This does NOT check the cache timestamp -- call `compute_status_from_cache`
/// for that. This function only answers: "is this a genuine Dikta key?"
pub fn validate_license_key(key: &str) -> Result<LicenseStatus, String> {
    if key.is_empty() {
        return Err("License key is empty".to_string());
    }

    // Expected format: DIKTA-XXXX-XXXX-XXXX-XXXX
    // 5 segments separated by '-', first is "DIKTA", rest are 4-char Base32 groups.
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 5 {
        return Err(format!(
            "Invalid key format: expected 5 dash-separated segments, got {}",
            parts.len()
        ));
    }

    if parts[0] != "DIKTA" {
        return Err("Invalid key format: must start with 'DIKTA'".to_string());
    }

    for (i, group) in parts[1..].iter().enumerate() {
        if group.len() != 4 {
            return Err(format!(
                "Invalid key format: segment {} has {} chars, expected 4",
                i + 1,
                group.len()
            ));
        }
        if !group.chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7')) {
            return Err(format!(
                "Invalid key format: segment {} contains invalid Base32 characters",
                i + 1
            ));
        }
    }

    // Decode the 16 Base32 chars (4 groups * 4 chars) into bytes.
    // RFC 4648 Base32: 16 chars = 80 bits = 10 bytes.
    let b32_str: String = parts[1..].join("");
    let decoded = base32::decode(base32::Alphabet::RFC4648 { padding: false }, &b32_str)
        .ok_or_else(|| "Invalid key: Base32 decoding failed".to_string())?;

    if decoded.len() != 10 {
        return Err(format!(
            "Invalid key: decoded length is {} bytes, expected 10",
            decoded.len()
        ));
    }

    // Layout: bytes 0..6 = payload, bytes 6..10 = first 4 bytes of HMAC.
    let payload = &decoded[0..6];
    let stored_hmac = &decoded[6..10];

    // Verify HMAC.
    let secret = build_secret();
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret)
        .map_err(|e| format!("HMAC init error: {e}"))?;
    mac.update(payload);
    let computed = mac.finalize().into_bytes();

    // Constant-time comparison of the 4-byte truncated HMAC.
    let computed_truncated = &computed[..4];
    if !constant_time_eq(computed_truncated, stored_hmac) {
        return Err("Invalid key: HMAC verification failed".to_string());
    }

    Ok(LicenseStatus::Licensed)
}

/// Computes the license status from a cached key and the timestamp at which
/// it was last validated.
///
/// - If `key` is empty: returns `Unlicensed`.
/// - If `key` is invalid (bad HMAC): returns `Unlicensed`.
/// - If `validated_at == 0`: returns `Unlicensed`.
/// - If `now - validated_at <= 30 days`: returns `Licensed`.
/// - If `now - validated_at <= 30 days + 48 hours`: returns `GracePeriod { until }`.
/// - Otherwise: returns `Unlicensed`.
pub fn compute_status_from_cache(key: &str, validated_at: u64) -> LicenseStatus {
    if key.is_empty() || validated_at == 0 {
        return LicenseStatus::Unlicensed;
    }

    // Verify the key is genuine before trusting the cache.
    if validate_license_key(key).is_err() {
        return LicenseStatus::Unlicensed;
    }

    let now = current_unix_timestamp();
    let elapsed = now.saturating_sub(validated_at);

    if elapsed <= LICENSED_DURATION_SECS {
        LicenseStatus::Licensed
    } else if elapsed <= LICENSED_DURATION_SECS + GRACE_PERIOD_SECS {
        let until = validated_at + LICENSED_DURATION_SECS + GRACE_PERIOD_SECS;
        LicenseStatus::GracePeriod { until }
    } else {
        LicenseStatus::Unlicensed
    }
}

/// Returns `true` if the given `feature` is accessible under `status`.
///
/// Rules:
/// - `Licensed`: all features allowed.
/// - `GracePeriod { until }`: all features allowed if `until > now`.
/// - `Unlicensed`: no paid features allowed.
pub fn is_feature_allowed(status: &LicenseStatus, _feature: LicensedFeature) -> bool {
    match status {
        LicenseStatus::Licensed => true,
        LicenseStatus::GracePeriod { until } => {
            let now = current_unix_timestamp();
            *until > now
        }
        LicenseStatus::Unlicensed => false,
    }
}

/// Returns a short string identifier for a `LicenseStatus`, suitable for
/// passing back to the frontend via Tauri commands.
///
/// - `"licensed"` for `Licensed`
/// - `"grace_period"` for `GracePeriod { .. }`
/// - `"unlicensed"` for `Unlicensed`
pub fn status_to_string(status: &LicenseStatus) -> String {
    match status {
        LicenseStatus::Licensed => "licensed".to_string(),
        LicenseStatus::GracePeriod { until } => format!("grace_period:{until}"),
        LicenseStatus::Unlicensed => "unlicensed".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the current Unix timestamp in seconds.
///
/// Falls back to 0 on platforms where `SystemTime` is unavailable (should
/// never happen in practice, but avoids a panic).
fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Constant-time byte slice comparison.
///
/// Avoids timing side-channels when comparing HMAC digests.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Test-only key generation
// ---------------------------------------------------------------------------

/// Generates a valid Dikta license key from arbitrary payload bytes.
///
/// Only available in test builds. Uses the same HMAC secret as
/// `validate_license_key` so the generated keys pass validation.
///
/// `payload` must be exactly 6 bytes. Panics otherwise (test-only).
#[cfg(test)]
pub fn generate_license_key(payload: &[u8; 6]) -> String {
    let secret = build_secret();
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret).expect("HMAC init must succeed in tests");
    mac.update(payload);
    let digest = mac.finalize().into_bytes();

    // Combine payload (6 bytes) + truncated HMAC (4 bytes) = 10 bytes.
    let mut raw = [0u8; 10];
    raw[..6].copy_from_slice(payload);
    raw[6..10].copy_from_slice(&digest[..4]);

    // Encode to Base32 (no padding).
    let b32 = base32::encode(base32::Alphabet::RFC4648 { padding: false }, &raw);

    // Split into 4-char groups and prepend "DIKTA-".
    let groups: Vec<&str> = b32
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("Base32 output is always valid UTF-8"))
        .collect();

    format!("DIKTA-{}", groups.join("-"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Key format validation ---

    #[test]
    fn test_valid_key_is_accepted() {
        let key = generate_license_key(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let result = validate_license_key(&key);
        assert!(result.is_ok(), "Generated key must be accepted: {result:?}");
        assert_eq!(result.unwrap(), LicenseStatus::Licensed);
    }

    #[test]
    fn test_empty_key_is_rejected() {
        let result = validate_license_key("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_wrong_prefix_is_rejected() {
        let result = validate_license_key("WRONG-AAAA-BBBB-CCCC-DDDD");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DIKTA"));
    }

    #[test]
    fn test_wrong_segment_length_is_rejected() {
        // Third segment has 3 chars instead of 4.
        let result = validate_license_key("DIKTA-AAAA-BBB-CCCC-DDDD");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_base32_chars_are_rejected() {
        // '0', '1', '8', '9' are not valid RFC 4648 Base32 characters.
        let result = validate_license_key("DIKTA-AAAA-0000-CCCC-DDDD");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid Base32 characters"));
    }

    #[test]
    fn test_hmac_bit_flip_is_rejected() {
        let key = generate_license_key(&[0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45]);
        // Flip the last character of the key string.
        let mut tampered = key.clone();
        let last_char = tampered.pop().unwrap();
        let replacement = if last_char == 'A' { 'B' } else { 'A' };
        tampered.push(replacement);

        let result = validate_license_key(&tampered);
        // May fail at Base32 decode or at HMAC check -- either way it must be Err.
        assert!(result.is_err(), "Tampered key must not be accepted");
    }

    #[test]
    fn test_too_few_segments_is_rejected() {
        let result = validate_license_key("DIKTA-AAAA-BBBB-CCCC");
        assert!(result.is_err());
    }

    #[test]
    fn test_too_many_segments_is_rejected() {
        let result = validate_license_key("DIKTA-AAAA-BBBB-CCCC-DDDD-EEEE");
        assert!(result.is_err());
    }

    // --- compute_status_from_cache ---

    #[test]
    fn test_cache_within_30_days_is_licensed() {
        let key = generate_license_key(&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60]);
        let now = current_unix_timestamp();
        // Validated 1 day ago -- well within 30-day window.
        let validated_at = now - (24 * 60 * 60);
        let status = compute_status_from_cache(&key, validated_at);
        assert_eq!(status, LicenseStatus::Licensed);
    }

    #[test]
    fn test_cache_between_30_and_32_days_is_grace_period() {
        let key = generate_license_key(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        let now = current_unix_timestamp();
        // Validated 31 days ago -- past the 30-day window, within 48h grace.
        let validated_at = now - (31 * 24 * 60 * 60);
        let status = compute_status_from_cache(&key, validated_at);
        assert!(
            matches!(status, LicenseStatus::GracePeriod { .. }),
            "Expected GracePeriod, got {status:?}"
        );
    }

    #[test]
    fn test_cache_older_than_32_days_is_unlicensed() {
        let key = generate_license_key(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let now = current_unix_timestamp();
        // Validated 33 days ago -- past both 30-day + 48h windows.
        let validated_at = now - (33 * 24 * 60 * 60);
        let status = compute_status_from_cache(&key, validated_at);
        assert_eq!(status, LicenseStatus::Unlicensed);
    }

    #[test]
    fn test_cache_zero_timestamp_is_unlicensed() {
        let key = generate_license_key(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let status = compute_status_from_cache(&key, 0);
        assert_eq!(status, LicenseStatus::Unlicensed);
    }

    #[test]
    fn test_cache_empty_key_is_unlicensed() {
        let now = current_unix_timestamp();
        let status = compute_status_from_cache("", now);
        assert_eq!(status, LicenseStatus::Unlicensed);
    }

    #[test]
    fn test_cache_invalid_key_is_unlicensed() {
        let now = current_unix_timestamp();
        let status = compute_status_from_cache("DIKTA-AAAA-AAAA-AAAA-AAAA", now);
        // HMAC won't match, so it must be rejected.
        assert_eq!(status, LicenseStatus::Unlicensed);
    }

    // --- is_feature_allowed ---

    #[test]
    fn test_licensed_allows_all_features() {
        let status = LicenseStatus::Licensed;
        let features = [
            LicensedFeature::AlternativeProviders,
            LicensedFeature::AllCleanupStyles,
            LicensedFeature::CustomPrompts,
            LicensedFeature::CommandMode,
            LicensedFeature::Snippets,
            LicensedFeature::AppProfiles,
            LicensedFeature::UnlimitedHistory,
            LicensedFeature::VoiceNotes,
            LicensedFeature::Sync,
            LicensedFeature::OfflineMode,
            LicensedFeature::WhisperMode,
            LicensedFeature::FillerAnalysis,
            LicensedFeature::CostTracking,
        ];
        for feature in features {
            assert!(
                is_feature_allowed(&status, feature),
                "Licensed must allow {feature:?}"
            );
        }
    }

    #[test]
    fn test_unlicensed_blocks_all_paid_features() {
        let status = LicenseStatus::Unlicensed;
        let features = [
            LicensedFeature::AlternativeProviders,
            LicensedFeature::AllCleanupStyles,
            LicensedFeature::CommandMode,
        ];
        for feature in features {
            assert!(
                !is_feature_allowed(&status, feature),
                "Unlicensed must block {feature:?}"
            );
        }
    }

    #[test]
    fn test_active_grace_period_allows_features() {
        let now = current_unix_timestamp();
        // Grace period expires in 24 hours -- still active.
        let status = LicenseStatus::GracePeriod {
            until: now + 24 * 60 * 60,
        };
        assert!(
            is_feature_allowed(&status, LicensedFeature::UnlimitedHistory),
            "Active grace period must allow paid features"
        );
    }

    #[test]
    fn test_expired_grace_period_blocks_features() {
        // Grace period expired 1 second ago.
        let status = LicenseStatus::GracePeriod { until: 1 };
        assert!(
            !is_feature_allowed(&status, LicensedFeature::Sync),
            "Expired grace period must block paid features"
        );
    }

    // --- status_to_string ---

    #[test]
    fn test_status_to_string_licensed() {
        assert_eq!(status_to_string(&LicenseStatus::Licensed), "licensed");
    }

    #[test]
    fn test_status_to_string_unlicensed() {
        assert_eq!(status_to_string(&LicenseStatus::Unlicensed), "unlicensed");
    }

    #[test]
    fn test_status_to_string_grace_period() {
        let s = status_to_string(&LicenseStatus::GracePeriod { until: 9999 });
        assert!(s.starts_with("grace_period:"));
        assert!(s.contains("9999"));
    }
}
