//! License validation module for the Voxlit open-core monetization model.
//!
//! ## Key format
//!
//! `VOXLIT-XXXX-XXXX-XXXX-XXXX`
//!
//! The payload is Base32-encoded (RFC 4648, no padding) with groups of 4
//! characters separated by hyphens.  Decoded it is 10 bytes:
//! - bytes 0..6 : 6-byte payload
//! - bytes 6..10: first 4 bytes of HMAC-SHA256(secret, payload[0..6])
//!
//! ## Payload layout
//!
//! Byte 0 encodes the key type:
//! - `0x00` = permanent (regular purchase)
//! - `0x01` = trial/tester (has an expiry date)
//! - Any other value = permanent (backward-compatibility for legacy keys)
//!
//! For trial keys (byte 0 == `0x01`):
//! - bytes 1-2: expiry as `u16` big-endian (days since 2025-01-01)
//! - bytes 3-5: 3-byte identifier (e.g. tester initials or a serial number)
//!
//! Legacy keys (e.g. `b"andyon"`) have byte 0 != `0x00` and != `0x01`.
//! They are treated as permanent.
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
const SECRET_PART_A: &[u8] = b"voxlit-license-v1";
/// Second half of the embedded HMAC secret.
const SECRET_PART_B: &[u8] = b"-2025-open-core!";

/// Legacy HMAC secret from before the Dikta → Voxlit rename.
/// Keys issued under the old name use this secret for HMAC verification.
const LEGACY_SECRET_PART_A: &[u8] = b"dikta-license-v1";

/// Combines the two secret parts into a single key used for HMAC operations.
/// The result is dropped after use -- not stored as a static.
fn build_secret() -> Vec<u8> {
    let mut s = Vec::with_capacity(SECRET_PART_A.len() + SECRET_PART_B.len());
    s.extend_from_slice(SECRET_PART_A);
    s.extend_from_slice(SECRET_PART_B);
    s
}

/// Builds the legacy HMAC secret for Dikta-era keys.
fn build_legacy_secret() -> Vec<u8> {
    let mut s = Vec::with_capacity(LEGACY_SECRET_PART_A.len() + SECRET_PART_B.len());
    s.extend_from_slice(LEGACY_SECRET_PART_A);
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

/// Unix timestamp of the trial epoch: 2025-01-01T00:00:00Z.
///
/// Trial expiry dates are encoded as days since this date to fit into 2 bytes
/// (max representable date: 2025-01-01 + 65535 days ≈ year 2204).
const TRIAL_EPOCH_SECS: u64 = 1_735_689_600; // 2025-01-01 00:00:00 UTC

/// Number of seconds in one day, used for day-offset arithmetic.
const SECS_PER_DAY: u64 = 24 * 60 * 60;

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
    /// Trial / tester key. All paid features are unlocked until `until`
    /// (Unix timestamp of the expiry date at midnight UTC).
    Trial { until: u64 },
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
    /// All cleanup styles (Polished, Verbatim, Chat) are now free.
    /// Kept as a variant for backward compatibility with any serialized data.
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
    /// Offline transcription via whisper.cpp: medium and large-v3 models require paid license.
    /// Only small model is free (no gate).
    OfflineMode,
    /// Whisper Mode: amplified mic for quiet dictation.
    WhisperMode,
    /// Filler-word frequency analysis.
    FillerAnalysis,
    /// API cost tracking dashboard.
    CostTracking,
    /// Dictionary entries beyond the free-tier limit of 20 terms.
    UnlimitedDictionary,
}

// ---------------------------------------------------------------------------
// Key validation
// ---------------------------------------------------------------------------

/// Validates a license key string.
///
/// Returns `Ok(LicenseStatus::Licensed)` for permanent keys and
/// `Ok(LicenseStatus::Trial { until })` for valid, non-expired trial keys.
/// Returns `Err` with a human-readable message for invalid or expired keys.
///
/// This does NOT check the cache timestamp -- call `compute_status_from_cache`
/// for that. This function only answers: "is this a genuine Voxlit key, and if
/// it is a trial key, has it expired?"
pub fn validate_license_key(key: &str) -> Result<LicenseStatus, String> {
    if key.is_empty() {
        return Err("License key is empty".to_string());
    }

    // Expected format: VOXLIT-XXXX-XXXX-XXXX-XXXX
    // 5 segments separated by '-', first is "VOXLIT", rest are 4-char Base32 groups.
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 5 {
        return Err(format!(
            "Invalid key format: expected 5 dash-separated segments, got {}",
            parts.len()
        ));
    }

    // Accept both "VOXLIT" and legacy "DIKTA" prefixes so keys issued
    // before the rename continue to work.
    if parts[0] != "VOXLIT" && parts[0] != "DIKTA" {
        return Err("Invalid key format: must start with 'VOXLIT' or 'DIKTA'".to_string());
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

    // Verify HMAC. Try the current secret first; if the key has a legacy
    // "DIKTA" prefix, also try the legacy secret so old keys keep working.
    let is_legacy = parts[0] == "DIKTA";
    let secrets_to_try: Vec<Vec<u8>> = if is_legacy {
        vec![build_legacy_secret(), build_secret()]
    } else {
        vec![build_secret()]
    };

    let mut hmac_ok = false;
    for secret in &secrets_to_try {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret)
            .map_err(|e| format!("HMAC init error: {e}"))?;
        mac.update(payload);
        let computed = mac.finalize().into_bytes();
        if constant_time_eq(&computed[..4], stored_hmac) {
            hmac_ok = true;
            break;
        }
    }

    if !hmac_ok {
        return Err("Invalid key: HMAC verification failed".to_string());
    }

    // Inspect byte 0 to determine key type.
    match payload[0] {
        0x01 => {
            // Trial key: bytes 1-2 are days since TRIAL_EPOCH (big-endian u16).
            let days_since_epoch = u16::from_be_bytes([payload[1], payload[2]]) as u64;
            let expiry_secs = TRIAL_EPOCH_SECS + days_since_epoch * SECS_PER_DAY;

            let now = current_unix_timestamp();
            if now >= expiry_secs {
                // Convert expiry to a human-readable date for the error message.
                let expiry_date = unix_secs_to_date_string(expiry_secs);
                return Err(format!("Trial license expired on {expiry_date}"));
            }

            Ok(LicenseStatus::Trial { until: expiry_secs })
        }
        // 0x00 = permanent, anything else = legacy key (backward-compat).
        _ => Ok(LicenseStatus::Licensed),
    }
}

/// Computes the license status from a cached key and the timestamp at which
/// it was last validated.
///
/// - If `key` is empty: returns `Unlicensed`.
/// - If `key` is invalid (bad HMAC): returns `Unlicensed`.
/// - If `key` is an expired trial: returns `Unlicensed` (ignores cache).
/// - If `validated_at == 0`: returns `Unlicensed`.
/// - For trial keys: returns `Trial { until }` if not expired.
/// - For permanent keys:
///   - If `now - validated_at <= 30 days`: returns `Licensed`.
///   - If `now - validated_at <= 30 days + 48 hours`: returns `GracePeriod { until }`.
///   - Otherwise: returns `Unlicensed`.
pub fn compute_status_from_cache(key: &str, validated_at: u64) -> LicenseStatus {
    if key.is_empty() || validated_at == 0 {
        return LicenseStatus::Unlicensed;
    }

    // Verify the key is genuine before trusting the cache.
    // For trial keys, validate_license_key also checks expiry.
    let validated = match validate_license_key(key) {
        Ok(status) => status,
        Err(_) => return LicenseStatus::Unlicensed,
    };

    // Trial keys: the embedded expiry date always wins over the cache timestamp.
    if let LicenseStatus::Trial { until } = validated {
        let now = current_unix_timestamp();
        if now >= until {
            return LicenseStatus::Unlicensed;
        }
        return LicenseStatus::Trial { until };
    }

    // Permanent keys: apply the 30-day + 48h grace window against validated_at.
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
/// - `Trial { until }`: all features allowed if `until > now`.
/// - `GracePeriod { until }`: all features allowed if `until > now`.
/// - `Unlicensed`: no paid features allowed.
pub fn is_feature_allowed(status: &LicenseStatus, _feature: LicensedFeature) -> bool {
    let now = current_unix_timestamp();
    match status {
        LicenseStatus::Licensed => true,
        LicenseStatus::Trial { until } => *until > now,
        LicenseStatus::GracePeriod { until } => *until > now,
        LicenseStatus::Unlicensed => false,
    }
}

/// Returns a short string identifier for a `LicenseStatus`, suitable for
/// passing back to the frontend via Tauri commands.
///
/// - `"licensed"` for `Licensed`
/// - `"trial:<until>"` for `Trial { until }`
/// - `"grace_period:<until>"` for `GracePeriod { .. }`
/// - `"unlicensed"` for `Unlicensed`
pub fn status_to_string(status: &LicenseStatus) -> String {
    match status {
        LicenseStatus::Licensed => "licensed".to_string(),
        LicenseStatus::Trial { until } => format!("trial:{until}"),
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

/// Converts a Unix timestamp (seconds) to a `"YYYY-MM-DD"` string.
///
/// Purely arithmetic -- no external date crate required.
/// Handles dates from 1970 through ~2200 correctly for our use case.
fn unix_secs_to_date_string(secs: u64) -> String {
    let days_since_epoch = secs / SECS_PER_DAY;
    // Gregorian calendar calculation (Tomohiko Sakamoto algorithm variant).
    let z = days_since_epoch as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------
// Test-only key generation
// ---------------------------------------------------------------------------

/// Generates a valid Voxlit license key from arbitrary payload bytes.
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

    // Split into 4-char groups and prepend "VOXLIT-".
    let groups: Vec<&str> = b32
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("Base32 output is always valid UTF-8"))
        .collect();

    format!("VOXLIT-{}", groups.join("-"))
}

/// Generates a trial license key with an embedded expiry date.
///
/// Only available in test builds.
///
/// - `identifier`: 3-byte tester identifier (e.g. `b"ts1"`).
/// - `days_valid`: number of days from today until the key expires.
///
/// Payload layout:
/// - byte 0: `0x01` (trial key type)
/// - bytes 1-2: (today + days_valid) as days since 2025-01-01, big-endian u16
/// - bytes 3-5: identifier
#[cfg(test)]
pub fn generate_trial_key(identifier: &[u8; 3], days_valid: u16) -> String {
    let now = current_unix_timestamp();
    // Compute how many days from TRIAL_EPOCH to (now + days_valid).
    let expiry_secs = now + days_valid as u64 * SECS_PER_DAY;
    // Clamp to TRIAL_EPOCH to avoid underflow on systems with clock issues.
    let days_since_epoch = expiry_secs.saturating_sub(TRIAL_EPOCH_SECS) / SECS_PER_DAY;
    // Saturate to u16::MAX (year ~2204) -- more than enough.
    let days_u16 = days_since_epoch.min(u16::MAX as u64) as u16;

    let mut payload = [0u8; 6];
    payload[0] = 0x01;
    payload[1] = (days_u16 >> 8) as u8;
    payload[2] = (days_u16 & 0xFF) as u8;
    payload[3] = identifier[0];
    payload[4] = identifier[1];
    payload[5] = identifier[2];

    generate_license_key(&payload)
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
        // Use byte 0 = 0x02 (permanent, non-trial) to avoid the trial branch.
        let key = generate_license_key(&[0x02, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let result = validate_license_key(&key);
        assert!(result.is_ok(), "Generated key must be accepted: {result:?}");
        assert_eq!(result.unwrap(), LicenseStatus::Licensed);
    }

    /// Prints a dev license key to stdout. Run with:
    /// `cargo test --lib license::tests::print_dev_key -- --nocapture`
    #[test]
    fn print_dev_key() {
        let key = generate_license_key(b"andyon");
        println!("\n=== DEV LICENSE KEY ===\n{key}\n=======================\n");
    }

    /// Generates a legacy DIKTA-prefixed key using the old HMAC secret.
    fn generate_legacy_dikta_key(payload: &[u8; 6]) -> String {
        let secret = build_legacy_secret();
        let mut mac = Hmac::<Sha256>::new_from_slice(&secret)
            .expect("HMAC init must succeed in tests");
        mac.update(payload);
        let digest = mac.finalize().into_bytes();

        let mut raw = [0u8; 10];
        raw[..6].copy_from_slice(payload);
        raw[6..10].copy_from_slice(&digest[..4]);

        let b32 = base32::encode(base32::Alphabet::RFC4648 { padding: false }, &raw);
        let groups: Vec<&str> = b32
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).expect("valid UTF-8"))
            .collect();

        format!("DIKTA-{}", groups.join("-"))
    }

    #[test]
    fn test_legacy_dikta_key_is_accepted() {
        let key = generate_legacy_dikta_key(b"andyon");
        let result = validate_license_key(&key);
        assert!(result.is_ok(), "Legacy DIKTA key must be accepted: {result:?}");
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
        assert!(result.unwrap_err().contains("VOXLIT"));
    }

    #[test]
    fn test_wrong_segment_length_is_rejected() {
        // Third segment has 3 chars instead of 4.
        let result = validate_license_key("VOXLIT-AAAA-BBB-CCCC-DDDD");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_base32_chars_are_rejected() {
        // '0', '1', '8', '9' are not valid RFC 4648 Base32 characters.
        let result = validate_license_key("VOXLIT-AAAA-0000-CCCC-DDDD");
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
        let result = validate_license_key("VOXLIT-AAAA-BBBB-CCCC");
        assert!(result.is_err());
    }

    #[test]
    fn test_too_many_segments_is_rejected() {
        let result = validate_license_key("VOXLIT-AAAA-BBBB-CCCC-DDDD-EEEE");
        assert!(result.is_err());
    }

    // --- Trial key tests ---

    #[test]
    fn test_trial_key_valid() {
        let key = generate_trial_key(b"ts1", 90);
        let result = validate_license_key(&key);
        assert!(result.is_ok(), "90-day trial key must be accepted: {result:?}");
        match result.unwrap() {
            LicenseStatus::Trial { until } => {
                let now = current_unix_timestamp();
                // Expiry should be roughly 90 days from now (allow 1-day tolerance).
                assert!(until > now, "Trial expiry must be in the future");
                assert!(
                    until <= now + 91 * SECS_PER_DAY,
                    "Trial expiry must not exceed 91 days from now"
                );
            }
            other => panic!("Expected Trial status, got {other:?}"),
        }
    }

    #[test]
    fn test_trial_key_expired() {
        // Build a trial key that expired yesterday by manually crafting the payload.
        // days_valid = 0 means expiry = today (start of day), which may or may not
        // have elapsed. To be safe, use a fixed past date: 1 day since epoch = 2025-01-02.
        let past_days: u16 = 1; // 2025-01-02 -- long in the past.
        let mut payload = [0u8; 6];
        payload[0] = 0x01;
        payload[1] = (past_days >> 8) as u8;
        payload[2] = (past_days & 0xFF) as u8;
        payload[3] = b't';
        payload[4] = b'x';
        payload[5] = b'p';
        let key = generate_license_key(&payload);

        let result = validate_license_key(&key);
        assert!(result.is_err(), "Expired trial key must be rejected");
        let err = result.unwrap_err();
        assert!(
            err.contains("expired"),
            "Error message must mention 'expired': {err}"
        );
        assert!(
            err.contains("2025"),
            "Error message must contain the expiry year: {err}"
        );
    }

    #[test]
    fn test_permanent_key_still_works() {
        // Andy's existing key -- payload b"andyon" = [0x61, 0x6e, 0x64, 0x79, 0x6f, 0x6e].
        // Byte 0 is 0x61 ('a'), which is neither 0x00 nor 0x01 -> treated as permanent.
        let key = generate_license_key(b"andyon");
        let result = validate_license_key(&key);
        assert!(result.is_ok(), "Andy's key must still be accepted: {result:?}");
        assert_eq!(
            result.unwrap(),
            LicenseStatus::Licensed,
            "Andy's key must yield Licensed (permanent)"
        );
    }

    /// Prints tester keys to stdout. Run with:
    /// `cargo test --lib license::tests::print_tester_keys -- --nocapture`
    #[test]
    fn print_tester_keys() {
        let testers: [(&[u8; 3], &str); 3] = [
            (b"ts1", "Tester 1"),
            (b"ts2", "Tester 2"),
            (b"ts3", "Tester 3"),
        ];
        println!("\n=== TESTER KEYS (90 days) ===");
        for (id, name) in testers {
            let key = generate_trial_key(id, 90);
            println!("{name}: {key}");
        }
        println!("=============================\n");
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
        let status = compute_status_from_cache("VOXLIT-AAAA-AAAA-AAAA-AAAA", now);
        // HMAC won't match, so it must be rejected.
        assert_eq!(status, LicenseStatus::Unlicensed);
    }

    #[test]
    fn test_trial_key_cache_returns_trial_status() {
        let key = generate_trial_key(b"ts1", 90);
        let now = current_unix_timestamp();
        let status = compute_status_from_cache(&key, now);
        assert!(
            matches!(status, LicenseStatus::Trial { .. }),
            "Valid trial key in cache must yield Trial status, got {status:?}"
        );
    }

    #[test]
    fn test_expired_trial_key_cache_returns_unlicensed() {
        // Craft a trial key with a past expiry date.
        let past_days: u16 = 1;
        let mut payload = [0u8; 6];
        payload[0] = 0x01;
        payload[1] = (past_days >> 8) as u8;
        payload[2] = (past_days & 0xFF) as u8;
        payload[3] = b'e';
        payload[4] = b'x';
        payload[5] = b'p';
        let key = generate_license_key(&payload);

        let now = current_unix_timestamp();
        // Even with a recent validated_at, an expired trial must be Unlicensed.
        let status = compute_status_from_cache(&key, now);
        assert_eq!(
            status,
            LicenseStatus::Unlicensed,
            "Expired trial key must yield Unlicensed even with fresh cache timestamp"
        );
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
    fn test_trial_allows_all_features() {
        let now = current_unix_timestamp();
        // Trial expires in 24 hours -- still active.
        let status = LicenseStatus::Trial {
            until: now + 24 * 60 * 60,
        };
        let features = [
            LicensedFeature::AlternativeProviders,
            LicensedFeature::AllCleanupStyles,
            LicensedFeature::OfflineMode,
        ];
        for feature in features {
            assert!(
                is_feature_allowed(&status, feature),
                "Active trial must allow {feature:?}"
            );
        }
    }

    #[test]
    fn test_expired_trial_blocks_features() {
        // Trial expired 1 second ago.
        let status = LicenseStatus::Trial { until: 1 };
        assert!(
            !is_feature_allowed(&status, LicensedFeature::UnlimitedHistory),
            "Expired trial must block paid features"
        );
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

    #[test]
    fn test_status_to_string_trial() {
        let s = status_to_string(&LicenseStatus::Trial { until: 9999 });
        assert!(s.starts_with("trial:"));
        assert!(s.contains("9999"));
    }

    // --- unix_secs_to_date_string ---

    #[test]
    fn test_date_string_epoch() {
        // 1970-01-01
        assert_eq!(unix_secs_to_date_string(0), "1970-01-01");
    }

    #[test]
    fn test_date_string_trial_epoch() {
        // TRIAL_EPOCH_SECS should be 2025-01-01
        assert_eq!(unix_secs_to_date_string(TRIAL_EPOCH_SECS), "2025-01-01");
    }

    #[test]
    fn test_date_string_known_date() {
        // 2026-03-11 00:00:00 UTC = 1773187200
        // Verified with: date -d "2026-03-11 00:00:00 UTC" +%s
        assert_eq!(unix_secs_to_date_string(1_773_187_200), "2026-03-11");
    }
}
