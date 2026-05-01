//! Hotkey wire-up for the Windows shell (Story 3.6, ADR-0011 SD-2/SD-4).
//!
//! [`register_hotkey`] parses the hotkey string from `ShellConfig`, registers it
//! with `tauri-plugin-global-shortcut`, and dispatches `Pressed`/`Released` events
//! to `SessionOrchestrator`. Degraded-mode semantics: parse or registration failures
//! emit `app.error` events (ADR-0009) and the app continues without a hotkey.
//!
//! Story 2.A.C2 adds:
//! - [`validate_hotkey_not_conflicting`]: grammar gate (`Shortcut::from_str`) +
//!   unregister-old + Win32 `RegisterHotKey` probe + re-register-old recovery.
//! - [`reregister_hotkey`]: register new shortcut + re-register-old recovery on fail.

use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use klarvo_core::error::{AppError, AppErrorKind};
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
    if let Err(_) = handle.global_shortcut().on_shortcut(shortcut, shortcut_dispatch_handler()) {
        // AC-D: registration failure path.
        let ts_ms = clock.now_ms();
        tauri::async_runtime::spawn(async move {
            emitter.emit_error("error.hotkey.registration_failed", ts_ms).await;
        });
    }
}

/// Pre-Settings-Write validation + Win32 conflict-probe with old-hotkey recovery
/// (Story 2.A.C2 AC-1; Code-Review patches P1/P2/P3/P10/P11).
///
/// Flow:
/// 1. Grammar-gate `new_combo` via `Shortcut::from_str` (P11) — fail → `error.hotkey.parse_failed`.
/// 2. Unregister `old_combo` via global-shortcut (best-effort) — defends against the
///    self-conflict scenario where Win32 sees our own existing registration as a clash (P10/D1).
/// 3. Win32 `RegisterHotKey` + `UnregisterHotKey` probe with `MOD_NOREPEAT` parity (P1),
///    per-call atomic ID (P2), and RAII guard for unwind-safe cleanup (P3).
/// 4. Probe-fail → re-register `old_combo` as recovery (P10) and return `HotkeyConflict`.
/// 5. Probe-success → caller proceeds with Settings-Write + [`reregister_hotkey`].
///
/// Uses `spawn_blocking` because `RegisterHotKey(NULL, ...)` is thread-specific (Win32
/// message-queue of the calling thread); calling it from the Tokio async runtime is safe
/// only from a dedicated blocking thread.
pub async fn validate_hotkey_not_conflicting<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    old_combo: &str,
    new_combo: &str,
) -> Result<(), AppError> {
    // P11: grammar gate — anything Shortcut::from_str rejects must not reach the probe,
    // otherwise probe-success would be followed by a guaranteed reregister parse-fail.
    Shortcut::from_str(new_combo).map_err(|_| AppError {
        kind: AppErrorKind::Validation,
        message: format!("invalid hotkey combo: {new_combo}"),
        user_message: Some("error.hotkey.parse_failed".into()),
        retryable: false,
    })?;

    let gs = app_handle.global_shortcut();
    let old_parsed = Shortcut::from_str(old_combo).ok();

    // P10/D1: unregister old before probing — Win32 conflict-detection is process-wide,
    // so even an unrelated combo could spuriously fail if global-hotkey still holds the old.
    if let Some(old_sc) = old_parsed.as_ref() {
        let _ = gs.unregister(*old_sc);
    }

    match probe_win32_combo(new_combo).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // P10/D1 recovery: probe failed → re-register old to keep app functional.
            if let Some(old_sc) = old_parsed {
                let _ = gs.on_shortcut(old_sc, shortcut_dispatch_handler());
            }
            Err(e)
        }
    }
}

/// Win32 `RegisterHotKey`/`UnregisterHotKey` probe.
///
/// Production registration via `tauri-plugin-global-shortcut` uses `MOD_NOREPEAT` —
/// the probe matches that bit (P1) so the probe and runtime agree on conflict semantics.
/// Per-call atomic ID (P2) prevents same-process collisions when concurrent calls land
/// on different blocking-pool threads. RAII guard (P3) ensures `UnregisterHotKey` runs
/// even if the closure unwinds between register and the explicit drop.
async fn probe_win32_combo(combo: &str) -> Result<(), AppError> {
    static NEXT_PROBE_ID: AtomicI32 = AtomicI32::new(0xBEEF);

    let combo = combo.to_string();
    let probe_id = NEXT_PROBE_ID.fetch_add(1, Ordering::Relaxed);

    tokio::task::spawn_blocking(move || {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
        };

        let (modifiers, vk) = parse_combo_to_win32(&combo).ok_or_else(|| AppError {
            kind: AppErrorKind::Validation,
            message: format!("cannot map hotkey combo to Win32 VK: {combo}"),
            user_message: Some("error.hotkey.parse_failed".into()),
            retryable: false,
        })?;

        // P3: scope-guard releases the probe registration on every exit path,
        // including panic unwind. Without this, a crashed probe would leak the
        // (process-wide) hotkey-id slot for the lifetime of the blocking thread.
        struct ProbeGuard {
            id: i32,
            registered: bool,
        }
        impl Drop for ProbeGuard {
            fn drop(&mut self) {
                if self.registered {
                    unsafe {
                        let _ = UnregisterHotKey(None, self.id);
                    }
                }
            }
        }
        let mut guard = ProbeGuard { id: probe_id, registered: false };

        // P1: MOD_NOREPEAT to match production-registration modifier mask exactly.
        let probe_mods = modifiers | MOD_NOREPEAT;
        let result = unsafe { RegisterHotKey(None, probe_id, probe_mods, vk) };

        match result {
            Ok(()) => {
                guard.registered = true; // Drop releases on return.
                Ok(())
            }
            Err(_) => Err(AppError {
                kind: AppErrorKind::HotkeyConflict,
                message: format!("hotkey combo already registered system-wide: {combo}"),
                user_message: Some("error.hotkey.conflict".into()),
                retryable: false,
            }),
        }
    })
    .await
    .map_err(|e| AppError {
        kind: AppErrorKind::Internal,
        message: format!("spawn_blocking panic in probe_win32_combo: {e}"),
        user_message: None,
        retryable: false,
    })?
}

/// Post-Settings-Write registration of `new_combo` with old-hotkey recovery
/// (Story 2.A.C2 AC-2; Code-Review patches P4/P12).
///
/// At call-time, [`validate_hotkey_not_conflicting`] has already unregistered
/// `old_combo` (and possibly re-registered it on probe-fail). The probe-success
/// path leaves the app with no shortcut bound; this function registers `new_combo`.
/// On `on_shortcut(new)` failure: re-register `old_combo` as recovery (P12) so the
/// app remains functional with the previous hotkey while Settings persist the new
/// value (the user must re-attempt or accept the divergence; AC-2 explicitly forbids
/// Settings-rollback).
pub fn reregister_hotkey<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    old_combo: &str,
    new_combo: &str,
) {
    let gs = app_handle.global_shortcut();

    let new_shortcut = match Shortcut::from_str(new_combo) {
        Ok(s) => s,
        Err(_) => {
            // P4: ADR-0009 lossless contract — emit, don't just trace.
            tracing::warn!(combo = new_combo, "reregister_hotkey: Shortcut::from_str failed after Win32 pre-validation");
            emit_hotkey_error_async(app_handle, "error.hotkey.parse_failed");
            // P12: try to keep the app functional with the old combo.
            if let Ok(old_sc) = Shortcut::from_str(old_combo) {
                let _ = gs.on_shortcut(old_sc, shortcut_dispatch_handler());
            }
            return;
        }
    };

    if let Err(e) = gs.on_shortcut(new_shortcut, shortcut_dispatch_handler()) {
        tracing::warn!(error = %e, combo = new_combo, "reregister_hotkey: on_shortcut(new) failed; recovering with old");
        // P12/D4: re-register old so the app still has a working hotkey.
        if let Ok(old_sc) = Shortcut::from_str(old_combo) {
            if let Err(re) = gs.on_shortcut(old_sc, shortcut_dispatch_handler()) {
                tracing::warn!(error = %re, combo = old_combo, "reregister_hotkey: recovery re-register-old also failed");
            }
        }
        emit_hotkey_error_async(app_handle, "error.hotkey.registration_failed");
    }
}

/// Spawn an async task that emits an `app.error` toast via ADR-0009 (rate-limited at the emitter).
fn emit_hotkey_error_async<R: tauri::Runtime>(app: &tauri::AppHandle<R>, key: &'static str) {
    let emitter: Arc<dyn ErrorEmitter> = app.state::<Arc<dyn ErrorEmitter>>().inner().clone();
    let clock: Arc<dyn Clock> = app.state::<Arc<dyn Clock>>().inner().clone();
    let ts_ms = clock.now_ms();
    tauri::async_runtime::spawn(async move {
        emitter.emit_error(key, ts_ms).await;
    });
}

/// Shared press/release dispatch closure for `tauri-plugin-global-shortcut`.
///
/// Extracted so both `register_hotkey` (boot) and `reregister_hotkey` (settings-change)
/// use identical dispatch logic without duplication.
fn shortcut_dispatch_handler<R: tauri::Runtime>(
) -> impl Fn(&tauri::AppHandle<R>, &Shortcut, tauri_plugin_global_shortcut::ShortcutEvent)
       + Send
       + Sync
       + 'static {
    |app: &tauri::AppHandle<R>, _shortcut: &Shortcut, event: tauri_plugin_global_shortcut::ShortcutEvent| {
        let orch = app.state::<SessionOrchestrator>().inner().clone();
        match event.state() {
            ShortcutState::Pressed => {
                tauri::async_runtime::spawn(async move { orch.on_press().await });
            }
            ShortcutState::Released => {
                tauri::async_runtime::spawn(async move { orch.on_release().await });
            }
        }
    }
}

/// Parse an Electron/Tauri accelerator combo string to Win32 `(HOT_KEY_MODIFIERS, vk_code)`.
///
/// Accepts case-insensitive modifier names: `Ctrl`, `Control`, `CommandOrControl`,
/// `CmdOrCtrl`, `Alt`, `Option`, `Shift`, `Win`, `Super`, `Meta`, `Command`, `Cmd`.
///
/// Returns `None` if the key part cannot be mapped to a Win32 virtual-key code.
fn parse_combo_to_win32(
    combo: &str,
) -> Option<(windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS, u32)> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
    };

    let mut modifiers = HOT_KEY_MODIFIERS(0);
    let mut key_part: Option<&str> = None;

    for token in combo.split('+') {
        match token.trim().to_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" | "cmdorctrl" | "command" | "cmd" => {
                modifiers |= MOD_CONTROL;
            }
            "alt" | "option" => {
                modifiers |= MOD_ALT;
            }
            "shift" => {
                modifiers |= MOD_SHIFT;
            }
            "win" | "super" | "meta" => {
                modifiers |= MOD_WIN;
            }
            _ => {
                key_part = Some(token.trim());
            }
        }
    }

    let key = key_part?;
    let vk = key_name_to_vk(key)?;
    Some((modifiers, vk))
}

/// Map a key name (from an accelerator string) to its Win32 virtual-key code.
fn key_name_to_vk(key: &str) -> Option<u32> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let vk = match key.to_lowercase().as_str() {
        // Letters
        "a" => VK_A, "b" => VK_B, "c" => VK_C, "d" => VK_D, "e" => VK_E,
        "f" => VK_F, "g" => VK_G, "h" => VK_H, "i" => VK_I, "j" => VK_J,
        "k" => VK_K, "l" => VK_L, "m" => VK_M, "n" => VK_N, "o" => VK_O,
        "p" => VK_P, "q" => VK_Q, "r" => VK_R, "s" => VK_S, "t" => VK_T,
        "u" => VK_U, "v" => VK_V, "w" => VK_W, "x" => VK_X, "y" => VK_Y,
        "z" => VK_Z,
        // Digits
        "0" => VK_0, "1" => VK_1, "2" => VK_2, "3" => VK_3, "4" => VK_4,
        "5" => VK_5, "6" => VK_6, "7" => VK_7, "8" => VK_8, "9" => VK_9,
        // Function keys (F1-F24 — F13-F24 commonly used by uncontested test combos
        // and on extended keyboards; also accepted by Shortcut::from_str)
        "f1" => VK_F1, "f2" => VK_F2, "f3" => VK_F3, "f4" => VK_F4,
        "f5" => VK_F5, "f6" => VK_F6, "f7" => VK_F7, "f8" => VK_F8,
        "f9" => VK_F9, "f10" => VK_F10, "f11" => VK_F11, "f12" => VK_F12,
        "f13" => VK_F13, "f14" => VK_F14, "f15" => VK_F15, "f16" => VK_F16,
        "f17" => VK_F17, "f18" => VK_F18, "f19" => VK_F19, "f20" => VK_F20,
        "f21" => VK_F21, "f22" => VK_F22, "f23" => VK_F23, "f24" => VK_F24,
        // Navigation / editing
        "space" => VK_SPACE,
        "return" | "enter" => VK_RETURN,
        "tab" => VK_TAB,
        "escape" | "esc" => VK_ESCAPE,
        "delete" | "del" => VK_DELETE,
        "insert" | "ins" => VK_INSERT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "backspace" | "back" => VK_BACK,
        _ => return None,
    };
    Some(vk.0 as u32)
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    /// Compile-check: `SessionOrchestrator` satisfies `Send + Sync + 'static`,
    /// the bounds required by `tauri::State<SessionOrchestrator>`.
    #[allow(dead_code)]
    fn _assert_state_bounds<T: Send + Sync + 'static>() {}

    #[test]
    fn session_orchestrator_satisfies_tauri_state_bounds() {
        _assert_state_bounds::<SessionOrchestrator>();
    }

    #[test]
    fn parse_combo_ctrl_shift_v() {
        let (mods, vk) = parse_combo_to_win32("ctrl+shift+v").unwrap();
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        assert_eq!(mods, MOD_CONTROL | MOD_SHIFT);
        assert_eq!(vk, VK_V.0 as u32);
    }

    #[test]
    fn parse_combo_commandorcontrol_space() {
        let (mods, vk) = parse_combo_to_win32("CommandOrControl+Space").unwrap();
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        assert_eq!(mods, MOD_CONTROL);
        assert_eq!(vk, VK_SPACE.0 as u32);
    }

    #[test]
    fn parse_combo_ctrl_shift_alt_f5() {
        let (mods, vk) = parse_combo_to_win32("ctrl+shift+alt+f5").unwrap();
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        assert_eq!(mods, MOD_CONTROL | MOD_SHIFT | MOD_ALT);
        assert_eq!(vk, VK_F5.0 as u32);
    }

    #[test]
    fn parse_combo_unknown_key_returns_none() {
        assert!(parse_combo_to_win32("ctrl+shift+tilde").is_none());
    }

    #[test]
    fn parse_combo_modifier_only_returns_none() {
        assert!(parse_combo_to_win32("ctrl+shift").is_none());
    }

    /// Win32 probe round-trip: register+unregister a very obscure combo to confirm
    /// the API is reachable and returns success on an uncontested key.
    ///
    /// Uses Ctrl+Shift+Alt+F24 (P7) — F24 is on extended keyboards only, so
    /// effectively never bound by user-facing apps even on developer machines
    /// that overload F12 (DevTools, NVIDIA overlay, screen-capture tools).
    /// Must run on Windows; skipped in non-Windows CI.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn win32_validation_uncontested_combo_succeeds() {
        let result = probe_win32_combo("ctrl+shift+alt+f24").await;
        assert!(result.is_ok(), "expected uncontested combo to succeed: {result:?}");
    }

    /// MANUAL TEST: Start app, press configured hotkey, observe recording.
    #[test]
    #[ignore = "MANUAL TEST: Start app, press CommandOrControl+Shift+Space, observe recording."]
    fn hotkey_manual_test() {}
}
