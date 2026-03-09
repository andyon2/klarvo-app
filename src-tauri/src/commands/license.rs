//! Tauri commands for license key management.
//!
//! Exposes three commands to the frontend:
//! - `validate_license`  -- validate + activate a key
//! - `get_license_status` -- query the current status
//! - `remove_license`    -- deactivate the current key
//!
//! Status strings returned to the frontend:
//! - `"licensed"`
//! - `"grace_period:{unix_timestamp_until}"`
//! - `"unlicensed"`

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::config::save_config;
use crate::license::{
    compute_status_from_cache, status_to_string, validate_license_key, LicenseStatus,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Validates a license key, updates the in-memory status, and persists the
/// key + timestamp to config.json.
///
/// Returns the new status string on success, or an error message on failure.
///
/// # Errors
/// - Returns an error if the key is invalid (bad format or HMAC mismatch).
/// - Returns an error if persisting the config fails.
#[tauri::command]
pub fn validate_license(key: String, state: State<'_, AppState>) -> Result<String, String> {
    // Validate the key cryptographically.
    validate_license_key(&key)?;

    // Record the current timestamp as the validation time.
    let validated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Update the in-memory license status.
    {
        let mut status = crate::lock!(state.license_status)?;
        *status = LicenseStatus::Licensed;
    }

    // Persist key + timestamp to config.
    {
        let inner = state.inner();
        let mut cfg = crate::lock!(inner.config)?;
        cfg.license_key = key;
        cfg.license_validated_at = validated_at;
        let cfg_clone = cfg.clone();
        drop(cfg);
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist license key: {e}"))?;
    }

    Ok("licensed".to_string())
}

/// Returns the current license status as a string.
///
/// The status is read from the in-memory cache (no disk I/O).
/// Possible return values: `"licensed"`, `"grace_period:{timestamp}"`, `"unlicensed"`.
#[tauri::command]
pub fn get_license_status(state: State<'_, AppState>) -> Result<String, String> {
    let status = crate::lock!(state.license_status)?;
    Ok(status_to_string(&status))
}

/// Removes the active license key, setting the status back to `Unlicensed`.
///
/// Clears both the in-memory state and the persisted config fields.
#[tauri::command]
pub fn remove_license(state: State<'_, AppState>) -> Result<(), String> {
    // Clear in-memory status.
    {
        let mut status = crate::lock!(state.license_status)?;
        *status = LicenseStatus::Unlicensed;
    }

    // Clear persisted key + timestamp.
    {
        let inner = state.inner();
        let mut cfg = crate::lock!(inner.config)?;
        cfg.license_key = String::new();
        cfg.license_validated_at = 0;
        let cfg_clone = cfg.clone();
        drop(cfg);
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist license removal: {e}"))?;
    }

    Ok(())
}

/// Recomputes the license status from the persisted config and updates the
/// in-memory state. Useful after the app config has been modified externally
/// or after a sync that might have brought in a new key.
///
/// This is not exposed as a Tauri command but is called internally at startup.
pub fn refresh_license_status_from_config(state: &AppState) -> Result<(), String> {
    let (key, validated_at) = {
        let cfg = crate::lock!(state.config)?;
        (cfg.license_key.clone(), cfg.license_validated_at)
    };

    let new_status = compute_status_from_cache(&key, validated_at);

    let mut status = crate::lock!(state.license_status)?;
    *status = new_status;

    Ok(())
}
