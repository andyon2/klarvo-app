//! Klarvo Windows shell — Phase-0 specta smoke surface + Story-3.1 skeleton.
//!
//! Exposes a single command and a single event registered via `tauri-specta`
//! so that `xtask generate-bindings` has a real payload to emit and
//! `xtask lint-events` has a real event type to enforce the dot-notation
//! rename convention against (G1 Validation-Patch).

pub mod bridge;
pub mod config;
pub mod i18n;
#[cfg(any(target_os = "windows", feature = "dev-plain-keystore"))]
pub mod keystore;
#[cfg(target_os = "windows")]
pub mod audio;
#[cfg(target_os = "windows")]
pub mod hotkey;
#[cfg(target_os = "windows")]
pub mod paste;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::{Builder, Event, collect_commands, collect_events};

#[tauri::command]
#[specta::specta]
fn ping(name: String) -> String {
    format!("pong: {name}")
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "app.ready")]
pub struct AppReady {
    pub session_id: String,
}

/// Shared specta builder — single source of truth for the runtime app
/// (`main.rs`) and the export binary (`bin/export_bindings.rs`).
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![ping])
        .events(collect_events![AppReady])
}
