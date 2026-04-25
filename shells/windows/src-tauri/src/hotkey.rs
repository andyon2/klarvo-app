//! Hotkey wire-up for the Windows shell (Story 3.6, ADR-0011 SD-2/SD-4).
//!
//! [`register_hotkey`] parses the hotkey string from `ShellConfig`, registers it
//! with `tauri-plugin-global-shortcut`, and dispatches `Pressed`/`Released` events
//! to `SessionOrchestrator`. Degraded-mode semantics: parse or registration failures
//! emit `app.error` events (ADR-0009) and the app continues without a hotkey.

use std::str::FromStr;
use std::sync::Arc;

use klarvo_core::event::ErrorEmitter;
use klarvo_core::time::Clock;
use klarvo_shell_orchestrator::SessionOrchestrator;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::config::ShellConfig;

/// Register the global push-to-talk hotkey from `config.hotkey`.
///
/// Called from the Tauri `.setup()` hook (Story 3.10) after `SessionOrchestrator`
/// **and** `Arc<dyn ErrorEmitter>` have been inserted into `tauri::State`
/// via `app.manage(...)`.
///
/// Degraded-mode on failure: parse or registration errors are forwarded to the
/// frontend via the managed `Arc<dyn ErrorEmitter>` slot; the app continues
/// without a hotkey (ADR-0009 SD-4, ADR-0011 SD-4).
pub fn register_hotkey<R: tauri::Runtime>(app: &tauri::App<R>, config: &ShellConfig) {
    let handle = app.handle().clone();
    // Pull the shared error-emitter and clock from the managed-state slots
    // established in the bootstrap closure (main.rs Step 11).
    // - Emitter: single ADR-0009-SD-1 source-of-truth (rate-limit / dedup land here in Phase-2).
    // - Clock: shared session-baseline so hotkey-error `ts_ms` stays comparable
    //   with orchestrator-emitted Recording* events (project_event_ts_ms_convention).
    let emitter: Arc<dyn ErrorEmitter> = app.state::<Arc<dyn ErrorEmitter>>().inner().clone();
    let clock: Arc<dyn Clock> = app.state::<Arc<dyn Clock>>().inner().clone();

    // AC-B: parse hotkey string → Shortcut.
    let shortcut = match Shortcut::from_str(&config.hotkey) {
        Ok(s) => s,
        Err(_) => {
            let ts_ms = clock.now_ms();
            let emitter = Arc::clone(&emitter);
            tauri::async_runtime::spawn(async move {
                emitter.emit_error("error.hotkey.parse_failed", ts_ms).await;
            });
            return;
        }
    };

    // AC-C: register shortcut + dispatch (plugin activated in main.rs Builder chain
    // per ADR-0011 SD-4).
    // Key-repeat filtering lives in SessionOrchestrator (ADR-0011 SD-3).
    if let Err(_) = handle.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
        let orch = app.state::<Arc<SessionOrchestrator>>().inner().clone();
        match event.state() {
            ShortcutState::Pressed => {
                tauri::async_runtime::spawn(async move { orch.on_press().await });
            }
            ShortcutState::Released => {
                tauri::async_runtime::spawn(async move { orch.on_release().await });
            }
        }
    }) {
        // AC-D: registration failure path.
        let ts_ms = clock.now_ms();
        tauri::async_runtime::spawn(async move {
            emitter.emit_error("error.hotkey.registration_failed", ts_ms).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-check: `Arc<SessionOrchestrator>` satisfies `Send + Sync + 'static`,
    /// the bounds required by `tauri::State<Arc<SessionOrchestrator>>`.
    ///
    /// Passes by compiling. No runtime or constructor calls needed.
    #[allow(dead_code)]
    fn _assert_state_bounds<T: Send + Sync + 'static>() {}

    #[test]
    fn arc_session_orchestrator_satisfies_tauri_state_bounds() {
        _assert_state_bounds::<Arc<SessionOrchestrator>>();
    }

    /// MANUAL TEST: Start app, press CommandOrControl+Shift+Space, observe recording.
    // cargo xtask test-hotkey-manual
    // This test is an anchor for the xtask smoke-test subcommand (Phase-2 enhancement).
    #[test]
    #[ignore = "MANUAL TEST: Start app, press CommandOrControl+Shift+Space, observe recording. Run via: cargo xtask test-hotkey-manual. Phase-2 enhancement: xtask smoke-test subcommand."]
    fn hotkey_manual_test() {}
}
