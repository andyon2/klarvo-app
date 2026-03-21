//! Tauri commands for Voice Command Mode (always-on background monitor).
//!
//! These commands are desktop-only -- the VAD-based monitor depends on
//! `cpal` continuous capture which is wired up only on desktop targets.

#[cfg(desktop)]
use tauri::{AppHandle, Manager, State};

#[cfg(desktop)]
use crate::config::save_config;
#[cfg(desktop)]
use crate::{AppState, lock};
#[cfg(desktop)]
use crate::voice_command::{start_voice_command_monitor, stop_voice_command_monitor};

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Toggles Voice Command Mode on or off.
///
/// - If the monitor is currently inactive: starts it and sets
///   `config.voice_command_enabled = true`, then persists the config.
/// - If the monitor is currently active: stops it and sets
///   `config.voice_command_enabled = false`, then persists the config.
///
/// Returns the **new** active state (`true` = now running).
#[cfg(desktop)]
#[tauri::command]
pub fn toggle_voice_command_mode(app: AppHandle) -> Result<bool, String> {
    use std::sync::atomic::Ordering;

    let state = app.state::<AppState>();
    let runtime_active = state.voice_command_active.load(Ordering::SeqCst);

    // Also check the config preference -- if the UI shows "on" but the
    // runtime is "off" (e.g. auto-start failed), the user still expects
    // a click to turn it OFF, not to start it.
    let config_enabled = state
        .config
        .lock()
        .ok()
        .map(|c| c.voice_command_enabled)
        .unwrap_or(false);

    let should_stop = runtime_active || config_enabled;

    if should_stop {
        // Always persist "disabled" FIRST, so even if stop_monitor fails
        // (or the app is hard-killed), the next launch won't auto-start.
        {
            let inner = state.inner();
            if let Ok(mut cfg) = lock!(inner.config) {
                cfg.voice_command_enabled = false;
                let _ = save_config(&inner.app_data_dir, &cfg);
            }
        }

        // Stop the monitor if it's actually running. Log errors but don't
        // fail the toggle -- the user wants it OFF regardless.
        if runtime_active {
            if let Err(e) = stop_voice_command_monitor(&app) {
                log::warn!("[voice_command_cmd] stop_monitor error (forcing off): {e}");
            }
        }

        // Force the flag off regardless.
        state.voice_command_active.store(false, Ordering::SeqCst);

        log::info!("[voice_command_cmd] Monitor stopped, preference saved as disabled");
        Ok(false)
    } else {
        // Start the monitor.
        start_voice_command_monitor(&app)?;

        // Persist: user turned it on.
        let inner = state.inner();
        let mut cfg = lock!(inner.config)?;
        cfg.voice_command_enabled = true;
        save_config(&inner.app_data_dir, &cfg)
            .map_err(|e| format!("Failed to save config: {e}"))?;

        log::info!("[voice_command_cmd] Monitor started, preference saved as enabled");
        Ok(true)
    }
}

/// Returns whether the Voice Command Mode monitor is currently running.
///
/// Reads the `voice_command_active` `AtomicBool` from `AppState` -- this
/// reflects the live runtime state (not the persisted preference).
#[cfg(desktop)]
#[tauri::command]
pub fn get_voice_command_active(state: State<'_, AppState>) -> bool {
    use std::sync::atomic::Ordering;
    state.voice_command_active.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[cfg(desktop)]
    #[test]
    fn test_get_voice_command_active_initial_false() {
        use crate::test_helpers::{make_state, temp_dir};
        use tauri::State;

        let dir = temp_dir();
        let app_state = make_state(&dir);

        // AtomicBool starts as false -- monitor is not running on fresh state.
        assert!(!app_state.voice_command_active.load(Ordering::SeqCst));
    }

    #[cfg(desktop)]
    #[test]
    fn test_voice_command_active_flag_roundtrip() {
        use crate::test_helpers::{make_state, temp_dir};
        use std::sync::atomic::Ordering;

        let dir = temp_dir();
        let app_state = make_state(&dir);

        // Simulate the monitor being active.
        app_state.voice_command_active.store(true, Ordering::SeqCst);
        assert!(app_state.voice_command_active.load(Ordering::SeqCst));

        // Simulate the monitor being stopped.
        app_state.voice_command_active.store(false, Ordering::SeqCst);
        assert!(!app_state.voice_command_active.load(Ordering::SeqCst));
    }
}
