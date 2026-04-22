use std::collections::HashMap;
use std::sync::Arc;

/// Phase-1 translation table. Keys are i18n keys emitted by klarvo-core (string literals).
/// Values are the user-visible strings for the active locale.
///
/// Phase-2: replace with ICU MessageFormat + pluralization, user-configurable locale
/// via Settings-Panel (Story-3.2 adds locale config, Story-3.1 fixes the default to `en`).
pub type I18nTable = HashMap<String, String>;

const EN_JSON: &str = include_str!("../../locales/en.json");
const DE_JSON: &str = include_str!("../../locales/de.json");

/// Load the default (`en`) locale and return it as a shared Arc for Tauri managed-state.
///
/// Both locale files are validated at boot; an invalid file produces a diagnostic panic.
///
/// # Panics
///
/// Panics if `locales/en.json` or `locales/de.json` is not valid JSON.
///
/// // Phase-2: replace panic with fail-soft AppError path per ADR-0009 SD-4 Boot-Error-UX.
pub fn load_default() -> Arc<I18nTable> {
    // Eagerly validate `de` so a corrupt locale file surfaces at boot, not on locale-switch.
    let _de: I18nTable = serde_json::from_str(DE_JSON).unwrap_or_else(|e| {
        // Phase-2: replace panic with fail-soft AppError path per ADR-0009 SD-4 Boot-Error-UX.
        panic!("i18n boot-fail: locales/de.json is not valid JSON: {e}")
    });

    let en: I18nTable = serde_json::from_str(EN_JSON).unwrap_or_else(|e| {
        // Phase-2: replace panic with fail-soft AppError path per ADR-0009 SD-4 Boot-Error-UX.
        panic!("i18n boot-fail: locales/en.json is not valid JSON: {e}")
    });

    Arc::new(en)
}
