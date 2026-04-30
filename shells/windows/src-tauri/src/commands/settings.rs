//! Settings Tauri-Command surface (Story 2.A.A4 AC-6/7/8/9/10).
//!
//! - 8 Commands registered in `specta_builder()` via `collect_commands!`.
//! - `UserSettings` + `SettingsChangedEvent` exported to TS by tauri-specta.
//! - `TauriSettingsEmitter` implements `klarvo_core::settings::SettingsEmitter`;
//!   lives here so `klarvo-core` has no Tauri dependency (ADR-0009 Hybrid-C analog).

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Emitter as _;
use tauri_specta::Event;

use klarvo_core::error::AppError;
use klarvo_core::recording::RecordingMode;
use klarvo_core::settings::Settings;

// ---------------------------------------------------------------------------
// Shared payload types (tauri-specta exported)
// ---------------------------------------------------------------------------

/// Bulk-read projection of all user-configurable Core-Settings (`get_user_settings` return type).
///
/// Shell-side type — aggregates typed accessor returns for a single IPC round-trip.
/// Lives in the shell, not in `klarvo-core` (tauri-specta concern; not a Core domain type).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub hotkey_slot1_combo: String,
    pub output_target_id: String,
    pub ui_language: String,
    pub dictionary_language: String,
    pub output_language: String,
    /// Serialised RecordingMode string (e.g. `"hold"`, `"toggle"`, `"autostop"`, `"wait_and_type"`).
    pub hotkey_slot1_mode: String,
}

/// Event payload emitted on every successful settings write (AC-5 + AC-6).
///
/// Frontend listeners (A8-Sub, C2, C3) subscribe to `"settings.changed"`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "settings.changed")]
pub struct SettingsChangedEvent {
    pub key: String,
    pub new_value: String,
}

// ---------------------------------------------------------------------------
// TauriSettingsEmitter (AC-5)
// ---------------------------------------------------------------------------

/// Shell implementation of `klarvo_core::settings::SettingsEmitter`.
///
/// Calls `app_handle.emit("settings.changed", ...)` on every settings write.
/// Generic over `R: tauri::Runtime` for MockRuntime compatibility in tests.
pub struct TauriSettingsEmitter<R: tauri::Runtime> {
    app_handle: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> TauriSettingsEmitter<R> {
    pub fn new(handle: tauri::AppHandle<R>) -> Self {
        Self { app_handle: handle }
    }
}

impl<R: tauri::Runtime> klarvo_core::settings::SettingsEmitter for TauriSettingsEmitter<R> {
    fn emit_settings_changed(&self, key: &str, new_value: &str) {
        let event = SettingsChangedEvent { key: key.into(), new_value: new_value.into() };
        if let Err(e) = self.app_handle.emit("settings.changed", &event) {
            tracing::warn!(error = %e, key, "failed to emit settings.changed event");
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands (AC-6 / AC-7)
// ---------------------------------------------------------------------------

// --- Core-Set (5 commands) ---

/// Set hotkey slot-1 combo with Win32 conflict pre-validation (Story 2.A.C2 + Code-Review patches).
///
/// Flow:
///   1. Read old combo (P5: explicit match — surfaces DB read errors instead of silent "").
///   2. Skip-if-equal fast-path (P10/D1) — idempotent re-save returns Ok without probing.
///   3. Win32 grammar gate + unregister-old + probe + recovery via
///      `validate_hotkey_not_conflicting` (P10/P11).
///   4. Settings-Write + `settings.changed` event via emitter.
///   5. Register new shortcut + recovery to old on failure via `reregister_hotkey` (P12).
///
/// Steps 3 and 5 are Windows-only; on other targets the combo is written directly.
#[tauri::command]
#[specta::specta]
pub async fn set_hotkey_slot1(
    #[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
    app: tauri::AppHandle,
    combo: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    // P5: explicit match — DB read errors should surface in logs, not coerce to "" silently
    // (which previously caused unregister(old) to no-op and leak the old registration).
    let old_combo = match settings.hotkey_slot1_combo() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "set_hotkey_slot1: failed to read old combo; treating as empty");
            String::new()
        }
    };

    // P10/D1: skip-if-equal fast-path — Win32 conflict-detection is process-wide,
    // so re-saving the currently-active combo would otherwise self-conflict.
    if combo == old_combo {
        return Ok(());
    }

    // AC-1 + P10/P11: grammar gate + unregister-old + Win32 probe + recovery.
    #[cfg(target_os = "windows")]
    crate::hotkey::validate_hotkey_not_conflicting(&app, &old_combo, &combo).await?;

    // Settings-Write + fires settings.changed event via TauriSettingsEmitter.
    settings.set_hotkey_slot1_combo(&combo)?;

    // AC-2 + P12: register new shortcut, with re-register-old recovery on failure.
    #[cfg(target_os = "windows")]
    crate::hotkey::reregister_hotkey(&app, &old_combo, &combo);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_ui_language(
    lang: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    settings.set_ui_language(&lang)
}

#[tauri::command]
#[specta::specta]
pub fn set_output_target(
    id: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    settings.set_output_target_id(&id)
}

#[tauri::command]
#[specta::specta]
pub fn set_dictionary_language(
    lang: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    settings.set_dictionary_language(&lang)
}

#[tauri::command]
#[specta::specta]
pub fn set_output_language(
    lang: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    settings.set_output_language(&lang)
}

// --- Core-Bulk-Get (1 command) ---

#[tauri::command]
#[specta::specta]
pub fn get_user_settings(
    settings: tauri::State<'_, Settings>,
) -> Result<UserSettings, AppError> {
    Ok(UserSettings {
        hotkey_slot1_combo: settings.hotkey_slot1_combo()?,
        output_target_id: settings.output_target_id()?,
        ui_language: settings.ui_language()?,
        dictionary_language: settings.dictionary_language()?,
        output_language: settings.output_language()?,
        hotkey_slot1_mode: settings.recording_mode_slot1()?.to_string(),
    })
}

// --- Recording-Mode (2 commands) ---

#[tauri::command]
#[specta::specta]
pub fn get_recording_mode_slot1(
    settings: tauri::State<'_, Settings>,
) -> Result<String, AppError> {
    settings.recording_mode_slot1().map(|m| m.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn set_recording_mode_slot1(
    mode: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    let parsed = RecordingMode::from_str(&mode)?;
    settings.set_recording_mode_slot1(parsed)?;
    // The orchestrator's mode_arc is updated by the `settings.changed` listener
    // in main.rs (single-writer pattern, AC-7 spirit). This command only writes
    // through Settings, which fires the emitter.
    Ok(())
}

// --- Plugin API (2 commands) ---

#[tauri::command]
#[specta::specta]
pub fn set_plugin_setting(
    plugin_id: String,
    key: String,
    value: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    settings.set_plugin_setting(&plugin_id, &key, &value)
}

#[tauri::command]
#[specta::specta]
pub fn get_plugin_setting(
    plugin_id: String,
    key: String,
    settings: tauri::State<'_, Settings>,
) -> Result<Option<String>, AppError> {
    settings.get_plugin_setting(&plugin_id, &key)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Compile-time check: TauriSettingsEmitter<Wry> satisfies SettingsEmitter trait bounds.
    #[allow(dead_code)]
    fn _assert_emitter_bounds<T: klarvo_core::settings::SettingsEmitter + Send + Sync>() {}

    #[allow(dead_code)]
    fn _check_wry() {
        _assert_emitter_bounds::<super::TauriSettingsEmitter<tauri::Wry>>();
    }
}
