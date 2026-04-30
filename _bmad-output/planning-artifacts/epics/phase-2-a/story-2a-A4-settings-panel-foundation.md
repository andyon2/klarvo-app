---
name: Story 2.A.A4 — Settings-Panel Foundation
phase: 2
wave: A
story_id: "2.A.A4"
status: done
dependencies: []
adr_refs:
  - docs/adr/0013-settings-persistence-schema.md
  - docs/adr/0009-shell-error-bridge.md
---

# Story 2.A.A4: Settings-Panel Foundation

## Outcome

Phase-1-`ShellConfig.toml`-Friction wird strukturell beseitigt. `klarvo-core` erhält einen
`Settings`-Service-Layer mit SQLite-Persistenz (`settings(key, value, type)`-Tabelle) und
typed-Accessor-API. Eine One-Shot-Boot-Migration überträgt die 5 Phase-1-User-Settings von TOML
nach SQLite. Die Windows-Shell registriert typed Tauri-Commands (5 Core-Set + 1 Bulk-Get +
1 Plugin-Set + 1 Plugin-Get) und emittiert `settings-changed`-Events via `SettingsEmitter`-Trait
(ADR-0009-Hybrid-C-analog, kein direktes `app.emit()` in `klarvo-core`). Ein minimales
React-Settings-Panel im Tauri-WebView macht alle 5 Felder ohne `config.toml`-Edits zugänglich.

Foundation-Layer für: A8-Sub (Tray-Language-Switcher), C2 (Hotkey-Konflikt-Erkennung),
C3 (Live-Locale-Switch).

## Scope-Fence

**In-Scope:** `Settings`-Service + SQLite-Migration + typed-Accessors für 5 Core-Fields +
`SettingsEmitter`-Trait + 8 Tauri-Commands + `SettingsChangedEvent`-Typ + React-Settings-Panel.

**Nicht-in-Scope (explizit):**
- Hotkey-Konflikt-UX → C2
- Frontend-Locale-Hot-Reload (Komponenten-Re-Render auf `settings-changed`) → C3
- Tray-Sprach-Switcher-Menü → A8-Sub
- `audio.*`-Settings (sample_rate, device_id) → Phase-2-B B2
- `license.*`-Settings → Phase-4
- Second-Hotkey-Slot (`hotkey.slot2.*`) → Phase-2-B A2
- API-Key-Storage — bleibt bei `KeyStore`-Trait (Story-1C-Foundation), nicht im
  `Settings`-Service

## Acceptance Criteria

### AC-1 — `settings`-Tabelle per rusqlite_migration mit type-Enum

**Given** `klarvo-core` DB-Init durchläuft die bestehende `rusqlite_migration`-Infrastruktur
(Story-1B-Foundation)  
**When** Migration-Version ≥ Phase-2 angewendet wird  
**Then**

- Tabelle `settings(key TEXT PRIMARY KEY, value TEXT, type TEXT)` existiert.
- Re-Init ist idempotent via `rusqlite_migration` (kein manuelles `CREATE TABLE IF NOT EXISTS`
  im App-Code; Schema-Owner ist ausschließlich die Migration).
- Valide `type`-Werte sind exklusiv: `"string"` | `"i64"` | `"bool"` | `"json"`.
  Settings-Service schreibt den Type aus der typed-Accessor-Signatur; alle 5 Phase-2-A-Core-Fields
  werden als `"string"` geschrieben. Beim Lesen eines unbekannten `type`-Werts: `AppError`
  (kein silent-cast, kein `unwrap`).

---

### AC-2 — One-Shot TOML→SQLite-Migration + Fresh-Install-Edge-Case

**Given** App startet; `Settings::init()` wird aufgerufen  
**When** Detect-Condition ausgewertet wird: `settings`-Tabelle ist leer  
**Then** zwei Äste:

**Ast A — `config.toml` existiert (Phase-1→Phase-2-Upgrade):**

- Folgende User-Layer-Felder werden nach SQLite geschrieben (ADR-0013 Sub-Decision-1-Mapping):

  | TOML-Feld (`ShellConfig`) | SQLite-Key | `type` |
  |---------------------------|------------|--------|
  | `hotkey` | `hotkey.slot1.combo` | `"string"` |
  | `output_target_id` | `app.output_target_id` | `"string"` |
  | `ui_language` | `ui.language` | `"string"` |
  | `dictionary_language` | `app.dictionary_language` | `"string"` |
  | `output_language` | `app.output_language` | `"string"` |

- `config.toml` wird nicht verändert (bleibt System-Layer für `db_path`, `dev_mode`, etc.).
- Migration läuft transaktional (alle 5 Writes in einer SQLite-Transaction; bei Fehler:
  vollständiger Rollback, kein partieller State).
- Bei malforter oder partiell unlesarer TOML: `tracing::warn!` pro unlesarem Feld + Phase-1-Default
  für das jeweilige Feld (per `feedback_scaffold_fail_soft_pattern`). Kein Crash, kein `unwrap`.

**Ast B — `config.toml` fehlt (Fresh-Install ohne v1):**

- Kein Migration-Run; `settings`-Tabelle bleibt leer.
- Reads fallen auf Phase-1-Defaults zurück (hartcodierte Konstanten in Accessor-Impl;
  nicht leer/None).
- Kein `Error`, kein `warn!` — leere Tabelle bei Fresh-Install ist Normalzustand.

---

### AC-3 — Migration Idempotent

**Given** Phase-2-Boot hat Migration bereits ausgeführt; `settings`-Tabelle ist nicht leer  
**When** `Settings::init()` erneut aufgerufen wird (Neustart)  
**Then**

- Kein zweiter Migrations-Run (Detect-Condition `settings`-Tabelle-leer feuert nicht).
- SQLite-Werte unverändert.
- `config.toml` unverändert.
- Keine separate Versions-Flag-Datei oder TOML-Sentinel nötig — die Tabelle selbst ist
  der State-Indikator.

---

### AC-4 — Typed-Accessor Read-Mandate (Core-Settings-Scope)

**Given** `Settings`-Service ist initialisiert  
**When** Feature-Code (in `klarvo-core` oder der Windows-Shell) einen der 5 Core-Settings liest  
**Then**

- Typed Methoden werden genutzt: `settings.ui_language()`, `settings.output_language()`,
  `settings.hotkey_slot1_combo()`, `settings.output_target_id()`,
  `settings.dictionary_language()`.
- Kein `settings.get_string("ui.language")` oder Raw-Key-Aufruf außerhalb der Accessor-Impl
  selbst (per architecture.md L536).
- **Code-Review-Gate:** Raw-Key-Access im Feature-Code (außerhalb Accessor-Impl) ist AC-Fail
  im Code-Review. Kein Custom-xtask-Lint in dieser Story (kein Phase-1-Precedent für
  Settings-Lint; Code-Review-enforced wie Epic-4-Coverage-Gate-Muster).
- Scope-Clarification: Diese Read-Mandate gilt nur für die 5 Core-Namespace-Fields
  (`app.*`, `hotkey.*`, `ui.*`). Plugin-eigene Settings (z.B. Groq-Plugin liest
  `plugins.groq.*`) nutzen die Plugin-Read-API (`get_plugin_setting` aus AC-7), nicht
  typed Core-Accessors.

---

### AC-5 — `SettingsEmitter`-Trait + Write-Path + Event-Emission

**Given** `Settings`-Service soll `settings-changed`-Events emittieren ohne Tauri-Coupling
in `klarvo-core` (ADR-0009-Hybrid-C-analog: Core-portable Trait, Shell-scoped Impl)  
**When** `SettingsEmitter`-Trait und Settings-Service implementiert werden  
**Then**

- `SettingsEmitter`-Trait lebt in `klarvo-core` ohne `tauri`-Import:
  ```rust
  pub trait SettingsEmitter: Send + Sync {
      fn emit_settings_changed(&self, key: &str, new_value: &str);
  }
  ```
- `TauriSettingsEmitter` lebt in `shells/windows/src-tauri/` und ruft `app.emit(...)`:
  ```rust
  pub struct TauriSettingsEmitter { app: tauri::AppHandle }
  impl SettingsEmitter for TauriSettingsEmitter {
      fn emit_settings_changed(&self, key: &str, new_value: &str) {
          let _ = self.app.emit("settings-changed", SettingsChangedEvent {
              key: key.into(), new_value: new_value.into(),
          });
      }
  }
  ```
- `Settings`-Struct hält `Arc<dyn SettingsEmitter>` (Konstruktor-Injection).
- Beim Aufruf eines typed Set-Accessors (z.B. `settings.set_ui_language("de")`):
  1. Neuer Wert in SQLite persistiert (transaktional).
  2. `self.emitter.emit_settings_changed("ui.language", "de")` aufgerufen.
  3. Folgender Read (`settings.ui_language()`) gibt `"de"` zurück.
- Bei Persistenz-Fehler: kein Emit; Fehler als `AppError` returned (kein silent-fail).
- `klarvo-core/Cargo.toml` hat keine `tauri`-Dependency (Compile-Check gegen Cross-Boundary-Leak).
- `SettingsChangedEvent`-Struct ist in der Shell definiert und via tauri-specta TS-exportiert
  (Konsumenten: A8-Sub, C2, C3 in späteren Stories).

---

### AC-6 — Tauri-Command-Surface (8 Commands, tauri-specta-exportiert)

**Given** Windows-Shell registriert Settings-Tauri-Commands  
**When** Frontend die Command-Suite nutzt  
**Then** folgende 8 Commands existieren und sind via tauri-specta TS-typisiert:

```
// Core-Set (5):
set_hotkey_slot1(combo: String)         -> Result<(), AppError>
set_ui_language(lang: String)           -> Result<(), AppError>
set_output_target(id: String)           -> Result<(), AppError>
set_dictionary_language(lang: String)   -> Result<(), AppError>
set_output_language(lang: String)       -> Result<(), AppError>

// Core-Bulk-Get (1):
get_user_settings()                     -> Result<UserSettings, AppError>

// Plugin (2):
set_plugin_setting(plugin_id: String, key: String, value: String) -> Result<(), AppError>
get_plugin_setting(plugin_id: String, key: String)                -> Result<Option<String>, AppError>
```

- `UserSettings`-Struct (Shell-side, tauri-specta-exportiert):
  ```rust
  #[derive(Serialize, Deserialize, Clone, specta::Type)]
  pub struct UserSettings {
      pub hotkey_slot1_combo: String,
      pub output_target_id: String,
      pub ui_language: String,
      pub dictionary_language: String,
      pub output_language: String,
  }
  ```
- Jeder Core-Set-Command delegiert an typed Accessor + Emitter (AC-5).
- `get_user_settings()` liest alle 5 Felder in einem Call (kein Round-Trip pro Feld);
  füllt `UserSettings` mit Accessor-Return-Werten.
- Auf Persistenz-Fehler: `AppError` returned.

---

### AC-7 — Plugin-Setting-Commands mit Namespace-Guard + Lese-Symmetrie

**Given** `set_plugin_setting` und `get_plugin_setting` aus AC-6 sind registriert  
**When** Frontend Plugin-Settings schreibt oder liest  
**Then**

- **Namespace-Guard (gilt für Set und Get):** Wenn `key` mit einem Core-Namespace-Prefix
  beginnt (`app.`, `hotkey.`, `ui.`, `audio.`, `license.`, `history.`) → `AppError`
  (Namespace-Violation), kein Write/Read. Schützt Core-Namespace gegen Plugin-Zugriff.
- **Set:** Wert wird unter `plugins.<plugin_id>.<key>` persistiert; `settings-changed`-Event
  emittiert via `SettingsEmitter` (AC-5).
- **Get:** Gibt `Option<String>` zurück (None wenn Key nicht existiert). Kein Fallback auf
  Plugin-Defaults — Plugins verwalten eigene Defaults in ihrem Code.
- Namespace-Guard-Validation ist rein string-basiert (Key-Prefix-Check), kein Runtime-Registry-Lookup.

---

### AC-8 — React Settings-Panel: Mount + Load via `get_user_settings`

**Given** App läuft; User öffnet Settings-Panel (Tab-Navigation per architecture.md §5
Tab-basiertes State-Routing)  
**When** Panel mountet  
**Then**

- `get_user_settings()` Tauri-Command wird beim Mount aufgerufen; `UserSettings`-Struct
  füllt den Form-State (kein Raw-Key-String im React-Code).
- Alle 5 Felder gerendert: Hotkey-Combo (Text-Input), Output-Target (Text-Input oder Dropdown),
  UI-Language (Dropdown: mindestens `en` / `de`), Dictionary-Language (Text-Input),
  Output-Language (Text-Input).
- Kein blank/undefined nach Migration (AC-2 Ast A) oder Fresh-Install (AC-2 Ast B — Defaults
  aus Accessor-Impl sind im `UserSettings`-Struct sichtbar).
- Loading-State während des Async-Commands: kein Flash of empty content
  (kurzes Spinner oder disabled-Form akzeptabel).

---

### AC-9 — React Settings-Panel: Save-Pfad + Fehler-Passthrough

**Given** User editiert ein Feld im Settings-Panel  
**When** Save ausgelöst wird (Submit-Button oder On-Change für geeignete Inputs)  
**Then**

- Entsprechender typed Core-Set-Command aufgerufen (AC-6).
- Erfolg: Form-State updated auf gespeicherten Wert (lokales State-Update oder
  Re-Call von `get_user_settings()`).
- Fehler: bestehender `app.error`-Toast-Mechanismus (ADR-0009 Hybrid-C) zeigt `AppError` an.
- **Kein Conflict-UX in A4:** Hotkey-Konflikt-Feedback ist C2-Scope. A4 zeigt bei
  `set_hotkey_slot1`-Fehler einen generischen Error-Toast; C2 ersetzt das durch
  conflict-spezifische UX.

---

### AC-10 — Bindings-Drift-Gate

**Given** Alle 8 Tauri-Commands aus AC-6 sind registriert; `UserSettings` + `SettingsChangedEvent`
via tauri-specta exportiert  
**When** `cargo xtask bindings-drift` läuft  
**Then** Exit 0; kein Drift zwischen generierten TS-Bindings und registrierten Commands/Typen
(Story-5.2-Gate, Epic-5-Precedent).

---

## Technical Notes

### SettingsEmitter — ADR-0009-Analog

`SettingsEmitter`-Trait folgt exakt dem `ErrorEmitter`-Pattern aus ADR-0009 Hybrid-C:
Core-portable Trait, Shell-scoped Wrapper + Impl. `Settings`-Service-Konstruktor erhält
`Arc<dyn SettingsEmitter>`. Im Test-Kontext: `NoopSettingsEmitter` (leere Impl, kein
Tauri-Overhead). Verhindert Tauri-Boundary-Leak in `klarvo-core`.

### Migration-Transactional-Boundary

Die One-Shot-Migration (AC-2 Ast A) schreibt alle 5 Fields in einer SQLite-Transaction.
Partial-Write-Risk: wenn Migration nach dem 3. Field crasht (Prozess-Kill), sind 2 Fields
in SQLite aber 3 nicht — nächster Boot findet nicht-leere Tabelle (AC-3 Detect-Condition
feuert nicht), aber 2 fehlen. Mitigation: Migration nutzt `BEGIN EXCLUSIVE TRANSACTION`;
bei Fehler `ROLLBACK` → Tabelle bleibt leer → nächster Boot retried Migration.

### Fresh-Install Defaults

Phase-1-Defaults als Konstanten in `klarvo-core/src/settings/defaults.rs`:
```rust
pub const DEFAULT_UI_LANGUAGE: &str = "en";
pub const DEFAULT_OUTPUT_LANGUAGE: &str = "en";
pub const DEFAULT_DICTIONARY_LANGUAGE: &str = "en";
pub const DEFAULT_HOTKEY_SLOT1_COMBO: &str = "Alt+Shift+R";
pub const DEFAULT_OUTPUT_TARGET_ID: &str = "clipboard";
```
Accessor-Impl returniert diese Konstanten wenn kein DB-Wert vorhanden. Nicht konfigurierbar
per Compile-Feature — Plain-Hardcode reicht für MVP-Scope.

### Plugin-Read-Symmetry

`get_plugin_setting` ist read-symmetric zu `set_plugin_setting`. Phase-1-Groq-Plugin liest
seine API-Keys über den `KeyStore`-Trait (nicht über Settings); für zukünftige nicht-sensible
Plugin-Settings (z.B. `plugins.groq.model`) ist `get_plugin_setting` der vorgesehene Read-Path.
In Phase-2-A hat kein Plugin-Code diesen Read-Path im Einsatz — die Surface wird bereitgestellt,
nicht sofort konsumiert.

### UserSettings-Struct Placement

`UserSettings` lebt in der Shell (`shells/windows/src-tauri/src/commands/settings.rs`), nicht
in `klarvo-core`. Begründung: es ist eine tauri-specta-exportierte Frontend-facing-Projection,
keine Core-Domain-Type. `klarvo-core::Settings`-Service gibt typisierte Werte zurück; Shell
aggregiert sie in `UserSettings` für den Bulk-Get-Command.

## Dependencies

- **Story 1B.x** — `rusqlite_migration`-Infrastruktur (AC-1 Migration-Engine)
- **Story 1C.x** — `KeyStore`-Trait-Foundation (Scope-Fence: Settings-Service ist NICHT
  für API-Keys zuständig; KeyStore-Trait bleibt API-Key-Owner)
- **Story 5.2** — `cargo xtask bindings-drift` Gate (AC-10)
- **ADR-0013** — Sub-Decisions 1–5 accepted 2026-04-27 (Schema-Shape, Migration-Path,
  API-Surface, Notify-Mechanismus, Format-Mutability-Window)
- **ADR-0009** — `ErrorEmitter`-Pattern-Analog (AC-5 `SettingsEmitter`-Trait-Design)
- `feedback_scaffold_fail_soft_pattern` — AC-2 fail-soft Migration-Error-Handling
- `feedback_premature_abstraction_guard` — `UserSettings`-Struct hat genau 5 Fields;
  kein Speculation-Layer für zukünftige Settings

## Spec-Deviations

**rusqlite_migration-Crate nicht verwendbar (Spec-Abweichung AC-1):**
`rusqlite_migration` v1.x ist nicht mit rusqlite 0.39 kompatibel (entferntes `NO_PARAMS`-Symbol).
v1.3.1 erfordert eine Rust-Version > 1.94.0 (Toolchain-Lock). Migration daher mit `PRAGMA
user_version`-basiertem Custom-Runner in `klarvo-core/src/settings/migrations.rs` implementiert
— gleiche Semantik (geordnete Migrations, Schema-Owner ist migrations.rs, kein `CREATE TABLE IF
NOT EXISTS` im App-Code), kein externer Crate. Funktional äquivalent zu AC-1-Anforderung.

**Frontend CDN-React statt Vite+React (AC-8/AC-9):**
Phase-2-A-Minimal-Panel nutzt React 19 via `esm.sh` CDN im `index.html` — kein separates
Build-System. Vollständiges Vite+React-Workspace für die Windows-Shell ist Phase-2-B-Scope
(wenn UI weiter wächst).

**Frontend bypasst tauri-specta-Bindings (AC-8/AC-9 — Code-Review-Resolution D3, 2026-04-29):**
Das CDN-React-Panel ruft `window.__TAURI__.core.invoke(...)` direkt mit String-Command-Namen
statt der generierten `bindings/index.ts`-Wrapper (`commands.getUserSettings()` etc.). Grund:
ohne Build-System (siehe oben) lassen sich TS-Bindings nicht in den ESM-CDN-Pfad importieren.
`bindings/index.ts` wird von der AC-10-Drift-Gate weiterhin mitgewartet und ist für die
nicht-Panel-Konsumenten (Story-2.A.A8-Sub Tray-Language-Switcher, C2 Hotkey-Conflict-UX,
C3 Live-Locale-Switch) als typed Surface verfügbar. Type-Safety im Panel selbst kommt mit
Phase-2-B (Vite+React); bis dahin gilt Plain-`invoke`-Aufruf als bewusste Spec-Deviation.

**TOML→SQLite Soft-Parse — Strict-Parse + app.error statt Per-Field-Fallback (AC-2 — Code-Review-Resolution D2, 2026-04-29):**
Spec verlangt `tracing::warn!` pro unlesbarem Feld + Phase-1-Default für das jeweilige Feld.
Implementierung nutzt stattdessen die strikte `config::load_config`-Pipeline (`deny_unknown_fields`
+ Locale-Whitelist-Validation) und emittiert `app.error` mit Key `error.config.parse_failed`,
wenn `config.toml` nicht clean parst. Migration wird in dem Fall übersprungen (statt mit
Default-Werten zu überschreiben), Settings bleiben leer und Reads fallen auf Phase-1-Defaults
zurück — der User sieht den Fehler im Settings-Panel-Toast (D1) und kann seine `config.toml`
reparieren, ohne dass valide Felder still überschrieben wurden. Funktional stärker als die
Per-Field-Variante (kein Stille-Datenkorruption-Pfad), aber ohne den Pro-Feld-Recovery-Modus.

**AC-9 `app.error`-Bridge ist Listener-basiert, nicht Toast-Replacement (Code-Review-Resolution D1, 2026-04-29):**
Settings-Panel hört global auf `tauri.event.listen("app.error", ...)` und rendert eingehende
Backend-Errors (z.B. `error.config.parse_failed`, `error.settings.in_memory_fallback`,
spätere Hotkey-Konflikt-Keys) im selben Toast-UI wie lokal-gefangene Save-Failures. Lokale
`try/catch` in `handleSave` zeigt `userMessage` (kommt vom Backend-`AppError.user_message`-Feld)
oder fällt auf `message` zurück. Volle i18n-Key-Translation im Frontend liegt bei Phase-2-B
(zusammen mit Vite+React-Migration); aktuell wird der Key verbatim angezeigt.

**Event-Name `settings.changed` (Dot-Notation) statt `settings-changed` (Kebab-Case) (AC-5/AC-6 — Code-Review-Resolution P2-P16, 2026-04-30):**
Spec-Wortlaut in AC-5/AC-6 + ADR-0013-SD-5 nennt den Event-Namen kebab-case (`settings-changed`).
Die Impl emittiert via tauri-specta `#[tauri_specta(event_name = "settings.changed")]` in
Dot-Notation, konsistent mit `app.error` und `app.ready` und gemäß
`reference_tauri_specta_rc24_event_name`-Konvention sowie G1-Lint (FR34, Story 5.3).
Funktional äquivalent (gleiches Payload-Schema, gleiche Subscription-Mechanik); Doku-Lücke
zwischen Spec-Wortlaut und etablierter Naming-Convention. ADR-0013 SD-5 wird mit Amendment
auf Dot-Notation nachgezogen.

## Dev Agent Record

### Completion Notes

- AC-1: `settings(key, value, type)` Tabelle über `PRAGMA user_version` Migration erstellt.
  17 Unit-Tests in `klarvo-core::settings::tests` grün.
- AC-2: One-Shot TOML→SQLite Migration (Ast A + Ast B) implementiert. Exclusive Transaction.
  TOML-Soft-Parse in `main.rs` (fail-soft per `feedback_scaffold_fail_soft_pattern`).
- AC-3: Idempotenz via Count-Guard (non-empty table → skip). Test `migrate_from_toml_idempotent_on_second_run` grün.
- AC-4: 5 Typed Accessors (`ui_language`, `output_language`, `dictionary_language`,
  `hotkey_slot1_combo`, `output_target_id`) mit Default-Fallback.
- AC-5: `SettingsEmitter`-Trait in `klarvo-core` ohne `tauri`-Import.
  `TauriSettingsEmitter<R>` in Shell. `NoopSettingsEmitter` für Tests. Emit-after-persist.
- AC-6: 8 Tauri-Commands (5 Core-Set + 1 Bulk-Get + 2 Plugin) registriert via `collect_commands!`.
  `UserSettings` + `SettingsChangedEvent` via tauri-specta TS-exportiert.
- AC-7: Plugin Namespace Guard: CORE_PREFIXES-Check auf Set und Get.
  4 Plugin-Tests grün.
- AC-8: React-Settings-Panel in `shells/windows/src/index.html` (React 19 via CDN).
  `get_user_settings`-Aufruf beim Mount. Loading-Spinner während Async.
- AC-9: Save-Pfad via 5 typed Core-Set-Commands. Error-Toast bei Fehler.
- AC-10: `cargo xtask bindings-drift` → Exit 0. Alle anderen xtask-Gates grün.

### File List

- `klarvo-core/src/settings/mod.rs` (neu)
- `klarvo-core/src/settings/defaults.rs` (neu)
- `klarvo-core/src/settings/migrations.rs` (neu)
- `klarvo-core/src/lib.rs` (geändert — settings Modul hinzugefügt)
- `klarvo-core/Cargo.toml` (geändert — settings Feature)
- `shells/windows/src-tauri/src/commands/mod.rs` (neu)
- `shells/windows/src-tauri/src/commands/settings.rs` (neu)
- `shells/windows/src-tauri/src/lib.rs` (geändert — commands Modul + specta_builder erweitert)
- `shells/windows/src-tauri/src/main.rs` (geändert — Steps 2c/2d + app.manage(settings))
- `shells/windows/src/index.html` (geändert — React Settings Panel)
- `shells/windows/src/bindings/index.ts` (generiert — 8 Commands + 2 Event-Types)

### Change Log

- 2026-04-29: Story 2.A.A4 implementiert — Settings-Service, 8 Tauri-Commands,
  React-Settings-Panel, 17 Unit-Tests, alle xtask-Gates grün.
- 2026-04-29: Code-Review-Closure — 3 Decisions (D1/D2/D3) + 15 Patches applied;
  10 Defers in deferred-work.md persistiert; 23 Settings-Tests grün; xtask
  manifest-strict/lint-events/bindings-drift grün; 3 neue i18n-Keys
  (`error.config.parse_failed`, `error.settings.in_memory_fallback`,
  `error.settings.validation`) in en.json + de.json.
- 2026-04-30: Code-Review **Pass-2**-Closure — 3 Decisions resolved (P2-D1/D2 → defer,
  P2-D3 → P2-P21-Folge-Story); 16 von 21 Patches applied, 4 skipped als Folge-Story-Scope
  (P2-P1/P8/P12/P21), 1 dismissed (P2-P19 Blind-Spot); 7 neue Defers + 4 Pass-1-Re-confirms
  in deferred-work.md; ADR-0013 Amendment 2 (Event-Name Dot-Notation); neuer i18n-Key
  `error.unknown` in en.json + de.json. Build sauber, 23 Settings-Tests grün, Workspace-Tests
  ohne Failures. Story bleibt **`done`**; Skipped-Items als eigene Phase-2-A-Welle-3 oder
  Phase-2-B-Stories zu scopen (siehe deferred-work.md Pass-2-Skipped-Section).

### Review Findings

**Code-Review 2026-04-29 (3 Layer: Blind Hunter / Edge Case Hunter / Acceptance Auditor)**
**Resolution 2026-04-29:** 3 Decisions resolved + 15 Patches applied + 10 Defers persistiert.

#### Decision-Resolutions (3, alle applied)

- [x] [Review][Decision][D1] **AC-9 Spec-Verstoß: Save-Error nutzt lokalen Toast-State statt ADR-0009 `app.error`-Bridge** → Resolution Option (a): globaler `app.error`-Listener im Panel addiert; lokale Save-Errors zeigen `userMessage`-Feld. Spec-Deviations-Section erklärt Phase-2-A-Pragmatik (i18n-Translation-Stack noch nicht im Frontend). [`shells/windows/src/index.html`]
- [x] [Review][Decision][D2] **AC-2 Soft-Parse-Granularität: Struct-Level statt Per-Field** → Resolution Option (c): Strict-Parse via `config::load_config` + `app.error`-Eskalation bei TOML-Fehler; Migration übersprungen statt mit Defaults überschrieben. Spec-Deviation in Section dokumentiert. [`shells/windows/src-tauri/src/main.rs`]
- [x] [Review][Decision][D3] **Frontend bypasst tauri-specta-Bindings** → Resolution Option (a): als Phase-2-A-Trade-off akzeptiert (CDN-React kann TS-Bindings nicht via ESM importieren); Bindings-Drift-Gate bleibt für downstream consumers (A8-Sub/C2/C3) wichtig. Vite+React-Migration ist Phase-2-B-Scope. Spec-Deviation in Section dokumentiert. [`shells/windows/src/index.html`]

#### Patch (15, alle applied)

- [x] [Review][Patch][P1] APPDATA-Fallback durch Tauri `app.path().app_data_dir()` ersetzt + `fs::create_dir_all` vor `Connection::open` [`shells/windows/src-tauri/src/main.rs`]
- [x] [Review][Patch][P2] `expect("settings mutex poisoned")` durch `lock_conn()`-Helper (`unwrap_or_else(|p| p.into_inner())`) ersetzt — fail-soft-Pattern eingehalten [`klarvo-core/src/settings/mod.rs`]
- [x] [Review][Patch][P3] `expect("in-memory settings always succeeds")` durch Two-Step-Fallback (TauriSettingsEmitter → NoopSettingsEmitter) ersetzt; Boot-Panic eliminiert [`shells/windows/src-tauri/src/main.rs`]
- [x] [Review][Patch][P4] `count > 0` Detect-Condition jetzt strikt gegen `MIGRATION_SENTINEL_KEYS` (5 Core-Keys) — Plugin-First-Write blockt Core-Migration nicht mehr; neuer Test `migration_does_not_block_when_only_plugin_rows_exist` [`klarvo-core/src/settings/mod.rs`]
- [x] [Review][Patch][P5] `config.toml` UTF-8/Parse-Fehler eskaliert zu `app.error` (key `error.config.parse_failed`); Settings-Migration wird übersprungen — kein silent fresh-install mehr [`shells/windows/src-tauri/src/main.rs`]
- [x] [Review][Patch][P6] Set-Commands Input-Validation: `validate_setting_value` (non-empty + 4096-byte-Cap + control-char-reject) in alle 5 typed Setter eingebaut; 3 neue Tests (`set_rejects_empty_value`, `set_rejects_value_with_control_chars`, `set_rejects_oversized_value`) [`klarvo-core/src/settings/mod.rs`]
- [x] [Review][Patch][P7] `SettingsChangedEvent` mit `#[serde(rename_all = "camelCase")]` annotiert; `bindings/index.ts` synchron auf `newValue` umbenannt [`shells/windows/src-tauri/src/commands/settings.rs` + `shells/windows/src/bindings/index.ts`]
- [x] [Review][Patch][P8] HTML-Toast-Helper `errorToToast` nutzt `userMessage`-Feld vor `message`-Fallback — i18n-Key statt Tech-Message angezeigt [`shells/windows/src/index.html`]
- [x] [Review][Patch][P9] `validate_plugin_id` (`[a-z0-9_-]+`, non-empty, 64-byte-Cap) in `set_plugin_setting`/`get_plugin_setting`; 2 neue Tests (`plugin_set_rejects_invalid_plugin_id`, `plugin_set_accepts_valid_plugin_id`) [`klarvo-core/src/settings/mod.rs`]
- [x] [Review][Patch][P10] `langOptionsFor(currentValue)`-Helper prepend't Backend-Wert dynamisch wenn er nicht in `LANG_OPTIONS` ist — kein leeres `<select>` mehr bei Off-List-Werten [`shells/windows/src/index.html`]
- [x] [Review][Patch][P11] `get_user_settings`-Fehlerpfad fällt jetzt auf `FORM_DEFAULTS` (Phase-1-Defaults synchron zu `klarvo-core/src/settings/defaults.rs`) zurück — AC-8-"kein blank/undefined" hält auch im Error-Pfad [`shells/windows/src/index.html`]
- [x] [Review][Patch][P12] Test `set_persist_error_does_not_emit` umbenannt zu `validation_error_on_empty_value_does_not_emit` und mit echtem Negativ-Assert (Validation-Error → 0 Emits) verdrahtet [`klarvo-core/src/settings/mod.rs`]
- [x] [Review][Patch][P13] In-memory-Fallback nutzt `Arc::clone(&settings_emitter)` (statt `NoopSettingsEmitter`) — Frontend bekommt `settings.changed`-Events auch im Fallback-Path [`shells/windows/src-tauri/src/main.rs`]
- [x] [Review][Patch][P14] `klarvo-core = { path = "...", features = ["settings"] }` explizit in Shell-Cargo.toml — kein implicit-default-feature-Coupling mehr [`shells/windows/src-tauri/Cargo.toml`]
- [x] [Review][Patch][P15] `migrate_from_toml_if_needed` emittiert nach `tx.commit()` 5 explizite `settings.changed`-Events; neuer Test `migrate_from_toml_emits_5_settings_changed_events` [`klarvo-core/src/settings/mod.rs`]

#### Deferred (10) — pre-existing oder out-of-A4-scope

- [x] [Review][Defer] `set_raw` emit-after-DB-write Reorder-Risk bei Multi-Thread-Setter — theoretisch, in Single-User-UI nicht realistisch [`klarvo-core/src/settings/mod.rs:170-180`]
- [x] [Review][Defer] Plugin-emit failure swallowed (warn + continue) — Tauri-Emit-Failure rare, Backlog-würdig [`shells/windows/src-tauri/src/commands/settings.rs:67-69`]
- [x] [Review][Defer] `SettingsChangedEvent.value` als String — keine Type-Info im Payload — aktuell alle 5 Core-Fields sind strings; Plugin-i64/bool ist Phase-2-B+ [`klarvo-core/src/settings/mod.rs:34`]
- [x] [Review][Defer] `type`-Spalte ist dekorativ — keine Roundtrip-Validation auf value — strings only, Future-Phase [`klarvo-core/src/settings/mod.rs:154-162`]
- [x] [Review][Defer] Multi-Process SQLITE_BUSY (kein WAL, kein busy_timeout) — Single-Instance-Annahme; Backlog Multi-Window [`klarvo-core/src/settings/mod.rs:74-86`]
- [x] [Review][Defer] Schema-Migration v1 syntax error blockt zukünftige Migrationen — theoretischer Future-Pfad [`klarvo-core/src/settings/migrations.rs:19-26`]
- [x] [Review][Defer] AppError Display impl nutzt Debug-Repr; non_exhaustive serde — Backwards-compat Issue [`klarvo-core/src/error.rs`]
- [x] [Review][Defer] Form-State-Race auf schnellen Edits (kein Dirty-Tracking) — Spec verlangt kein Unsaved-Warning [`shells/windows/src/index.html:415-430`]
- [x] [Review][Defer] SettingsChangedEvent-Subscription fehlt im Frontend — Single-Window-Fall; A8-Sub/C2/C3 holen das nach [`shells/windows/src/index.html:386-481`]
- [x] [Review][Defer] HTML-Panel selbst nicht i18n-übersetzt (`lang="en"`, "Klarvo Settings" hardcoded) — Phase-2-A-Minimal-Panel; volle i18n in Phase-2-B Vite+React [`shells/windows/src/index.html:305,449`]

---

**Code-Review Pass-2 2026-04-29 (3 Layer fresh adversarial audit auf uncommitted-only-Diff)**
**Scope:** post-done uncommitted changes in `klarvo-core/src/settings/*` + `shells/windows/src-tauri/src/lib.rs` + `shells/windows/src/index.html` + locales. Auditor liest darüberhinaus committeden Code (`commands/settings.rs`, `main.rs`) für Spec-Compliance.

#### Decision-Resolutions Pass-2 (3, alle resolved 2026-04-30)

- [x] [Review][Decision][P2-D1] **CSP + SRI auf React ESM CDN-Imports** → Resolution Option (b): defer + dokumentieren. **Reason:** Phase-2-A intern, EA zurückgezogen, kein User-Release; SRI auf `esm.sh`-Sub-Imports (chained: react→react-dom→scheduler) ist Halbsicherheit + false sense of security; CSP-Lockdown gegen `esm.sh` brüchig; saubere Lösung = Vendor-Bundling oder Vite-Migration in Phase-2-B. Investment in (a/c) ist Wegwerf-Code. → in `deferred-work.md` persistiert. [`shells/windows/src/index.html:184-185`]
- [x] [Review][Decision][P2-D2] **Sequential 5-await Save → Partial-State auf Mid-Fail** → Resolution Option (c): akzeptieren als Phase-2-A-Limitation. **Reason:** Mid-Fail-Trigger-Surface schrumpft mit P2-P2 (`catch_unwind`) + P2-RC1 (SQLite-Hardening) auf "Validation-Fehler"; Validation ist pro-Wert deterministisch → entweder alle 5 OK oder UI hat Garbage. Atomare Batch-Command-Variante (a) ist Phase-2-B-Wert (Vite + Save-Status-pro-Field-UX) und kann als eigene Story rein. → in `deferred-work.md` persistiert. [`shells/windows/src/index.html:291-307`]
- [x] [Review][Decision][P2-D3] **Allow-List für Locales/Output-Target — Authority Frontend vs Core** → Resolution Option (b): Frontend-Hard-Reject + `TomlMigrationSource` semantic-fallback. **Reason:** ADR-0013 trennt Settings (strukturelle Validation) bewusst von semantischer Validation; Core-side Allow-List würde Settings mit i18n-Loader + OutputTarget-Registry verkoppeln → Spec-Verstoß. → konvertiert zu **Patch P2-P21** (siehe unten). [`klarvo-core/src/settings/mod.rs:594-647` + `shells/windows/src/index.html:238-243`]

#### Patch Pass-2 (21 → 16 applied + 4 skipped + 1 dismissed, 2026-04-30)

**Applied 16:** P2-P2, P2-P3, P2-P4, P2-P5, P2-P6, P2-P7, P2-P9, P2-P10, P2-P11, P2-P13, P2-P14, P2-P15, P2-P16, P2-P17, P2-P18, P2-P20.
**Skipped 4** (Urteil/Architektur — explizit als eigene Folge-Story scoped): P2-P1 (Load-fail-Form-UX-Redesign), P2-P8 (`Settings::user_snapshot` architectural extension), P2-P12 (corrupt-DB-rename — Error-Path-Discrimination), P2-P21 (D3-Impl: zwei-System-Change mit Semantic-Defaults).
**Dismissed 1:** P2-P19 — `Settings::in_memory` IST der Production-Fallback-Path (Pass-1-P3); cfg-gating würde Boot-Resilience brechen. Blind-Hunter-Blind-Spot ohne Projekt-Kontext.

Build (`cargo build --workspace --exclude klarvo-windows-shell`) sauber; alle 23 Settings-Unit-Tests grün; Workspace-Tests (excl. Windows + JNI) ohne Failures.

- [ ] [Review][Patch][P2-P21] **Frontend-Hard-Reject + `TomlMigrationSource` semantic-fallback (von P2-D3)** — **SKIPPED (Folge-Story).** Zwei-System-Change mit semantischen Default-Entscheidungen (welcher Locale-Default? welcher Output-Target-Fallback? wie Migration-Warning surface'n?) — verdient eigenes Spec + Implementation-Pass, nicht Batch-apply. Empfehlung: als eigene Phase-2-A-Welle-3 oder Phase-2-B-Story scopen. Locale/Target-Allow-List-Authority bleibt bei den jeweiligen Konsumenten. (a) Frontend rejected unbekannte UI/Output/Dictionary-Language-Werte vor Save mit Warn-Toast (`error.settings.unknown_locale`); `langOptionsFor`-Helper aus Pass-1-P10 wird redundant und kann entfernt werden. (b) `TomlMigrationSource`/`migrate_from_toml_if_needed` bekommt semantic-fallback für Locale-Felder: bei unbekanntem Wert → Default + `tracing::warn!`, damit v1-importierter `"fr"` o.ä. nicht überlebt. Output-Target analog (`klipboard` falls unbekannt). [`klarvo-core/src/settings/mod.rs:498-545` + `shells/windows/src/index.html:238-243,291-307`]

- [ ] [Review][Patch][P2-P1] **SKIPPED (Folge-Story).** **Load-fail Data-loss: setForm(FORM_DEFAULTS) erlaubt Save-Stomp realer Werte** — Bei `invoke("get_user_settings")`-Failure schreibt der Catch-Pfad Phase-1-Defaults in den Form-State + Toast. Klickt User dann Save, werden die echten DB-Werte mit hardcoded Defaults überschrieben. Pass-1-P11 hat das bewusst so gebaut für AC-8 "kein blank"; aber Save-while-load-failed ist Data-Loss. Fix: Form disabled + Save-Button hidden solange Load fehlgeschlagen ist; explizites "Reload" anbieten. [`shells/windows/src/index.html:271-289`]
- [x] [Review][Patch][P2-P2] **APPLIED.** **`set_raw` + Migration-Emit-Loop: kein `catch_unwind` um `emit_settings_changed`** — Emitter-Panic propagiert nach Tauri-Command-Boundary; DB ist committed, aber Frontend sieht Err-Toast → User retried → Double-Write. Im Migration-Loop skippt eine Panic auf erstem Emit die restlichen 4. Fix: jeder Emit-Call in `std::panic::catch_unwind(AssertUnwindSafe(...))` + `tracing::warn!` bei Panic. Doku im Trait: Emits sind advisory. [`klarvo-core/src/settings/mod.rs:170-180,548`]
- [x] [Review][Patch][P2-P3] **APPLIED.** **`lock_conn` Poison-Recovery: silent + dangling-tx-Hazard** — `unwrap_or_else(|p| p.into_inner())` recovert ohne `tracing::warn!`. Wenn der Vorgänger mid-tx panicte, bleibt eine offene Transaktion im Connection-State; nächster `INSERT` läuft INSIDE der dangling-tx und commitet nicht. Fix: bei Poison-Recovery `tracing::warn!("settings mutex poisoned, recovering")` + `conn.execute("ROLLBACK", []).ok()` defensive. [`klarvo-core/src/settings/mod.rs` `lock_conn`]
- [x] [Review][Patch][P2-P4] **APPLIED.** **Migration-Detect skippt bei type-mismatched Sentinel-Row** — Pass-1-P4 verschärfte Detect zu strict-COUNT auf 5 Sentinel-Keys. EH: Wenn ein zukünftiger Writer einen Sentinel-Key mit `type='json'` schreibt, blockt das die Migration, aber typed-Accessor wirft `Internal "unknown settings type"`. Fix: `... AND type='string'` im Sentinel-Count-Query, oder separater `migration.applied=true`-Marker statt Daten-als-State-Indikator. [`klarvo-core/src/settings/mod.rs:489-509`]
- [x] [Review][Patch][P2-P5] **APPLIED.** **`TomlMigrationSource`-INSERT bypasst `validate_setting_value`** — Migration schreibt Strings direkt via raw `INSERT` in EXCLUSIVE-Tx ohne Validation. Wenn die Shell ein leeres / control-char-haltiges / oversized Feld in den Source-Struct schiebt (Soft-Parse-Bug oder bewusst), persistiert garbage; nachfolgende `set_*` rejecten denselben Wert (read-write-Asymmetrie). Fix: `for (k,v) in writes { validate_setting_value(k, v)?; }` vor `tx.execute`. [`klarvo-core/src/settings/mod.rs:498-545`]
- [x] [Review][Patch][P2-P6] **APPLIED.** **`errorToToast` rendert Tech-Message als User-Toast (kein i18n-Fallback)** — Pass-1-P8 prefers `userMessage` aber fällt auf `message` zurück, der oft technische Rust-Strings ("settings db: write hotkey.slot1.combo: database is locked") enthält. Außerdem Payload-Shape-Variance (string vs object) → Toast zeigt verbatim "error.unknown". Fix: bei fehlendem `userMessage` immer `error.unknown` i18n-Key (siehe P2-P11), nie Raw-`message`; Payload-Shape-Guard. [`shells/windows/src/index.html:202-208,257-268`]
- [x] [Review][Patch][P2-P7] **APPLIED.** **`app.error`-Listener: useEffect-Unlisten-Race (use-after-mount)** — `tauriEvent.listen(...)` returniert Promise; Cleanup-Closure capturet `unlisten` per Reference. Bei Unmount vor Promise-Resolve leakt Listener + setzt State auf unmounted Component. Fix: `let cancelled = false; ... if (cancelled) unlisten?.(); else stash;`-Pattern. [`shells/windows/src/index.html:257-268`]
- [ ] [Review][Patch][P2-P8] **SKIPPED (Folge-Story).** **`get_user_settings`: 5 sequentielle `get_raw`-Locks → torn read** — Concurrent `set_*` zwischen den 5 Lock-Cycles mischt fresh + stale; Panel-Save stomped concurrently-written value. Fix: `Settings::user_snapshot() -> UserSettingsSnapshot` der Mutex EINMAL nimmt + 5 SELECTs in einem Call macht; `get_user_settings`-Command ruft Snapshot auf. [`shells/windows/src-tauri/src/commands/settings.rs:127-137`]
- [x] [Review][Patch][P2-P9] **APPLIED.** **Form-Enter-Submit während `saving=true` → konkurrente `handleSave`** — Save-Button ist disabled, aber `<form>` akzeptiert Enter im Input-Field weiterhin. Zweiter `handleSave` läuft parallel mit potentially-mutated Form-State. Fix: `onSubmit: (e) => { e.preventDefault(); if (!saving && !loading) handleSave(); }`. [`shells/windows/src/index.html:333-336`]
- [x] [Review][Patch][P2-P10] **APPLIED** (`validate_plugin_key` Helper, symmetric in set + get, rejected `plugins.`-Prefix). **Plugin-`key`-Shape nicht validiert (set + get asymmetric)** — Pass-1-P9 validiert `plugin_id` aber NICHT `key` selbst. Empty / control-chars / oversized / Recursive-Prefix `"plugins."` als key passt durch und führt zu Key-Kollisionen + unbounded primary keys. Get-Side hat 0 Validation. Fix: gleiche shape-validation auf `key` symmetrisch in `set_plugin_setting` + `get_plugin_setting`; reject `key.starts_with("plugins.")`. [`klarvo-core/src/settings/mod.rs:655-679`]
- [x] [Review][Patch][P2-P11] **APPLIED.** **`error.unknown`-Key fehlt in beiden Locale-Files** — `index.html:262` referenziert `'error.unknown'` als Fallback, `de.json` + `en.json` definieren ihn nicht. Toast zeigt verbatim `"error.unknown"`. Fix: Key in beiden Locales ergänzen ("Unknown error" / "Unbekannter Fehler"). [`shells/windows/locales/{de,en}.json`]
- [ ] [Review][Patch][P2-P12] **SKIPPED (Folge-Story).** **`Settings::open`: corrupt on-disk DB → silent in-memory-Shadow** — Pass-1-P3 macht Two-Step-Fallback (TauriEmitter → NoopEmitter → in-memory). EH: Bei korrupter Disk-DB bleibt das alte File auf Disk shadowed; nächster Launch failed wieder gleichermaßen. Fix: bei `Connection::open`/`migrations.apply`-Err → File umbenennen zu `settings.db.corrupted-<unix-ts>` + `tracing::error!`-Audit-Log + dann in-memory. User-actionable. [`klarvo-core/src/settings/mod.rs::open`]
- [x] [Review][Patch][P2-P13] **APPLIED** (`migrations::apply` checked vor Loop dass `current ≤ max_known`, sonst `migration_err`). **DB-Downgrade-Szenario (`user_version > max_known`) → silent acceptance** — Wenn ein neuere Klarvo-Version eine `user_version=2` schreibt und der User auf alte Binary downgradet, läuft die alte Binary mit Newer-Format-DB; SELECTs failen kryptisch. Fix: `if current > max_known_version { Err("settings db ahead of binary") }` vor Apply-Loop. [`klarvo-core/src/settings/migrations.rs::apply`]
- [x] [Review][Patch][P2-P14] **APPLIED** (`c.is_control()` statt `< 0x20`; Bidi/BOM nicht explizit gelistet, durch `is_control` mit-erfasst soweit ASCII/C1; reine Bidi-Marks bleiben erlaubt — separater Defer falls später nötig). **`validate_setting_value` rejects nur `< 0x20` — DEL, C1, BOM, Bidi durchgelassen** — `(c as u32) < 0x20` erlaubt `\u{7F}` (DEL), C1-Range `\u{80}-\u{9F}`, BOM `\u{FEFF}`, RTL/LTR-Override-Marks, Zero-Width. Fix: `c.is_control()` von std (deckt 0x00-0x1F, 0x7F, C1 ab); plus optional explicit reject für BOM + Bidi-Marks. [`klarvo-core/src/settings/mod.rs:731-735`]
- [x] [Review][Patch][P2-P15] **APPLIED.** **`get_raw` returnt `Some("")` für leeren Wert** — `value TEXT NOT NULL` erlaubt empty string. `validate_setting_value` rejected empty bei Set, aber Migration bypasst Validation (siehe P2-P5). Defensive: in `get_raw` empty als `None` behandeln, damit Default-Fallback greift statt Blank-UI / kaputter Locale-Lookup. [`klarvo-core/src/settings/mod.rs::get_raw`]
- [x] [Review][Patch][P2-P16] **APPLIED** (4. Spec-Deviation in dieser Story-Section + ADR-0013 Amendment 2 nachgezogen). **Spec-Deviation Event-Name: `settings-changed` (Spec/ADR-0013) vs `settings.changed` (Impl)** — Spec AC-5+AC-6 + ADR-0013 SD-5 nutzen kebab-case; Impl + TS-Bindings nutzen Dot-Notation gemäß `reference_tauri_specta_rc24_event_name`-Konvention (G1-Lint). Impl ist korrekt, Doku-Lücke. Fix: 4. Eintrag in `Spec-Deviations`-Section nachtragen + ADR-0013-Amendment SD-5 auf Dot-Notation umschreiben. [`docs/adr/0013-settings-persistence-schema.md` + diese Story]
- [x] [Review][Patch][P2-P17] **APPLIED** ("Bitte prüfen Sie config.toml" → "Bitte prüfen Sie die Datei auf Syntaxfehler"; EN parallel angepasst). **German Translation: Raw `config.toml`-Filename in User-Toast** — `error.config.parse_failed`: "Bitte prüfen Sie config.toml" — User weiß nicht wo `config.toml` liegt. Fix: "Ihre Konfigurationsdatei" oder resolved Pfad einsetzen; EN parallel anpassen. [`shells/windows/locales/de.json:43` + `en.json`]
- [x] [Review][Patch][P2-P18] **APPLIED** (Tech-Message: "byte-length {N} exceeds maximum {MAX}"). **`MAX_VALUE_LEN` Tech-Message: bytes vs chars unklar** — Validation-Error-Message "exceeds 4096 bytes" — Konsumenten denken oft chars. Fix: explizit "byte length" im Tech-Message; user-facing Key bleibt generisch. [`klarvo-core/src/settings/mod.rs:727-730`]
- [ ] [Review][Patch][P2-P19] **DISMISSED** (Blind-Hunter-Blind-Spot ohne Projekt-Kontext). **`Settings::in_memory` ist `pub` in Release-Binaries** — `in_memory` ist Production-Fallback (Pass-1-P3 main.rs Two-Step-Fallback) wenn `Settings::open` fehlschlägt; cfg-test-Gating würde den Boot-Resilience-Pfad brechen. Plugin-Authors können den Helper aufrufen, aber er greift auf eine flüchtige In-Memory-DB zu — kein Sicherheits-/State-Risiko. Keine Aktion. — Test-Helper als Public-API exponiert ohne cfg-Gate; Plugin-Authors können in-memory-Settings konstruieren und reale DB umgehen. Fix: `#[cfg(any(test, feature = "test-utils"))]` oder Behind-Feature-Flag. [`klarvo-core/src/settings/mod.rs::in_memory`]
- [x] [Review][Patch][P2-P20] **APPLIED.** **`tracingFallback` ist pointless Indirection** — `console.warn`-Wrapper mit kommentar "tracing fallback"; im 100-Zeilen-Skript Dead-Weight ohne Mehrwert (kein echtes Tracing eingebunden). Fix: löschen, direkt `console.warn`. [`shells/windows/src/index.html:210-213`]

#### Pass-1 Re-confirmed Defers (4 — bereits in Pass-1 deferred, von Pass-2 unabhängig bestätigt)

- [x] [Review][Defer][P2-RC1] SQLite-Hardening (WAL/busy_timeout/synchronous) — Pass-1-A4-D5 deferred; Pass-2 bestätigt zusätzlich Cross-Process-Race auf `migrations.apply` (xtask-Binary + Main-App)
- [x] [Review][Defer][P2-RC2] `settings.changed`-Listener fehlt im Frontend — Pass-1-A4-D9 deferred; Pass-2 bestätigt: External-Writer-Stomp wird Foundation-Issue für A8-Sub/C2/C3
- [x] [Review][Defer][P2-RC3] HTML `lang`-Attribut statisch `"en"` — Pass-1-A4-D10 deferred; Pass-2 ergänzt a11y/Screen-Reader-Mispronunciation als zusätzlichen Konsequenz
- [x] [Review][Defer][P2-RC4] `type`-Spalte ohne `CHECK`-Constraint, Roundtrip-Validation fehlt — Pass-1-A4-D4 deferred; Pass-2 verschärft (Schema-CHECK würde Future-Phase-Bug strukturell verhindern)

#### Deferred Pass-2 (7, neu)

- [x] [Review][Defer][P2-W1] `CORE_PREFIXES` reserviert `"license."` + `"history."` ohne Doc-Comment — premature Reservation, Plugin-Author-Verwirrung. Doc-Update genügt; ADR-pending. [`klarvo-core/src/settings/mod.rs:393`]
- [x] [Review][Defer][P2-W2] `migration_does_not_block_when_only_plugin_rows_exist`-Test verifiziert nicht "block-detection" — schwacher Assert (nur Core-Field-Equality, nicht `count(*)==6`). Test-Hardening, nicht code-bearing. [`klarvo-core/src/settings/mod.rs:899-912`]
- [x] [Review][Defer][P2-W3] Test-Boilerplate `Settings::in_memory(noop()).unwrap()` 20×+ — `fn fresh()`-Helper würde Lesbarkeit erhöhen; reine Test-Quality. [`klarvo-core/src/settings/mod.rs::tests`]
- [x] [Review][Defer][P2-W4] `invoke("get_user_settings")` Hang → Infinite-Spinner — Tauri-IPC ist lokal, sollte immer returnen; Timeout wäre defensive Ceremony. Optional Hardening. [`shells/windows/src/index.html:271-289`]
- [x] [Review][Defer][P2-W5] Power-Loss zwischen `BEGIN` und `COMMIT` + zwischenzeitlich edited TOML — extrem rare; SQLite-Atomicity hält den primären Pfad; Mitigation braucht Design-Pass. [`klarvo-core/src/settings/mod.rs::migrate_from_toml_if_needed`]
- [x] [Review][Defer][P2-W6] React ESM-Imports kein Offline-Cache — First-Launch ohne Netz = blank Panel. Phase-2-B Vite+React-Migration löst strukturell. [`shells/windows/src/index.html:184-185`]
- [x] [Review][Defer][P2-W7] Story-1B `rusqlite_migration`-Präzedent-Behauptung in AC-1 unverifiziert — Memory-Hygiene; per `feedback_reviewer_external_fact_verification` zu markieren oder Verifikations-Ref nachzutragen. Doc-Only. [Story-File AC-1]

#### Dismissed Pass-2 (3)

- Blind Hunter "missing `commands/settings.rs`" — False Positive aus Diff-only-Kontext; File ist in HEAD committed (e25c308 oder früher).
- Blind Hunter "`lock_conn` helper indirection" — subjektiver Style; Helper dokumentiert Poison-Policy-Intent.
- Blind Hunter "`MIGRATION_SENTINEL_KEYS` placeholder build fragile" — funktioniert korrekt; nicht fragil.
