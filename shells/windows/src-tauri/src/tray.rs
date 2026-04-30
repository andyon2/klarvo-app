//! Tray-menu construction for the Windows shell (Story 2.A.A8-Sub).
//!
//! Centralises the menu layout so the same builder runs at boot and from the
//! `settings.changed` listener that rebuilds the menu when `ui.language`
//! changes (AC-3 live update). Keeps `main.rs` free of menu/i18n details.
//!
//! # Scope-fence (AC-5, Option B)
//!
//! The locale items in the language sub-menu are rendered as **disabled**
//! `CheckMenuItem`s — they show the active locale visually, but clicking them
//! does **not** trigger a settings write. Active locale switching lives in the
//! Settings-Panel (`set_ui_language` Tauri command, Story 2.A.A4); the tray
//! is purely reactive to the `settings.changed` event.
//!
//! Story 2.A.C3 (Live-Locale-Switch) will reuse [`build_menu`] when it upgrades
//! the managed `I18nTable` to an interior-mutable container.

use std::collections::HashMap;

use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Manager, Runtime};

/// Stable id of the system-tray icon. `main.rs` registers the tray under this
/// id and the `settings.changed` listener resolves it via `app.tray_by_id`.
pub const TRAY_ID: &str = "klarvo-tray";

/// Locales that surface in the tray language sub-menu. Pair is
/// `(locale_code, i18n_key_for_display_label)`. Mirrors the `ui.language`
/// allow-list enforced by `ShellConfig::ui_language` (Story 4.1).
pub const SUPPORTED_LOCALES: &[(&str, &str)] = &[
    ("en", "tray.language.en"),
    ("de", "tray.language.de"),
];

/// Build the tray menu for the given i18n table and active locale.
///
/// Layout:
/// ```text
/// Klarvo            (disabled info row)
/// Sprache ▶
///   ✓ English       (disabled — visual marker)
///     Deutsch       (disabled — visual marker)
/// Beenden
/// ```
///
/// Translation keys consumed:
/// - `tray.menu.exit`
/// - `tray.language_switcher.label`
/// - `tray.language.<code>` for each entry of [`SUPPORTED_LOCALES`]
///
/// Missing keys fall back to the locale code or English label so the tray
/// stays usable even if a future locale ships an incomplete translation.
pub fn build_menu<R: Runtime, M: Manager<R>>(
    manager: &M,
    i18n: &HashMap<String, String>,
    active_locale: &str,
) -> tauri::Result<Menu<R>> {
    let lookup = |key: &str, fallback: &str| -> String {
        i18n.get(key).cloned().unwrap_or_else(|| fallback.to_string())
    };

    let language_header = lookup("tray.language_switcher.label", "Language");
    let exit_label = lookup("tray.menu.exit", "Exit");

    let mut submenu_builder = SubmenuBuilder::with_id(manager, "language", &language_header);
    for (code, key) in SUPPORTED_LOCALES {
        let label = lookup(key, code);
        let item = CheckMenuItemBuilder::with_id(format!("language.{code}"), &label)
            .checked(active_locale == *code)
            .enabled(false)
            .build(manager)?;
        submenu_builder = submenu_builder.item(&item);
    }
    let submenu = submenu_builder.build()?;

    MenuBuilder::new(manager)
        .item(&MenuItemBuilder::with_id("info", "Klarvo").enabled(false).build(manager)?)
        .item(&submenu)
        .item(&MenuItemBuilder::with_id("quit", &exit_label).build(manager)?)
        .build()
}

/// Rebuild the tray menu using the locale loaded fresh from the embedded
/// locale files. No-op when the tray is missing (e.g. when the boot-time
/// builder failed in fail-soft mode).
///
/// Reloads the locale file on every call rather than mutating shared state,
/// so this stays compatible with the boot-time `Arc<I18nTable>` snapshot
/// used elsewhere in `main.rs`.
pub fn rebuild_for_locale<R: Runtime>(app: &tauri::AppHandle<R>, locale: &str) {
    let i18n = crate::i18n::load(locale);
    let menu = match build_menu(app, &i18n, locale) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, locale, "tray menu rebuild failed");
            return;
        }
    };
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        tracing::warn!(tray_id = TRAY_ID, "tray not found while applying locale change");
        return;
    };
    if let Err(e) = tray.set_menu(Some(menu)) {
        tracing::error!(error = %e, locale, "tray.set_menu failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_locales_have_distinct_codes() {
        let mut seen = std::collections::HashSet::new();
        for (code, _) in SUPPORTED_LOCALES {
            assert!(seen.insert(*code), "duplicate locale code: {code}");
        }
    }

    #[test]
    fn supported_locales_match_label_keys() {
        for (code, key) in SUPPORTED_LOCALES {
            assert!(
                key.starts_with("tray.language."),
                "label key for {code} must use tray.language.* namespace, got {key}"
            );
            assert!(
                key.ends_with(code),
                "label key for {code} should end with the locale code, got {key}"
            );
        }
    }
}
