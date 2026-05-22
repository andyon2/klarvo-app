//! Klarvo Windows shell — Phase-0 specta smoke surface + Story-3.1 skeleton.
//!
//! Exposes a single command and a single event registered via `tauri-specta`
//! so that `xtask generate-bindings` has a real payload to emit and
//! `xtask lint-events` has a real event type to enforce the dot-notation
//! rename convention against (G1 Validation-Patch).
//!
//! Story 2.A.A4: `commands::settings` adds 8 Settings commands + `SettingsChangedEvent`.

pub mod bridge;
pub mod notification;
pub mod commands;
pub mod config;
pub mod i18n;
pub mod tray;
#[cfg(target_os = "windows")]
pub mod overlay;
#[cfg(any(target_os = "windows", feature = "dev-plain-keystore"))]
pub mod keystore;
#[cfg(target_os = "windows")]
pub mod audio;
#[cfg(target_os = "windows")]
pub mod hotkey;
#[cfg(target_os = "windows")]
pub mod focus;
#[cfg(target_os = "windows")]
pub mod paste;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::{Builder, Event, collect_commands, collect_events};

use commands::history::{clear_history, delete_history_entry, get_history};
use commands::recording::cancel_recording;
use commands::telemetry::export_debug_zip_cmd;
#[cfg(target_os = "windows")]
use overlay::pill_bar::dev_pill_bar_enter_live_preview;
use commands::settings::{
    SettingsChangedEvent, get_plugin_setting, get_recording_mode_slot1, get_user_settings,
    list_audio_input_devices, reload_locale, set_audio_input_device, set_dictionary_language,
    set_hotkey_slot1, set_hotkey_slot2, set_output_language, set_output_target,
    set_plugin_setting, set_recording_mode_slot1, set_recording_mode_slot2, set_ui_language,
};

#[tauri::command]
#[specta::specta]
fn ping(name: String) -> String {
    format!("pong: {name}")
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "app:ready")]
pub struct AppReady {
    pub session_id: String,
}

/// Shared specta builder — single source of truth for the runtime app
/// (`main.rs`) and the export binary (`bin/export_bindings.rs`).
pub fn specta_builder() -> Builder<tauri::Wry> {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            ping,
            // Story 2.A.A4: Settings commands (AC-6/7)
            set_hotkey_slot1,
            set_ui_language,
            set_output_target,
            set_dictionary_language,
            set_output_language,
            get_user_settings,
            set_plugin_setting,
            get_plugin_setting,
            // Story 2.B.A1: Recording-Mode commands (AC-8)
            get_recording_mode_slot1,
            set_recording_mode_slot1,
            // Story 8.1: Second hotkey-slot commands
            set_hotkey_slot2,
            set_recording_mode_slot2,
            // Story 2.A.C3: Live-Locale-Switch
            reload_locale,
            // Story 9.2: History commands
            get_history,
            delete_history_entry,
            clear_history,
            // Story 9.5: Debug-Export command
            export_debug_zip_cmd,
            // Story 11.1: Pill-Bar abort button
            cancel_recording,
            // Story 12.3: Audio input device selection
            list_audio_input_devices,
            set_audio_input_device,
        ]);
    // Story 11.3: LP-resize test trigger (Windows-only; overlay module is cfg-gated)
    #[cfg(target_os = "windows")]
    let builder = builder.commands(collect_commands![dev_pill_bar_enter_live_preview]);
    builder.events(collect_events![AppReady, SettingsChangedEvent])
}
