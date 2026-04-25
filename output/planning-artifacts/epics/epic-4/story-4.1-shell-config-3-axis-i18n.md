---
name: Story 4.1 — ShellConfig 3-Axis i18n Schema
epic: 4
story_number: "4.1"
status: review
dependencies:
  - "3.2"
---

# Story 4.1: ShellConfig 3-Axis i18n Schema

## Outcome

`shells/windows/src-tauri/src/config.rs` ersetzt das einzelne `locale`-Feld durch drei
unabhängige i18n-Achsen `ui_language`, `dictionary_language`, `output_language` (FR26 +
`memory/project_i18n_three_axes`). Hard-Replace ohne Backwards-Compatibility-Alias —
v1-Live-Tester sind zurückgezogen (`memory/project_ea_withdrawn`); v1→v2-Migration ist
separater Epic-7-Scope. Phase-1 konsumiert nur `ui_language`; `dictionary_language` und
`output_language` sind forward-looking Schema-Felder, die von Phase-2-Plugins gelesen werden.

## Acceptance Criteria

### AC-A — Schema-Replacement: drei unabhängige Felder

**Given** `ShellConfig` aus Story 3.2 enthält ein einziges `locale: String`-Feld
**When** Story 4.1 implementiert wird
**Then**

- `ShellConfig` ist deklariert als:
  ```rust
  #[derive(Debug, Clone, serde::Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct ShellConfig {
      #[serde(default = "ShellConfig::default_hotkey")]
      pub hotkey: String,
      #[serde(default = "ShellConfig::default_output_target")]
      pub output_target_id: String,
      #[serde(default = "ShellConfig::default_ui_language")]
      pub ui_language: String,
      #[serde(default = "ShellConfig::default_dictionary_language")]
      pub dictionary_language: String,
      #[serde(default = "ShellConfig::default_output_language")]
      pub output_language: String,
  }
  ```
- Defaults:
  - `default_ui_language() -> "en"`
  - `default_dictionary_language() -> "en"`
  - `default_output_language() -> "en"`
- `default_locale()` aus Story 3.2 wird **gelöscht**, nicht behalten
- Das alte `locale: String`-Feld ist vollständig entfernt aus der Struct-Definition und
  `Default`-Impl

### AC-B — Hard-Replace ohne Compat-Alias

**Given** Hard-Replace-Entscheidung (Andy 2026-04-25, Story-Brief)
**When** ein User eine `config.toml` mit dem alten Feld `locale = "de"` lädt
**Then**

- `#[serde(deny_unknown_fields)]` führt dazu, dass `locale` als unbekanntes Feld erkannt
  wird → `AppError { kind: Configuration, user_message: Some("error.config.unknown_field"), ... }`
- Es gibt **keinen** Serde-Alias `#[serde(alias = "locale")]` und **keine** Migration-Logik
  in `parse_from_str`
- Rustdoc auf `ShellConfig` enthält:
  `// Story 4.1: replaced single 'locale' field with ui_language/dictionary_language/output_language.`
  `// v1→v2 config-migration is Epic 7 scope; Phase-1 has no live testers (ref project_ea_withdrawn).`

### AC-C — Validation pro Achse

**Given** `parse_from_str` validiert Locale-Werte nach erfolgreichem Serde-Deserialize
**When** ein nicht-`en|de`-Wert in einem der drei Felder steht
**Then**

- Whitelist-Check pro Feld:
  ```rust
  for (field_name, value) in [
      ("ui_language", &config.ui_language),
      ("dictionary_language", &config.dictionary_language),
      ("output_language", &config.output_language),
  ] {
      if !matches!(value.as_str(), "en" | "de") {
          return Err(AppError {
              kind: AppErrorKind::Configuration,
              message: format!("unsupported {field_name}: {value}"),
              user_message: Some("error.config.invalid_language".to_string()),
              retryable: false,
          });
      }
  }
  ```
- **Neuer i18n-Key:** `error.config.invalid_language` ersetzt `error.config.invalid_locale`
  semantisch (3-Achsen-Begriff statt Single-Locale-Begriff). Der alte Key
  `error.config.invalid_locale` wird in 4.4 (Coverage-Audit) deprecated; Story 4.1 fügt
  den neuen Key in `locales/en.json` + `locales/de.json` hinzu, den alten Key zu entfernen
  ist Scope von 4.4
- Die `AppError.message` enthält den Field-Namen, sodass Logs identifizieren, welche der
  drei Achsen den Reject ausgelöst hat (`AppError.user_message` bleibt generic, da Shell
  nicht zwischen den drei Achsen formuliert — Phase-1 zeigt einen einzigen Toast)

### AC-D — Unit-Tests aktualisiert + erweitert

**Given** Story 3.2 hatte 5 Tests in `config.rs::tests`
**When** Story 4.1 die Schema-Erweiterung committed
**Then**

- Test 1 — Happy-Path leeres TOML: alle drei Language-Felder sind `"en"`
- Test 2 — Happy-Path explicit:
  ```toml
  hotkey = "CommandOrControl+Shift+Space"
  output_target_id = "clipboard"
  ui_language = "de"
  dictionary_language = "en"
  output_language = "de"
  ```
  → Ok mit unabhängig gesetzten Werten
- Test 3 — Unknown-Field rejected: `unknown_key = "Y"` → existing test passes (regression)
- Test 4 — **Geändert:** `locale = "fr"` → `Err` mit `user_message ==
  Some("error.config.unknown_field")` (weil `locale` jetzt unknown ist), **nicht** mehr
  `error.config.invalid_locale`
- **Neuer Test 4a — Invalid-UI-Language rejected:** `ui_language = "fr"` → `Err` mit
  `user_message == Some("error.config.invalid_language")` und `message` enthält `"ui_language"`
- **Neuer Test 4b — Invalid-Dictionary-Language rejected:** `dictionary_language = "es"`
  → `Err` mit `user_message == Some("error.config.invalid_language")` und `message`
  enthält `"dictionary_language"`
- **Neuer Test 4c — Invalid-Output-Language rejected:** `output_language = "it"`
  → analog 4b mit `"output_language"` im Message
- **Neuer Test 4d — Mixed-Languages happy-path:** `ui_language = "de"`,
  `dictionary_language = "en"`, `output_language = "de"` → Ok (drei Achsen sind unabhängig
  setzbar; kein impliziter Cross-Check)
- Test 5 — Missing-File: existing test passes (regression)

### AC-E — i18n-Key-Registration

**Given** Story 4.1 fügt `error.config.invalid_language` als neuen Key ein
**When** die Story committed wird
**Then**

- `locales/en.json` enthält:
  ```json
  "error.config.invalid_language": "Unsupported language. Supported values: en, de."
  ```
- `locales/de.json` enthält den gleichen Key mit TODO-Marker (echte Übersetzung in 4.3):
  ```json
  "error.config.invalid_language": "TODO(de): Unsupported language. Supported values: en, de."
  ```
- Der bestehende `error.config.invalid_locale`-Key bleibt unverändert in beiden Files —
  Cleanup ist Scope von Story 4.4 (Coverage-Audit)

### AC-F — Phase-1 Konsumenten-Surface

**Given** Phase-1 hat keinen Plugin der `dictionary_language` oder `output_language` liest
**When** Story 4.1 committed ist
**Then**

- `dictionary_language` und `output_language` sind in `tauri::State<Arc<ShellConfig>>`
  verfügbar (über die bestehende Bootstrap-Step-11-`app.manage`-Registrierung in `main.rs`),
  werden aber von keinem Phase-1-Plugin konsumiert
- Die Felder sind **nicht** versteckt hinter einem `#[cfg(feature = ...)]`-Gate; sie sind
  Teil des öffentlichen Schemas
- Rustdoc auf den beiden Feldern dokumentiert den Forward-Looking-Status:
  ```rust
  /// Phase-1: not consumed by any plugin. Phase-2+ will route this into
  /// dictionary-aware STT/cleanup plugins (PRD FR26 axis 2).
  pub dictionary_language: String,
  ```
  analog für `output_language` mit Hinweis auf Cleanup-Stage

### AC-G — `main.rs`-Bootstrap kompatibel

**Given** `main.rs` Step 1-2 lädt `ShellConfig` und Step 11 macht `app.manage(Arc::new(config.clone()))`
**When** Story 4.1 das Schema erweitert
**Then**

- `main.rs` braucht keine Anpassung — `ShellConfig::default()` und `load_config` returnieren
  weiterhin den gleichen Typ, nur die Innen-Felder sind anders
- `config.output_target_id` (Step 10 `SessionOrchestrator::new`) bleibt unverändert verwendet
- `config.hotkey` (Step 12 `register_hotkey`) bleibt unverändert verwendet
- Story 4.2 wird `config.ui_language` für Locale-Loading konsumieren — diese Konsumption
  ist nicht Scope von 4.1
- `cargo build -p klarvo-windows-shell` (oder Crate-Name) bleibt grün

## Technical Notes

### Hard-Replace-Rationale

Memory `project_ea_withdrawn` (2026-04-14): Keine aktiven v1-Tester mehr, kein Release-Druck.
v1→v2-Config-Migration ist explizit Epic 7 Scope (`epics.md:2263 Epic 7 V1→V2 Data Migration
Path`). Soft-Aliases (`#[serde(alias = "locale")]`) wären Premature-Compatibility (
`feedback_premature_abstraction_guard`) — sie würden Migrations-Code aus Epic 7 vorwegnehmen
ohne klaren Konsumenten in Phase-1.

### Three-Axis Independence

`memory/project_i18n_three_axes`: UI-Language (Shell-Strings) / Dictionary-Language
(Plugin-Dictionary-Lookups z. B. STT-Hint-Words) / Output-Language (Cleanup-Target-Language).
Kein implizites Cross-Field-Default — wenn User nur `ui_language = "de"` setzt, bleiben
`dictionary_language` und `output_language` auf `"en"` (Default), nicht auto-promoted.
User der alle drei deutsch will, muss alle drei explizit setzen. Dieser
Über-Konfigurations-Aufwand ist Phase-2-Settings-UI-Scope; Phase-1 ist Sanity-Tester-Targeted
(Memory: `feedback_skip_with_rationale` — keine UX-Optimierung vor MVP-Validation).

### Whitelist statt Enum

Analog Story 3.2 Technical-Note: 2 Locales (`en|de`) in 3 Feldern = 6 Variant-Slots, Enum
würde gegen `feedback_premature_abstraction_guard` verstoßen. Phase-2 (mehr Locales) ist
der natürliche Enum-Upgrade-Moment.

### `error.config.invalid_language` neuer Key, nicht Rename

Renaming `error.config.invalid_locale` → `error.config.invalid_language` würde Story 4.1 in
Doppelt-Touchpoint zu Story 4.4 (Coverage-Audit) verschränken. Story 4.1 fügt den neuen Key
hinzu; alter Key wird in 4.4 entfernt (zusammen mit anderen Aufräum-Touches). Trennt
Schema-Add von Schema-Cleanup.

## Dependencies

- Story 3.2 (`ShellConfig`-Struct + `parse_from_str` + `load_config` existieren)
- `memory/project_i18n_three_axes` — drei unabhängige Achsen, kein Single-Locale-Field
- `memory/project_ea_withdrawn` — keine v1-Tester, Hard-Replace ohne Migration-Alias OK
- `memory/feedback_premature_abstraction_guard` — kein Enum, kein `#[serde(alias)]`
- ADR-0009 §SD-2 — Shell resolved i18n-Keys (Konsumption-Pfad bleibt stabil)

## Tasks/Subtasks

- [x] Task 1 — `ShellConfig`-Schema umbauen (AC-A, AC-B, AC-F)
  - [x] 1.1 `ui_language`/`dictionary_language`/`output_language` als `pub` Felder mit `#[serde(default = "...")]`
  - [x] 1.2 Defaults `default_ui_language`/`default_dictionary_language`/`default_output_language` jeweils `"en"`
  - [x] 1.3 `locale`-Feld + `default_locale()` löschen
  - [x] 1.4 `Default`-Impl auf neue Felder umstellen
  - [x] 1.5 Rustdoc auf `dictionary_language` + `output_language` (Phase-1 forward-looking, kein Konsument)
  - [x] 1.6 Rustdoc auf `ShellConfig` mit Hard-Replace-Note (Verweis auf Epic 7 Migration)
- [x] Task 2 — Validation pro Achse (AC-C)
  - [x] 2.1 `parse_from_str` Loop über drei Felder mit `matches!("en"|"de")`
  - [x] 2.2 Field-Name in `AppError.message`, `user_message = "error.config.invalid_language"`
- [x] Task 3 — i18n-Key registrieren (AC-E)
  - [x] 3.1 `error.config.invalid_language` in `locales/en.json` (echter EN-String)
  - [x] 3.2 `error.config.invalid_language` in `locales/de.json` mit `TODO(de):`-Marker
- [x] Task 4 — Unit-Tests anpassen + erweitern (AC-D)
  - [x] 4.1 Test 1 (`happy_path_empty_toml_uses_defaults`) — drei Languages = `"en"`
  - [x] 4.2 Test 2 (`happy_path_explicit_values`) — alle drei Sprachfelder explizit gesetzt
  - [x] 4.3 Test 4 (`legacy_locale_field_rejected_as_unknown_field`) — Hard-Replace-Effekt: `locale` ist jetzt unknown_field
  - [x] 4.4 Test 4a (`invalid_ui_language_rejected`) — `error.config.invalid_language` + `message.contains("ui_language")`
  - [x] 4.5 Test 4b (`invalid_dictionary_language_rejected`) — analog
  - [x] 4.6 Test 4c (`invalid_output_language_rejected`) — analog
  - [x] 4.7 Test 4d (`mixed_languages_independent_axes`) — `de`/`en`/`de` happy-path
- [x] Task 5 — Build + Tests verifizieren (AC-G)
  - [x] 5.1 `cargo build -p klarvo-windows-shell --lib` grün (1m 11s, kein Warning)
  - [x] 5.2 `cargo test -p klarvo-windows-shell --lib` grün, 9/9 Tests in `config::tests` pass, 0 failed, 1 ignored (existing bridge-manual-smoke)
  - [x] 5.3 `main.rs`-Bootstrap unverändert — `config.output_target_id` (Step 10) + `config.hotkey` (Step 12) sind die einzigen Konsumenten; keine `locale`-Reads im Tree

## Dev Agent Record

### Implementation Plan

1. Schema-Umbau in `shells/windows/src-tauri/src/config.rs`: drei `pub`-Felder + drei `default_*_language()`-Helper, Default-Impl angepasst, `locale` + `default_locale()` ersatzlos gestrichen.
2. Per-Axis-Validation: `for (field_name, value) in [...]`-Loop über die drei Achsen, `matches!(value.as_str(), "en" | "de")`-Whitelist, `AppError.message` mit Feld-Name (für Log-Forensik), `user_message = "error.config.invalid_language"`.
3. Locale-Files: neuer Key `error.config.invalid_language` in `en.json` (echter EN-String) und `de.json` (mit `TODO(de):`-Prefix für Story 4.3-Cleanup). `error.config.invalid_locale` bleibt unverändert in beiden Files (Cleanup ist Story 4.4).
4. Tests: 4 alte Tests adaptiert + 4 neue Tests; alter `invalid_locale_rejected` in `legacy_locale_field_rejected_as_unknown_field` umbenannt, da der Hard-Replace-Effekt strukturell anders ist (Serde-Reject statt Post-Parse-Validation-Reject).

### Completion Notes

- AC-A ✅ — Drei `pub`-Felder mit unabhängigen Defaults, alter `locale`-Slot entfernt.
- AC-B ✅ — Hard-Replace ohne `#[serde(alias)]`-Shim; Rustdoc-Note auf `ShellConfig` verweist auf Epic 7 + `memory/project_ea_withdrawn`.
- AC-C ✅ — Per-Axis-Whitelist-Loop in `parse_from_str`; Test 4a/4b/4c assertet pro Achse, dass `message` den Feld-Namen enthält (Log-Forensik).
- AC-D ✅ — 9/9 Tests grün: 5 adaptiert (Test 1/2 erweitert auf 3 Felder, Test 4 zu `unknown_field`-Effekt umgemünzt) + 4 neu (4a/4b/4c/4d).
- AC-E ✅ — `error.config.invalid_language` in beiden Locale-Files; `error.config.invalid_locale` bleibt für Story 4.4-Cleanup.
- AC-F ✅ — Rustdoc auf `dictionary_language` + `output_language` mit Phase-1-Forward-Looking-Note + PRD-FR26-Achsen-Referenz.
- AC-G ✅ — `main.rs`-Bootstrap kompiliert ohne Touch (verifiziert via `grep config\.locale` — keine Treffer); `cargo build -p klarvo-windows-shell --lib` grün.

### Notes für nachgelagerte Stories

- **Story 4.2** wird `i18n::load(&config.ui_language)` einführen und Step 2b im Bootstrap anlegen — die Schema-Voraussetzung dafür ist mit dieser Story erfüllt.
- **Story 4.4** entfernt den jetzt verwaisten `error.config.invalid_locale`-Key aus beiden Locale-Files; Coverage-Test prüft das mechanisch via `REQUIRED_KEYS`-Whitelist.
- **`shells/windows/src/locales/de.json`** (Frontend, leeres `{}`) wurde nicht angefasst — Phase-1 hat keinen Frontend-i18n-Konsumenten (Story 4.2 AC-F).

## File List

- `shells/windows/src-tauri/src/config.rs` — `ShellConfig`-Schema auf 3 Achsen erweitert; `parse_from_str` mit Per-Axis-Whitelist-Loop; 9 Unit-Tests adaptiert/ergänzt.
- `shells/windows/locales/en.json` — neuer Key `error.config.invalid_language` mit EN-String.
- `shells/windows/locales/de.json` — neuer Key `error.config.invalid_language` mit `TODO(de):`-Marker (Story 4.3 ersetzt).

## Change Log

- 2026-04-25: Story 4.1 implementiert — `ShellConfig` mit drei unabhängigen i18n-Achsen (`ui_language`/`dictionary_language`/`output_language`); Hard-Replace ohne Compat-Alias; `error.config.invalid_language` in beiden Locale-Files; 9 Unit-Tests grün; `main.rs`-Bootstrap unverändert.

## Review Findings (2026-04-25)

Konsolidierter Report: `_bmad-output/implementation-artifacts/epic-4-code-review-2026-04-25.md`

- [x] [Review][Patch] Stale Rustdoc-Tabelle in `load_config` referenziert `error.config.invalid_locale` — Code emittiert `error.config.invalid_language` [shells/windows/src-tauri/src/config.rs:150] — fixed 2026-04-25
- [x] [Review][Defer] TOML-Type-Mismatch (`ui_language = 42` etc.) wird auf `error.config.missing` aliased — semantisch falsche User-Message [shells/windows/src-tauri/src/config.rs:103-116] — pre-existing aus Story 3.2-Branch-Logic, Phase-2-Settings-UI deckt das natürlich ab

## Status

review
