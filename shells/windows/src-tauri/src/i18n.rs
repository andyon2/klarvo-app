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
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // NOTE(5.3): G3-Sub-Lint B (cargo xtask lint-events) prüft seit Story 5.3
    // mechanisch, ob Code-Emit-Sites in klarvo-core + klarvo-plugins in en.json
    // registriert sind. Diese Konstante und die zugehörigen Tests sind eine
    // überlappende manuelle Absicherung; Entfernung in einer Phase-2-Cleanup-Story.
    //
    // Manually maintained until the Phase-2-Cleanup-Story removes this list.
    // New error.* constants in core or plugins MUST be added here in the same PR
    // — PR reviewer should ask for the locale-file diff alongside any new constant.
    //
    // Audit source: _bmad-output/implementation-artifacts/i18n-coverage-audit-2026-04-25.md
    // Spec delta: story AC-F listed `error.stt.upstream_unavailable`; actual Groq plugin emits
    // `error.stt.upstream_5xx` + `error.stt.upstream_4xx` (klarvo-plugin-groq/src/lib.rs:54,58).
    //
    // Epic-4 review follow-up (2026-04-25): `error.internal` is the unwrap_or-Fallback emitted by
    // klarvo-shell-orchestrator/src/session.rs:148,155,176 when an AppError carries
    // user_message: None (5/6 PluginError variants in klarvo-core/src/error.rs:73-101). The audit
    // grep targeted user_message: Some(...) and missed the unwrap_or pattern; fix is to register
    // the key here without a PluginError refactor (deferred to Phase-2 backlog).
    const REQUIRED_KEYS: &[&str] = &[
        "error.config.missing",
        "error.config.unknown_field",
        "error.config.invalid_language",
        "error.config.output_target_not_found",
        "error.config.parse_failed",
        "error.settings.in_memory_fallback",
        "error.settings.validation",
        "error.audio.start_failed",
        "error.audio.device_unavailable",
        "error.audio.unsupported_format",
        "error.recording.timeout",
        "error.paste.send_input_failed",
        "error.keystore.read_failed",
        "error.keystore.not_found",
        "error.keystore.backend_unavailable",
        "error.keystore.key_missing",
        "error.hotkey.parse_failed",
        "error.hotkey.registration_failed",
        "error.hotkey.conflict",
        "error.pipeline.toml_parse_failure",
        "error.pipeline.schema_version_unsupported",
        "error.pipeline.unknown_stage_type",
        "error.pipeline.plugin_not_found",
        "error.pipeline.stage_type_mismatch",
        "error.output.target_not_found",
        "error.output.clipboard_unavailable",
        "error.stt.network",
        "error.stt.timeout",
        "error.stt.rate_limited",
        "error.stt.auth_failed",
        "error.stt.invalid_audio",
        "error.stt.key_not_configured",
        "error.stt.upstream_5xx",
        "error.stt.upstream_4xx",
        "error.internal",
        // Story 2.A.A4 P2-P11 added `error.unknown` to the locale files but
        // missed REQUIRED_KEYS; co-located in 2.A.A8-Sub since this test was
        // already touched here.
        "error.unknown",
        "tray.menu.exit",
        "tray.language_switcher.label",
        "tray.language.en",
        "tray.language.de",
    ];

    #[test]
    fn en_json_covers_all_required_keys() {
        let table: I18nTable = serde_json::from_str(EN_JSON).expect("en.json must be valid JSON");
        let mut missing = Vec::new();
        for &key in REQUIRED_KEYS {
            if !table.contains_key(key) {
                missing.push(key);
            }
        }
        assert!(
            missing.is_empty(),
            "en.json is missing required keys: {missing:?}"
        );
    }

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
    fn no_orphan_keys_in_en_json() {
        let table: I18nTable = serde_json::from_str(EN_JSON).expect("en.json must be valid JSON");
        let allowed: BTreeSet<&str> = REQUIRED_KEYS.iter().copied().collect();
        let orphans: Vec<&str> = table
            .keys()
            .map(String::as_str)
            .filter(|k| !allowed.contains(k))
            .collect();
        assert!(
            orphans.is_empty(),
            "en.json contains orphan keys not in REQUIRED_KEYS: {orphans:?}. \
             Add to REQUIRED_KEYS if the emit site exists, or remove from en.json."
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
