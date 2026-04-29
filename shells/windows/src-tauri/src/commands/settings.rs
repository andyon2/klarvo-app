//! Settings Tauri-Command surface (Story 2.A.A4 AC-6/7/8/9/10).
//!
//! - 8 Commands registered in `specta_builder()` via `collect_commands!`.
//! - `UserSettings` + `SettingsChangedEvent` exported to TS by tauri-specta.
//! - `TauriSettingsEmitter` implements `klarvo_core::settings::SettingsEmitter`;
//!   lives here so `klarvo-core` has no Tauri dependency (ADR-0009 Hybrid-C analog).

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Emitter as _;
use tauri_specta::Event;

use klarvo_core::error::AppError;
use klarvo_core::settings::Settings;

// ---------------------------------------------------------------------------
// Shared payload types (tauri-specta exported)
// ---------------------------------------------------------------------------

/// Bulk-read projection of all 5 Core-Settings (AC-6 `get_user_settings` return type).
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

#[tauri::command]
#[specta::specta]
pub fn set_hotkey_slot1(
    combo: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    settings.set_hotkey_slot1_combo(&combo)
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
    })
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
