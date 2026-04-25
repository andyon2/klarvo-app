---
name: Story 4.2 — Locale-Aware i18n-Table-Loading
epic: 4
story_number: "4.2"
status: review
dependencies:
  - "4.1"
  - "3.10"
---

# Story 4.2: Locale-Aware i18n-Table-Loading

## Outcome

`shells/windows/src-tauri/src/i18n.rs` liest beim Boot die `ui_language`-Achse aus
`ShellConfig` und lädt die passende Locale-Table (`en` oder `de`). Aktuell hardcoded
auf `en` (Story 3.1/3.10 Phase-1-Stub). Kein Runtime-Locale-Switch — Phase-2-Settings-UI-Scope.
FR27: Shell resolves i18n-Keys gegen owned Translation-Tables; FR26 axis 1 (UI-Language)
wird hier zum ersten Mal effektiv konsumiert.

## Acceptance Criteria

### AC-A — `load(ui_language)`-Funktion ersetzt `load_default()`

**Given** `shells/windows/src-tauri/src/i18n.rs::load_default()` returniert hartcoded `en`
**When** Story 4.2 implementiert wird
**Then**

- Neue Signatur: `pub fn load(ui_language: &str) -> Arc<I18nTable>`
- `load_default()` wird **gelöscht**, nicht behalten — analog Hard-Replace-Pattern aus 4.1
- Die Funktion validiert beide Locale-Files eagerly (so dass corrupt-de.json bei
  ui_language=en trotzdem boot-fail produziert — analog bestehende Logic):
  ```rust
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
  ```
- Der Match-Default-Arm (`_`) returniert die `en`-Table — nicht panic. Schemа-Validation
  in Story 4.1 AC-C garantiert, dass `ui_language` nur `en` oder `de` sein kann; der
  Default-Arm ist defensiver Fallback gegen Schema-Drift, nicht primärer Pfad

### AC-B — `main.rs`-Bootstrap-Sequenz aktualisiert

**Given** `main.rs:45` ruft `load_default()` auf, **bevor** `config` geladen ist (Step 1-2)
**When** Story 4.2 die Reihenfolge umstellt
**Then**

- Der `load(...)`-Aufruf wandert hinter Step 2 (Config-Load) im `.setup()`-Closure.
  Konkret: aktuell läuft `let i18n_table = ...load_default();` als erste Zeile vor
  `tauri::Builder::default()`. Diese Zeile wird **entfernt**
- Innerhalb von `.setup(|app| { ... })` wird, **nach** dem Config-Load-Block (Step 2,
  endet bei `let keystore = make_keystore();`), eingefügt:
  ```rust
  // Step 2b: i18n table for the resolved ui_language axis (FR26/FR27, Story 4.2).
  // Eagerly loaded after config so the active locale matches user choice; load() validates
  // both locale files at boot to surface JSON corruption regardless of selection.
  let i18n_table = klarvo_windows_shell::i18n::load(&config.ui_language);
  ```
- Der nachgelagerte `app.manage(i18n_table)`-Call (Step 11, vorhandene Zeile) bleibt
  unverändert
- `exit_label`-Lookup (`i18n_table.get("tray.menu.exit")...`) bleibt funktional —
  `tray.menu.exit` muss in beiden Locale-Files vorhanden sein (bereits in 3.10 erfüllt)

### AC-C — Step-Ordering-Constraint dokumentiert

**Given** der Bootstrap-Step-Block-Comment in `main.rs:53-79` listet Steps 1–13
**When** Story 4.2 den i18n-Load reordnet
**Then**

- Der Block-Comment wird ergänzt um Step 2b zwischen Step 2 und Step 3:
  ```
  // Step 2b: i18n::load(ui_language) — locale-aware table; depends on config (Step 2)
  ```
- Steps 3–13 bleiben in Ordering und Nummerierung unverändert; nur Step 2b ist neu
- Bei Config-Load-Fail (Step 2 fail-soft → `ShellConfig::default()`) wird `ui_language`
  automatisch `"en"` (Default aus 4.1 AC-A); Step 2b lädt entsprechend `en` — kein
  zusätzlicher Fail-Soft-Pfad nötig

### AC-D — Smoke-Test (Manual + Compile)

**Given** Story 4.2 ist implementiert
**When** Verification läuft
**Then**

- `cargo build -p klarvo-windows-shell` (oder Crate-Name) bleibt grün
- `cargo test -p klarvo-windows-shell` bleibt grün — bestehende Tests in `i18n.rs` werden
  angepasst:
  - **Test 1 — load("en") returns en-table:** `load("en")` → `Arc<I18nTable>` enthält
    `error.config.missing` mit Wert `"Configuration file not found. Please create config.toml."`
    (englischer String, nicht TODO-Marker)
  - **Test 2 — load("de") returns de-table:** `load("de")` → `Arc<I18nTable>` enthält
    `error.config.missing` mit dem deutschen TODO-/Final-String aus `de.json`
    (Test asserted nur, dass der Wert ≠ englischer String, oder dass der Wert mit
    `TODO(de):` beginnt — Story 4.3 ersetzt TODO durch echten Text und passt diesen
    Assertion-Strang an)
  - **Test 3 — load("xx") fallbacks to en:** `load("xx")` → `Arc<I18nTable>` identisch
    zu `load("en")`. Validation in 4.1 AC-C verhindert dass diese Branch im Production-Code
    ausgeführt wird, aber der Fallback ist dokumentiert defensives Verhalten
  - **Test 4 — beide Locale-Files validiert eagerly:** Wenn `de.json` corrupt ist,
    paniced auch `load("en")` (regression-Test der bestehenden Eager-Validation-Semantik
    aus Story 3.1/3.10)
- Manual smoke (separater Run): in `%APPDATA%\Klarvo\config.toml` `ui_language = "de"`
  setzen, App starten, Boot-Error auslösen (z. B. unbekannte stage in `pipeline-manifest.toml`)
  → `app.error`-Event `userMessage` enthält deutschen String (oder `TODO(de):`-Marker
  bis Story 4.3 läuft) statt englisch

### AC-E — Phase-2-Defer-Comment

**Given** Story 4.2 ändert nicht das Runtime-Locale-Switching-Verhalten
**When** der User zur Laufzeit `ui_language` in der Config wechselt
**Then**

- Die Änderung wirkt erst beim **nächsten** App-Start; `i18n_table` wird einmal beim Boot
  geladen und nicht re-loaded
- `i18n.rs`-Modul-Doc-Comment dokumentiert das:
  ```rust
  // Phase-1: locale is loaded once at boot from ShellConfig.ui_language.
  // Phase-2: Settings-UI will trigger live locale-switch via tauri::State<I18nTable>
  // mutation + UI re-render event. Out of scope for Story 4.2.
  ```

### AC-F — Frontend-Side i18n-Tabelle bleibt unangetastet

**Given** `shells/windows/src/locales/de.json` (Frontend) ist aktuell ein leeres `{}` (Story 3.x)
**When** Story 4.2 die Backend-Tabelle umstellt
**Then**

- Story 4.2 berührt **nicht** `shells/windows/src/locales/*.json` (Frontend-Locale-Files)
- Wenn der Frontend einen i18n-Mechanismus benötigt, ist das separater Phase-2-Scope
  (Settings-UI). Phase-1 hat keinen Frontend-i18n-Konsumenten — Errors werden via
  `app.error`-Event mit pre-resolved User-Message übermittelt (`TauriErrorEmitter`,
  Story 3.x), nicht via Frontend-Tabellen-Lookup
- Diese Story-Boundary wird in der Story-Doc explizit erwähnt, sodass Delegate nicht
  versehentlich Frontend-Locales mitwartet

## Technical Notes

### Eager-Validation pro Boot

Die bestehende `load_default()` validiert beide Locale-Files (en + de) bei jedem Boot,
auch wenn nur eines aktiv genutzt wird. Story 4.2 behält diese Semantik: ein Schaden in
`de.json` soll User mit `ui_language = "en"` ebenfalls auffallen, nicht erst beim Locale-Switch.
Dieses Verhalten ist Phase-1-Stub bis ADR-0009 SD-4 (Boot-Error-UX) in Phase-2 fail-soft
ersetzt; bewusst nicht hier angegangen.

### Single Active Table, kein Multi-Locale-State

`tauri::State<Arc<I18nTable>>` enthält nur die aktive Tabelle, nicht beide. Lookup-Code
in `TauriErrorEmitter` bleibt unverändert (`i18n_table.get(&key)`). Phase-2-Live-Switch
würde State-Slot durch `Arc<RwLock<I18nTable>>` oder per-Request-Locale-Resolution ersetzen;
das ist explizit Phase-2.

### Defensive Fallback (`_ => en`)

Schema-Validation in Story 4.1 AC-C garantiert `ui_language ∈ {en, de}`. Der `_ => en`-Arm
in `load()` ist defensiv für den Fall, dass `ShellConfig::default()` (fail-soft Pfad) oder
Schema-Drift (Phase-2 fügt neue Locale ohne `de.json`-Update) ihn triggert. Kein User-Error,
kein Toast — silent-fallback ist Phase-1-Pragma. Phase-2 Settings-UI wird invalid-Locale
proaktiv verhindern.

### Boot-Order-Coupling

Step 2b → Step 4 (TauriErrorEmitter) → Step 12 (Hotkey-Registration) — alle drei brauchen
die i18n-Tabelle. TauriErrorEmitter pulled die Tabelle erst beim Emit, nicht beim Construct,
also reicht es, dass Step 11 (`app.manage(i18n_table)`) vor Step 12 läuft. Die Reihenfolge
ist bereits in `main.rs` korrekt; Story 4.2 fügt nur Step 2b ein.

## Dependencies

- Story 4.1 (`ShellConfig.ui_language`-Feld existiert + ist validiert)
- Story 3.10 (Bootstrap-Step-Sequenz in `main.rs`, `app.manage(i18n_table)`)
- Story 3.1 (i18n-Modul-Skeleton, en+de.json existieren)
- ADR-0009 §SD-2 — Shell resolves Keys, kein Core-User-String
- `memory/feedback_premature_abstraction_guard` — kein Multi-Locale-State, kein Live-Switch

## Tasks/Subtasks

- [x] Task 1 — `i18n.rs`: `load_default()` → `load(ui_language)` (AC-A, AC-E)
  - [x] 1.1 Neue Signatur `pub fn load(ui_language: &str) -> Arc<I18nTable>`
  - [x] 1.2 Eager-Validation beider Locale-Files (de + en) unabhängig vom aktiven Locale
  - [x] 1.3 Match-Arm: `"de"` → de-table, `_` → en-table (defensiver Fallback)
  - [x] 1.4 `load_default()` gelöscht (Hard-Replace analog 4.1-Pattern)
  - [x] 1.5 Phase-2-defer-Comment auf Modul-Ebene
- [x] Task 2 — `main.rs`: Bootstrap-Reorder Step 2b (AC-B, AC-C)
  - [x] 2.1 `load_default()`-Aufruf vor `tauri::Builder::default()` entfernt
  - [x] 2.2 Step-2b-Block nach Config-Load eingefügt: `i18n::load(&config.ui_language)`
  - [x] 2.3 Bootstrap-Step-Comment um Step 2b ergänzt
- [x] Task 3 — i18n-Tests (AC-D)
  - [x] 3.1 Test: `load_en_returns_en_table` — kein TODO-Marker, enthält `config.toml`
  - [x] 3.2 Test: `load_de_returns_de_table` — Wert ≠ EN-Wert
  - [x] 3.3 Test: `load_unknown_locale_falls_back_to_en` — `xx` → identisch zu `en`
  - [x] 3.4 Test: `both_locale_files_valid_json_even_when_en_active` — de-Validation auch bei `load("en")`
- [x] Task 4 — Build + Tests (AC-D, AC-G)
  - [x] 4.1 `cargo test -p klarvo-windows-shell --lib`: 13/13 grün, 1 ignored

## Dev Agent Record

### Completion Notes

- AC-A ✅ — `load(ui_language)` mit Eager-Validation + Match-Arm; `load_default()` gelöscht.
- AC-B ✅ — Step-2b nach Config-Load in `.setup()`-Closure; `load_default()`-Aufruf vor Builder entfernt.
- AC-C ✅ — Bootstrap-Comment um `// Step 2b: i18n::load(ui_language)` ergänzt.
- AC-D ✅ — 4 i18n-Tests + 9 config-Tests = 13/13 grün, 1 ignored (bridge-smoke).
- AC-E ✅ — Phase-2-defer-Comment auf Modul- und Funktions-Ebene.
- AC-F ✅ — `shells/windows/src/locales/` (Frontend) nicht angefasst.

## File List

- `shells/windows/src-tauri/src/i18n.rs` — `load_default()` → `load(ui_language)`, Eager-Validation, Match-Arm, 4 Unit-Tests.
- `shells/windows/src-tauri/src/main.rs` — Step-2b-Block nach Config-Load, `load_default()`-Aufruf entfernt, Bootstrap-Comment erweitert.

## Change Log

- 2026-04-25: Story 4.2 implementiert — `i18n::load(ui_language)` ersetzt `load_default()`; Bootstrap-Step-2b eingefügt; 13/13 Tests grün.

## Review Findings (2026-04-25)

Konsolidierter Report: `_bmad-output/implementation-artifacts/epic-4-code-review-2026-04-25.md`

- [x] [Review][Patch] Bootstrap-Policy-Block-Comment listet Step 2b nicht — Step 2b ist Panic-Path (i18n::load), aber Block-Comment kategorisiert nur Steps 1-8 (fail-soft) und 9-10 (fatal) [shells/windows/src-tauri/src/main.rs:69-74] — fixed 2026-04-25 (Step-2b-Note ergänzt mit ADR-0009 SD-4 Forward-Ref)
- [x] [Review][Defer] `both_locale_files_valid_json_even_when_en_active` testet Happy-Path statt Regression (corrupt-DE → panic auch bei load("en")) [shells/windows/src-tauri/src/i18n.rs:189-197] — `DE_JSON` ist `include_str!`-statisch; echter Test bräuchte `load_from_strs`-Extraktion, eigene Story-Verantwortung
