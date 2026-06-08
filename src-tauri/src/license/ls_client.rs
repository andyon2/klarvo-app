//! HTTP client for the Lemon Squeezy License API.
//!
//! Covers three endpoints:
//!   - `/activate`   — register this device against a license key
//!   - `/validate`   — check that an existing activation is still valid
//!   - `/deactivate` — free the activation slot (e.g. on uninstall)
//!
//! All endpoints use `application/x-www-form-urlencoded` POST bodies and
//! return JSON.  The HTTP status code is **not** the source of truth; the
//! `activated` / `valid` / `deactivated` boolean fields in the JSON body are.
//!
//! Reference: <https://docs.lemonsqueezy.com/api/licenses>

use serde::Deserialize;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

#[derive(Error, Debug)]
pub enum LsApiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("License API error: {0}")]
    Api(String),
}

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

/// Successful result of [`activate`].
#[derive(Debug)]
pub struct LsActivateResult {
    /// UUID assigned by Lemon Squeezy for this device activation.
    /// Must be persisted locally; required for [`validate`] and [`deactivate`].
    pub instance_id: String,
    /// How many activations have been used on this license key after this call.
    pub activation_usage: u32,
    /// Maximum number of activations allowed for this license key.
    pub activation_limit: u32,
}

// ---------------------------------------------------------------------------
// Internal deserialisation structs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ActivateResponse {
    activated: bool,
    error: Option<String>,
    instance: Option<InstancePayload>,
    license_key: Option<LicenseKeyPayload>,
}

#[derive(Deserialize)]
struct InstancePayload {
    id: String,
    // `name` is returned by the API but not needed by callers.
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize)]
struct LicenseKeyPayload {
    // Present in activate + validate responses.
    activation_usage: Option<u32>,
    activation_limit: Option<u32>,
    // Non-zero in test-mode activations — we reject these in production builds.
    #[serde(default)]
    test_mode: bool,
}

#[derive(Deserialize)]
struct ValidateResponse {
    valid: bool,
    error: Option<String>,
    license_key: Option<LicenseKeyPayload>,
}

#[derive(Deserialize)]
struct DeactivateResponse {
    deactivated: bool,
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// Base URL + shared client helper
// ---------------------------------------------------------------------------

const LS_BASE_URL: &str = "https://api.lemonsqueezy.com/v1/licenses";

fn user_agent() -> String {
    format!("Klarvo/{}", env!("CARGO_PKG_VERSION"))
}

/// Returns a one-shot `reqwest::Client` with the shared headers that every
/// Lemon Squeezy request needs.
fn build_client() -> Result<reqwest::Client, LsApiError> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json"),
    );
    // `user_agent()` is a runtime String, so we cannot use `from_static`.
    let ua = user_agent();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&ua).unwrap_or_else(|_| HeaderValue::from_static("Klarvo")),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(LsApiError::Network)
}

/// Whether a LemonSqueezy `test_mode` key must be rejected.
///
/// Test-mode keys are issued by LS test mode and must never unlock a public
/// release build, but they stay usable in debug builds so developers can test
/// the activation flow without a paid live key. Pure and side-effect-free so
/// the launch-critical decision is unit-testable independent of the build
/// profile; production wires `is_release = !cfg!(debug_assertions)`.
fn rejects_test_mode(test_mode: bool, is_release: bool) -> bool {
    test_mode && is_release
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Activate a license key for this device.
///
/// On success returns an [`LsActivateResult`] whose `instance_id` **must** be
/// persisted; it is required by [`validate`] and [`deactivate`].
///
/// Returns `Err(LsApiError::Api(_))` when:
/// - the activation limit has been reached (HTTP 422),
/// - the key is invalid / already expired, or
/// - the response contains a test-mode activation (production builds only).
pub async fn activate(license_key: &str, instance_name: &str) -> Result<LsActivateResult, LsApiError> {
    let client = build_client()?;
    let url = format!("{}/activate", LS_BASE_URL);

    let params = [
        ("license_key", license_key),
        ("instance_name", instance_name),
    ];

    let response = client
        .post(&url)
        .form(&params)
        .send()
        .await
        .map_err(LsApiError::Network)?;

    // Always read the body — LS returns JSON even on 4xx.
    let body: ActivateResponse = response
        .json()
        .await
        .map_err(LsApiError::Network)?;

    if !body.activated {
        return Err(LsApiError::Api(
            body.error.unwrap_or_else(|| "Activation failed".to_string()),
        ));
    }

    let instance = body
        .instance
        .ok_or_else(|| LsApiError::Api("Missing instance in activate response".to_string()))?;

    let lk = body
        .license_key
        .ok_or_else(|| LsApiError::Api("Missing license_key in activate response".to_string()))?;

    // Reject test-mode activations in production builds.
    if rejects_test_mode(lk.test_mode, !cfg!(debug_assertions)) {
        return Err(LsApiError::Api(
            "Test-mode license keys are not accepted in production builds".to_string(),
        ));
    }

    Ok(LsActivateResult {
        instance_id: instance.id,
        activation_usage: lk.activation_usage.unwrap_or(0),
        activation_limit: lk.activation_limit.unwrap_or(0),
    })
}

/// Validate an existing activation.
///
/// Returns `Ok(true)` when the license is still active, `Ok(false)` when it
/// has been revoked or the key is no longer valid.  Network errors are
/// propagated as `Err`.
pub async fn validate(license_key: &str, instance_id: &str) -> Result<bool, LsApiError> {
    let client = build_client()?;
    let url = format!("{}/validate", LS_BASE_URL);

    let params = [
        ("license_key", license_key),
        ("instance_id", instance_id),
    ];

    let response = client
        .post(&url)
        .form(&params)
        .send()
        .await
        .map_err(LsApiError::Network)?;

    let body: ValidateResponse = response
        .json()
        .await
        .map_err(LsApiError::Network)?;

    if !body.valid {
        // Surface the API-provided reason in logs but return Ok(false) so
        // callers can distinguish "key invalid" from a network failure.
        if let Some(err) = &body.error {
            log::warn!("LS validate: key invalid — {}", err);
        }
        return Ok(false);
    }

    // Reject test-mode keys in production builds.
    if let Some(lk) = &body.license_key {
        if rejects_test_mode(lk.test_mode, !cfg!(debug_assertions)) {
            return Err(LsApiError::Api(
                "Test-mode license keys are not accepted in production builds".to_string(),
            ));
        }
    }

    Ok(true)
}

/// Deactivate an instance and free the activation slot.
///
/// Call this on uninstall or when the user wants to transfer the license to a
/// different machine.
pub async fn deactivate(license_key: &str, instance_id: &str) -> Result<(), LsApiError> {
    let client = build_client()?;
    let url = format!("{}/deactivate", LS_BASE_URL);

    let params = [
        ("license_key", license_key),
        ("instance_id", instance_id),
    ];

    let response = client
        .post(&url)
        .form(&params)
        .send()
        .await
        .map_err(LsApiError::Network)?;

    let body: DeactivateResponse = response
        .json()
        .await
        .map_err(LsApiError::Network)?;

    if !body.deactivated {
        return Err(LsApiError::Api(
            body.error.unwrap_or_else(|| "Deactivation failed".to_string()),
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- helpers -----------------------------------------------------------

    /// Parse a raw JSON string as `ActivateResponse` and run the same
    /// production logic as `activate()` — including the real `rejects_test_mode`
    /// gate — but without any HTTP call. `is_release` is threaded explicitly
    /// (instead of reading `cfg!(debug_assertions)`) so the release reject path
    /// is exercisable under `cargo test`, which always runs in debug.
    fn parse_activate(json: &str, is_release: bool) -> Result<LsActivateResult, LsApiError> {
        let body: ActivateResponse = serde_json::from_str(json)
            .map_err(|e| LsApiError::Api(e.to_string()))?;

        if !body.activated {
            return Err(LsApiError::Api(
                body.error.unwrap_or_else(|| "Activation failed".to_string()),
            ));
        }

        let instance = body
            .instance
            .ok_or_else(|| LsApiError::Api("Missing instance".to_string()))?;

        let lk = body
            .license_key
            .ok_or_else(|| LsApiError::Api("Missing license_key".to_string()))?;

        if rejects_test_mode(lk.test_mode, is_release) {
            return Err(LsApiError::Api(
                "Test-mode license keys are not accepted in production builds".to_string(),
            ));
        }

        Ok(LsActivateResult {
            instance_id: instance.id,
            activation_usage: lk.activation_usage.unwrap_or(0),
            activation_limit: lk.activation_limit.unwrap_or(0),
        })
    }

    fn parse_validate(json: &str, is_release: bool) -> Result<bool, LsApiError> {
        let body: ValidateResponse = serde_json::from_str(json)
            .map_err(|e| LsApiError::Api(e.to_string()))?;

        if !body.valid {
            return Ok(false);
        }

        if let Some(lk) = &body.license_key {
            if rejects_test_mode(lk.test_mode, is_release) {
                return Err(LsApiError::Api(
                    "Test-mode license keys are not accepted in production builds".to_string(),
                ));
            }
        }
        Ok(true)
    }

    fn parse_deactivate(json: &str) -> Result<(), LsApiError> {
        let body: DeactivateResponse = serde_json::from_str(json)
            .map_err(|e| LsApiError::Api(e.to_string()))?;

        if !body.deactivated {
            return Err(LsApiError::Api(
                body.error.unwrap_or_else(|| "Deactivation failed".to_string()),
            ));
        }
        Ok(())
    }

    // ---- activate ----------------------------------------------------------

    #[test]
    fn test_activate_success() {
        let json = r#"{
            "activated": true,
            "instance": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Andy-Desktop"
            },
            "license_key": {
                "status": "active",
                "activation_usage": 1,
                "activation_limit": 3,
                "test_mode": false
            }
        }"#;

        let result = parse_activate(json, true).expect("activate should succeed");
        assert_eq!(result.instance_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(result.activation_usage, 1);
        assert_eq!(result.activation_limit, 3);
    }

    #[test]
    fn test_activate_limit_reached() {
        // Lemon Squeezy returns HTTP 422 with activated=false and an error message.
        let json = r#"{
            "activated": false,
            "error": "This license key has reached the activation limit.",
            "license_key": {
                "status": "active",
                "activation_usage": 3,
                "activation_limit": 3,
                "test_mode": false
            }
        }"#;

        let err = parse_activate(json, true).expect_err("should fail with limit error");
        match err {
            LsApiError::Api(msg) => assert!(
                msg.contains("activation limit"),
                "unexpected error message: {msg}"
            ),
            other => panic!("expected Api error, got: {other}"),
        }
    }

    // ---- validate ----------------------------------------------------------

    #[test]
    fn test_validate_valid() {
        let json = r#"{
            "valid": true,
            "license_key": {
                "status": "active",
                "test_mode": false
            }
        }"#;

        let result = parse_validate(json, true).expect("validate should succeed");
        assert!(result, "key should be valid");
    }

    #[test]
    fn test_validate_invalid() {
        let json = r#"{
            "valid": false,
            "error": "This license key is invalid.",
            "license_key": null
        }"#;

        let result = parse_validate(json, true).expect("parse should not fail");
        assert!(!result, "key should be invalid");
    }

    // ---- deactivate --------------------------------------------------------

    #[test]
    fn test_deactivate_success() {
        let json = r#"{"deactivated": true}"#;

        parse_deactivate(json).expect("deactivate should succeed");
    }

    #[test]
    fn test_deactivate_failure() {
        let json = r#"{
            "deactivated": false,
            "error": "Instance not found."
        }"#;

        let err = parse_deactivate(json).expect_err("should fail");
        match err {
            LsApiError::Api(msg) => assert!(
                msg.contains("Instance not found"),
                "unexpected error message: {msg}"
            ),
            other => panic!("expected Api error, got: {other}"),
        }
    }

    // ---- test-mode reject (launch gate) ------------------------------------

    #[test]
    fn test_rejects_test_mode_truth_table() {
        // Only a test-mode key in a release build is rejected.
        assert!(rejects_test_mode(true, true), "test-mode + release must reject");
        // Inversion guards: flipping the && to || would turn any of these RED.
        assert!(!rejects_test_mode(false, true), "live key + release must accept");
        assert!(!rejects_test_mode(true, false), "test-mode + debug must accept");
        assert!(!rejects_test_mode(false, false), "live key + debug must accept");
    }

    #[test]
    fn test_activate_rejects_test_mode_in_release() {
        let json = r#"{
            "activated": true,
            "instance": { "id": "id-1", "name": "Andy-Desktop" },
            "license_key": {
                "status": "active",
                "activation_usage": 1,
                "activation_limit": 3,
                "test_mode": true
            }
        }"#;

        let err = parse_activate(json, true).expect_err("test-mode key must be rejected in release");
        match err {
            LsApiError::Api(msg) => assert!(
                msg.contains("Test-mode"),
                "unexpected error message: {msg}"
            ),
            other => panic!("expected Api error, got: {other}"),
        }

        // Same key is accepted in a debug build so devs can test the flow.
        parse_activate(json, false).expect("test-mode key must be accepted in debug");
    }

    #[test]
    fn test_validate_rejects_test_mode_in_release() {
        let json = r#"{
            "valid": true,
            "license_key": {
                "status": "active",
                "test_mode": true
            }
        }"#;

        let err = parse_validate(json, true).expect_err("test-mode key must be rejected in release");
        match err {
            LsApiError::Api(msg) => assert!(
                msg.contains("Test-mode"),
                "unexpected error message: {msg}"
            ),
            other => panic!("expected Api error, got: {other}"),
        }

        // Same key is accepted in a debug build.
        let ok = parse_validate(json, false).expect("test-mode key must be accepted in debug");
        assert!(ok, "debug validate should return true");
    }
}
