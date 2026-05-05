//! Settings Tauri-Command surface (Story 2.A.A4 AC-6/7/8/9/10).
//!
//! - 8 Commands registered in `specta_builder()` via `collect_commands!`.
//! - `UserSettings` + `SettingsChangedEvent` exported to TS by tauri-specta.
//! - `TauriSettingsEmitter` implements `klarvo_core::settings::SettingsEmitter`;
//!   lives here so `klarvo-core` has no Tauri dependency (ADR-0009 Hybrid-C analog).

use std::str::FromStr;

use tauri_plugin_global_shortcut::Shortcut;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Emitter as _;
use tauri_specta::Event;

use klarvo_core::error::{AppError, AppErrorKind};
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
    /// Slot-2 hotkey combo; `None` when not configured (Story 8.1 D-3).
    pub hotkey_slot2_combo: Option<String>,
    /// Slot-2 recording mode; `"hold"` when not set.
    pub hotkey_slot2_mode: String,
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

    // P12 (Code-Review-Closure 2026-05-05): symmetric D-2 guard — reject when the
    // new slot-1 combo would collide with the configured slot-2 combo. Without
    // this, the only check was the Win32 OS-probe in `validate_hotkey_not_conflicting`,
    // which reports `error.hotkey.conflict` ("already in use by another application")
    // — misleading because the "other application" is Klarvo itself. Compares
    // parsed Shortcuts so case / modifier-order / whitespace differences cannot
    // bypass the check.
    if let Some(slot2) = settings.hotkey_slot2_combo().map_err(|e| AppError {
        kind: AppErrorKind::Internal,
        message: format!("failed to read slot-2 combo for D-2 conflict check: {}", e.message),
        user_message: Some("error.internal".into()),
        retryable: true,
    })? {
        if !slot2.is_empty() {
            let collides = match (Shortcut::from_str(&combo), Shortcut::from_str(&slot2)) {
                (Ok(a), Ok(b)) => a == b,
                _ => combo == slot2,
            };
            if collides {
                return Err(AppError {
                    kind: AppErrorKind::Configuration,
                    message: format!("hotkey slot-1 combo identical to slot-2: {combo}"),
                    user_message: Some("error.settings.hotkey.slot_conflict".into()),
                    retryable: false,
                });
            }
        }
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
        hotkey_slot2_combo: settings.hotkey_slot2_combo()?,
        hotkey_slot2_mode: settings.recording_mode_slot2()?.to_string(),
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

// --- Slot-2 Commands (Story 8.1) ---

/// Set or clear the slot-2 hotkey combo.
///
/// `None` → clears the combo (slot 2 becomes inactive on next reboot).
/// `Some(combo)` → validates grammar + Slot-1 conflict before writing to DB.
/// Re-registration happens at next app start (no live re-register — see Dev Notes).
#[tauri::command]
#[specta::specta]
pub async fn set_hotkey_slot2(
    combo: Option<String>,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    match combo {
        None => settings.clear_hotkey_slot2_combo()?,
        Some(ref new_combo) => {
            // Grammar gate first — needed both for validation and for normalized
            // comparison against slot-1 (Code-Review-Closure 2026-05-05 P3:
            // raw-string compare let case/modifier-order/whitespace differences
            // bypass the D-2 guard).
            let new_parsed = Shortcut::from_str(new_combo).map_err(|_| AppError {
                kind: AppErrorKind::Configuration,
                message: format!("invalid hotkey combo: {new_combo}"),
                user_message: Some("error.hotkey.parse_failed".into()),
                retryable: false,
            })?;

            // D-2 Backend-Guard (P3 + P7): compare parsed Shortcuts. P7: surface
            // DB-read errors instead of `unwrap_or_default()` which would silently
            // skip the conflict-check on a transient SQLite failure.
            let slot1 = settings.hotkey_slot1_combo().map_err(|e| AppError {
                kind: AppErrorKind::Internal,
                message: format!("failed to read slot-1 combo for D-2 conflict check: {}", e.message),
                user_message: Some("error.internal".into()),
                retryable: true,
            })?;
            if !slot1.is_empty() {
                let collides = match Shortcut::from_str(&slot1) {
                    Ok(slot1_parsed) => slot1_parsed == new_parsed,
                    Err(_) => new_combo == &slot1, // raw fallback if slot-1 unparseable
                };
                if collides {
                    return Err(AppError {
                        kind: AppErrorKind::Configuration,
                        message: format!("hotkey slot-2 combo identical to slot-1: {new_combo}"),
                        user_message: Some("error.settings.hotkey.slot_conflict".into()),
                        retryable: false,
                    });
                }
            }
            settings.set_hotkey_slot2_combo(new_combo)?;
        }
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_recording_mode_slot2(
    mode: String,
    settings: tauri::State<'_, Settings>,
) -> Result<(), AppError> {
    let parsed = RecordingMode::from_str(&mode)?;
    settings.set_recording_mode_slot2(parsed)
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

// --- Locale-Reload (Story 2.A.C3) ---

/// Upper bound for the `lang` parameter accepted by [`reload_locale`].
/// BCP-47 tags in `SUPPORTED_LANGUAGES` are 2-character codes today; 16 leaves
/// room for region/script subtags without admitting unbounded WebView strings.
const MAX_LANG_LEN: usize = 16;

/// Inner reload logic — extracted for unit-testability (avoids Tauri runtime in tests).
///
/// Validates `lang` against the length cap and the supported-language allow-list
/// before writing. Oversized or unknown locales are a fail-soft no-op
/// (`tracing::warn!` with sanitized log field + `Ok(())`).
fn apply_locale_reload(lang: &str, i18n_table: &crate::i18n::SharedI18nTable) -> Result<(), AppError> {
    if lang.len() > MAX_LANG_LEN {
        let truncated: String = lang.chars().take(MAX_LANG_LEN).collect();
        tracing::warn!(
            lang_truncated = %truncated,
            lang_len = lang.len(),
            "reload_locale: oversized locale; rejecting"
        );
        return Ok(());
    }
    if !crate::config::SUPPORTED_LANGUAGES.contains(&lang) {
        // Strip control chars to prevent log injection from a misbehaving WebView.
        let sanitized: String = lang.chars().filter(|c| !c.is_control()).collect();
        tracing::warn!(lang = %sanitized, "reload_locale: unsupported locale; keeping current i18n table");
        return Ok(());
    }
    let new_table = crate::i18n::load_locale(lang);
    // Recover from poisoning: the wrapped `HashMap` cannot be partially mutated
    // (replace-only), so a poisoned lock still holds intact data — fail-soft per ADR-0009.
    *i18n_table.write().unwrap_or_else(|e| e.into_inner()) = new_table;
    Ok(())
}

/// Reload the backend `i18n_table` for `lang` without restarting the app (Story 2.A.C3 AC-2).
///
/// Called by the frontend `settings.changed` listener (AC-3) when `ui.language` changes.
/// Unknown locales are fail-soft: table unchanged + `tracing::warn!`.
#[tauri::command]
#[specta::specta]
pub fn reload_locale(
    lang: String,
    i18n_table: tauri::State<'_, crate::i18n::SharedI18nTable>,
) -> Result<(), AppError> {
    apply_locale_reload(&lang, &i18n_table)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::i18n::{load_locale, SharedI18nTable};
    use std::sync::{Arc, RwLock};

    fn make_table(lang: &str) -> SharedI18nTable {
        Arc::new(RwLock::new(load_locale(lang)))
    }

    /// Compile-time check: TauriSettingsEmitter<Wry> satisfies SettingsEmitter trait bounds.
    #[allow(dead_code)]
    fn _assert_emitter_bounds<T: klarvo_core::settings::SettingsEmitter + Send + Sync>() {}

    #[allow(dead_code)]
    fn _check_wry() {
        _assert_emitter_bounds::<super::TauriSettingsEmitter<tauri::Wry>>();
    }

    #[test]
    fn reload_locale_known_lang_replaces_table() {
        let table = make_table("en");
        let en_val = table.read().unwrap().get("error.config.missing").unwrap().clone();

        apply_locale_reload("de", &table).unwrap();

        let de_val = table.read().unwrap().get("error.config.missing").unwrap().clone();
        assert_ne!(en_val, de_val, "de table must differ from en table after reload");
    }

    #[test]
    fn reload_locale_unknown_lang_is_noop() {
        let table = make_table("en");
        let before: crate::i18n::I18nTable = table.read().unwrap().clone();

        apply_locale_reload("zz", &table).unwrap();

        let after = table.read().unwrap();
        assert_eq!(before, *after, "unknown locale must not mutate the i18n table");
    }

    #[test]
    fn reload_locale_de_then_en_round_trips() {
        let table = make_table("en");
        apply_locale_reload("de", &table).unwrap();
        apply_locale_reload("en", &table).unwrap();

        let expected = load_locale("en");
        assert_eq!(*table.read().unwrap(), expected, "table must match en after de→en round-trip");
    }

    #[test]
    fn reload_locale_same_lang_is_idempotent() {
        let table = make_table("en");
        apply_locale_reload("en", &table).unwrap();
        assert!(table.read().unwrap().contains_key("error.config.missing"));
    }
}
