//! Tauri commands for Voice Command Mode (always-on background monitor).
//!
//! These commands are desktop-only -- the VAD-based monitor depends on
//! `cpal` continuous capture which is wired up only on desktop targets.

#[cfg(desktop)]
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(desktop)]
use crate::AppState;
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
            if let Err(e) = inner.save_config_locked("voice command state", |cfg| {
                cfg.voice_command_enabled = false;
            }) {
                log::warn!(
                    "[voice_command_cmd] skipped persisting voice_command_enabled=false: {e} \
                     (auto-start preference may be stale)"
                );
            }
        }

        // Cancel any active recording that was started by a voice command.
        // If we don't do this, the recording keeps running after the monitor
        // is stopped, leaving orphaned state that breaks the next toggle cycle.
        if state.recorder.is_recording() {
            log::info!("[voice_command_cmd] Cancelling active recording before stopping monitor");
            let _ = state.recorder.stop_recording();
            if let Ok(mut guard) = state.recording_start.lock() {
                *guard = None;
            }
            // Also clear any auto-loop flag.
            state.auto_loop_active.store(false, Ordering::SeqCst);
            let _ = app.emit(
                crate::hotkey::EVENT_STATE_CHANGED,
                crate::hotkey::PipelineEvent::idle(),
            );
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
        state.inner().save_config_locked("voice command state", |cfg| {
            cfg.voice_command_enabled = true;
        })?;

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
