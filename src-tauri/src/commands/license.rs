//! Tauri commands for license key management.
//!
//! Exposes four commands to the frontend:
//! - `validate_license`    -- validate + activate a key (HMAC or Lemon Squeezy)
//! - `get_license_status`  -- query the current status
//! - `remove_license`      -- remove the key locally (no API call)
//! - `deactivate_license`  -- deactivate an LS key remotely, then clear locally
//!
//! Status strings returned to the frontend:
//! - `"licensed"`
//! - `"trial:{unix_timestamp_expires}"`
//! - `"grace_period:{unix_timestamp_until}"`
//! - `"unlicensed"`

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::config::save_config;
use crate::license::{
    compute_status_from_cache, compute_status_from_cache_ls, compute_trial_status,
    ls_client, status_to_string, validate_key_dual_path, LicenseStatus, ValidateResult,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Validates a license key (HMAC or Lemon Squeezy), updates the in-memory
/// status, and persists the key + metadata to config.json.
///
/// Returns the new status string on success, or an error message on failure.
///
/// # Errors
/// - Returns an error if the key is invalid (bad format, HMAC mismatch, or LS API error).
/// - Returns an error if persisting the config fails.
#[tauri::command]
pub async fn validate_license(key: String, state: State<'_, AppState>) -> Result<String, String> {
    let instance_name = {
        let cfg = crate::lock!(state.config)?;
        cfg.device_id.clone()
    };

    let result = validate_key_dual_path(&key, &instance_name).await?;

    let validated_at = current_timestamp();

    match result {
        ValidateResult::Hmac(status) => {
            // Update in-memory status.
            {
                let mut s = crate::lock!(state.license_status)?;
                *s = status.clone();
            }
            // Persist to config.
            {
                let inner = state.inner();
                let mut cfg = crate::lock!(inner.config)?;
                cfg.license_key = key;
                cfg.license_validated_at = validated_at;
                cfg.license_source = "hmac".to_string();
                cfg.ls_instance_id = String::new();
                cfg.ls_last_validated_at = 0;
                let cfg_clone = cfg.clone();
                drop(cfg);
                save_config(&inner.app_data_dir, &cfg_clone)
                    .map_err(|e| format!("Failed to persist license: {e}"))?;
            }
            Ok(status_to_string(&status))
        }
        ValidateResult::LemonSqueezy { status, instance_id } => {
            // Update in-memory status.
            {
                let mut s = crate::lock!(state.license_status)?;
                *s = status.clone();
            }
            // Persist to config.
            {
                let inner = state.inner();
                let mut cfg = crate::lock!(inner.config)?;
                cfg.license_key = key;
                cfg.license_validated_at = validated_at;
                cfg.license_source = "lemon_squeezy".to_string();
                cfg.ls_instance_id = instance_id;
                cfg.ls_last_validated_at = validated_at;
                let cfg_clone = cfg.clone();
                drop(cfg);
                save_config(&inner.app_data_dir, &cfg_clone)
                    .map_err(|e| format!("Failed to persist license: {e}"))?;
            }
            Ok(status_to_string(&status))
        }
    }
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

/// Removes the active license key locally, reverting to trial (if still within
/// the 14-day window) or unlicensed.
///
/// Clears both the in-memory state and the persisted config fields.
/// For Lemon Squeezy keys this does NOT call the deactivation API -- use
/// `deactivate_license` to also free the device slot.
#[tauri::command]
pub fn remove_license(state: State<'_, AppState>) -> Result<(), String> {
    // Read first_install_at before locking license_status (consistent lock order).
    let first_install_at = {
        let cfg = crate::lock!(state.config)?;
        cfg.first_install_at
    };

    // Recompute status: may still be in trial period.
    {
        let mut status = crate::lock!(state.license_status)?;
        *status = compute_trial_status(first_install_at);
    }

    // Clear persisted key + all license fields.
    {
        let inner = state.inner();
        let mut cfg = crate::lock!(inner.config)?;
        cfg.license_key = String::new();
        cfg.license_validated_at = 0;
        cfg.license_source = String::new();
        cfg.ls_instance_id = String::new();
        cfg.ls_last_validated_at = 0;
        let cfg_clone = cfg.clone();
        drop(cfg);
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist license removal: {e}"))?;
    }

    Ok(())
}

/// Deactivates the current LS license (frees the device slot), then clears
/// local state. For HMAC keys, behaves like `remove_license` (no API call).
#[tauri::command]
pub async fn deactivate_license(state: State<'_, AppState>) -> Result<(), String> {
    let (key, source, instance_id) = {
        let cfg = crate::lock!(state.config)?;
        (
            cfg.license_key.clone(),
            cfg.license_source.clone(),
            cfg.ls_instance_id.clone(),
        )
    };

    // For LS keys: call deactivate API to free the activation slot.
    if source == "lemon_squeezy" && !instance_id.is_empty() {
        ls_client::deactivate(&key, &instance_id)
            .await
            .map_err(|e| format!("Failed to deactivate license: {e}"))?;
    }

    // Read first_install_at before locking license_status.
    let first_install_at = {
        let cfg = crate::lock!(state.config)?;
        cfg.first_install_at
    };

    // Recompute status: may still be in trial period.
    {
        let mut status = crate::lock!(state.license_status)?;
        *status = compute_trial_status(first_install_at);
    }
    {
        let inner = state.inner();
        let mut cfg = crate::lock!(inner.config)?;
        cfg.license_key = String::new();
        cfg.license_validated_at = 0;
        cfg.license_source = String::new();
        cfg.ls_instance_id = String::new();
        cfg.ls_last_validated_at = 0;
        let cfg_clone = cfg.clone();
        drop(cfg);
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist deactivation: {e}"))?;
    }

    Ok(())
}

/// Returns the license source string stored in config: "hmac", "lemon_squeezy", or "".
///
/// Used by the frontend to decide whether to show the "Deactivate (free slot)"
/// button (Lemon Squeezy only) or just the plain "Remove" button (HMAC).
#[tauri::command]
pub fn get_license_source(state: State<'_, AppState>) -> Result<String, String> {
    let cfg = crate::lock!(state.config)?;
    Ok(cfg.license_source.clone())
}

/// Recomputes the license status from the persisted config and updates the
/// in-memory state. Useful after the app config has been modified externally
/// or after a sync that might have brought in a new key.
///
/// Handles both HMAC keys (offline check) and Lemon Squeezy keys (cache-based
/// check using the last validated timestamp).
///
/// This is not exposed as a Tauri command but is called internally at startup.
#[allow(dead_code)] // reserved for startup sync and future external config reload
pub fn refresh_license_status_from_config(state: &AppState) -> Result<(), String> {
    let (key, source, validated_at, ls_instance_id, ls_last_validated_at) = {
        let cfg = crate::lock!(state.config)?;
        (
            cfg.license_key.clone(),
            cfg.license_source.clone(),
            cfg.license_validated_at,
            cfg.ls_instance_id.clone(),
            cfg.ls_last_validated_at,
        )
    };

    let new_status = if source == "lemon_squeezy" {
        compute_status_from_cache_ls(&ls_instance_id, ls_last_validated_at)
    } else {
        compute_status_from_cache(&key, validated_at)
    };

    let mut status = crate::lock!(state.license_status)?;
    *status = new_status;

    Ok(())
}
