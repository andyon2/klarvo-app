use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Phase-1 translation table. Keys are i18n keys emitted by klarvo-core (string literals).
/// Values are the user-visible strings for the active locale.
pub type I18nTable = HashMap<String, String>;

/// Interior-mutable i18n state for live-locale-switch (Story 2.A.C3).
/// `RwLock` preferred over `Mutex`: read-heavy (many lookups, rare locale-switch writes).
pub type SharedI18nTable = Arc<RwLock<I18nTable>>;

const EN_JSON: &str = include_str!("../../locales/en.json");
const DE_JSON: &str = include_str!("../../locales/de.json");

/// Load the i18n table for `lang` and return the raw `HashMap`.
///
/// Both locale files are validated eagerly; a corrupt `de.json` surfaces even when
/// `lang = "en"`. Unknown locales fall back to English — the default arm is the
/// **primary** defense for the runtime path: Story 2.A.C3's `reload_locale` accepts
/// arbitrary strings from the WebView, so the boot-time Schema-Validation guarantee
/// (Story 4.1, `ui_language ∈ {en, de}`) no longer covers every caller.
///
/// Called at boot by [`load`] and on locale-switch by `reload_locale` (Story 2.A.C3).
///
/// # Panics
///
/// Panics if `locales/en.json` or `locales/de.json` is not valid JSON.
pub fn load_locale(lang: &str) -> I18nTable {
    let de: I18nTable = serde_json::from_str(DE_JSON).unwrap_or_else(|e| {
        panic!("i18n boot-fail: locales/de.json is not valid JSON: {e}")
    });
    let en: I18nTable = serde_json::from_str(EN_JSON).unwrap_or_else(|e| {
        panic!("i18n boot-fail: locales/en.json is not valid JSON: {e}")
    });
    match lang {
        "de" => de,
        _ => en,
    }
}

/// Load the locale table for `ui_language` wrapped in `Arc<RwLock<>>` for Tauri managed-state.
///
/// Story 2.A.C3: the `RwLock` wrapper makes the table interior-mutable so `reload_locale`
/// can replace it on a live locale-switch without restarting the app.
pub fn load(ui_language: &str) -> SharedI18nTable {
    Arc::new(RwLock::new(load_locale(ui_language)))
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // i18n-Drift wird durch `cargo xtask lint-events` (Stories 5.3 + 5.6) mechanisch enforct.

    #[test]
    fn de_json_covers_same_key_set() {
        let en: I18nTable = serde_json::from_str(EN_JSON).expect("en.json must be valid JSON");
        let de: I18nTable = serde_json::from_str(DE_JSON).expect("de.json must be valid JSON");
        let en_keys: BTreeSet<&str> = en.keys().map(String::as_str).collect();
        let de_keys: BTreeSet<&str> = de.keys().map(String::as_str).collect();
        let en_only: Vec<&&str> = en_keys.difference(&de_keys).collect();
        let de_only: Vec<&&str> = de_keys.difference(&en_keys).collect();
        assert!(
            en_only.is_empty() && de_only.is_empty(),
            "Key-set mismatch — en-only: {en_only:?}, de-only: {de_only:?}"
        );
    }

    #[test]
    fn no_todo_markers_in_en_json() {
        let table: I18nTable = serde_json::from_str(EN_JSON).expect("en.json must be valid JSON");
        let todo_keys: Vec<(&str, &str)> = table
            .iter()
            .filter(|(_, v)| v.starts_with("TODO"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert!(
            todo_keys.is_empty(),
            "en.json has TODO markers (en is the authoritative master): {todo_keys:?}"
        );
    }

    #[test]
    fn load_en_returns_en_table() {
        let table = load_locale("en");
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
        let table = load_locale("de");
        let val = table.get("error.config.missing").expect("key must exist");
        let en_table = load_locale("en");
        let en_val = en_table.get("error.config.missing").expect("key must exist");
        assert_ne!(
            val, en_val,
            "de table value should differ from en table value"
        );
    }

    #[test]
    fn load_unknown_locale_falls_back_to_en() {
        let en_table = load_locale("en");
        let fallback_table = load_locale("xx");
        assert_eq!(
            en_table.get("error.config.missing"),
            fallback_table.get("error.config.missing"),
            "unknown locale must fall back to en table"
        );
    }

    #[test]
    fn both_locale_files_valid_json_even_when_en_active() {
        // load_locale("en") validates both files eagerly — this test asserts that loading
        // "en" does NOT skip de.json validation (regression guard for eager-validation semantics).
        // If de.json were corrupt, this call would panic — that's the intended behavior.
        let table = load_locale("en");
        assert!(!table.is_empty(), "en table must not be empty");
    }

    // Note: a `load_returns_shared_i18n_table` test was removed during the C3
    // code-review pass — it asserted what the type signature already guarantees
    // (compile-time enforced) and added no behavioural coverage beyond
    // `load_locale`'s own tests.
}
