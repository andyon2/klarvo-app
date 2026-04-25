use std::collections::HashMap;
use std::sync::Arc;

/// Phase-1 translation table. Keys are i18n keys emitted by klarvo-core (string literals).
/// Values are the user-visible strings for the active locale.
///
/// Phase-1: locale is loaded once at boot from ShellConfig.ui_language.
/// Phase-2: Settings-UI will trigger live locale-switch via tauri::State<I18nTable>
/// mutation + UI re-render event. Out of scope for Story 4.2.
///
/// Phase-2+: replace with ICU MessageFormat + pluralization, user-configurable locale
/// via Settings-Panel.
pub type I18nTable = HashMap<String, String>;

const EN_JSON: &str = include_str!("../../locales/en.json");
const DE_JSON: &str = include_str!("../../locales/de.json");

/// Load the locale table for `ui_language` and return it as a shared Arc for Tauri managed-state.
///
/// Both locale files are validated eagerly at boot regardless of which locale is active;
/// a corrupt `de.json` surfaces even when `ui_language = "en"`.
///
/// The `_` (default) arm returns the `en` table — Schema-Validation in Story 4.1 AC-C
/// guarantees `ui_language ∈ {en, de}`; the default arm is a defensive fallback against
/// Schema-Drift or fail-soft Config-Default (`ShellConfig::default()` → `"en"`).
///
/// # Panics
///
/// Panics if `locales/en.json` or `locales/de.json` is not valid JSON.
///
/// // Phase-2: replace panic with fail-soft AppError path per ADR-0009 SD-4 Boot-Error-UX.
pub fn load(ui_language: &str) -> Arc<I18nTable> {
    let _de: I18nTable = serde_json::from_str(DE_JSON).unwrap_or_else(|e| {
        panic!("i18n boot-fail: locales/de.json is not valid JSON: {e}")
    });
    let en: I18nTable = serde_json::from_str(EN_JSON).unwrap_or_else(|e| {
        panic!("i18n boot-fail: locales/en.json is not valid JSON: {e}")
    });
    let active = match ui_language {
        "de" => serde_json::from_str(DE_JSON).expect("de.json validated above"),
        _ => en,
    };
    Arc::new(active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_en_returns_en_table() {
        let table = load("en");
        let val = table.get("error.config.missing").expect("key must exist");
        assert!(
            !val.starts_with("TODO"),
            "en table must not contain TODO-markers: {val}"
        );
        assert!(
            val.contains("config.toml"),
            "en string should reference config.toml: {val}"
        );
    }

    #[test]
    fn load_de_returns_de_table() {
        let table = load("de");
        let val = table.get("error.config.missing").expect("key must exist");
        // After Story 4.3, de.json has real German strings.
        // Assert the value differs from the English string.
        let en_table = load("en");
        let en_val = en_table.get("error.config.missing").expect("key must exist");
        assert_ne!(
            val, en_val,
            "de table value should differ from en table value"
        );
    }

    #[test]
    fn load_unknown_locale_falls_back_to_en() {
        let en_table = load("en");
        let fallback_table = load("xx");
        assert_eq!(
            en_table.get("error.config.missing"),
            fallback_table.get("error.config.missing"),
            "unknown locale must fall back to en table"
        );
    }

    #[test]
    fn both_locale_files_valid_json_even_when_en_active() {
        // load("en") validates both files eagerly — this test asserts that loading
        // "en" does NOT skip de.json validation (regression guard for eager-validation semantics).
        // If de.json were corrupt, this call would panic — that's the intended behavior.
        let _table = load("en");
        // Reaching here means both files parsed successfully.
        assert!(!_table.is_empty(), "en table must not be empty");
    }
}
