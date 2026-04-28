---
name: Story 2.A.A4 — Settings-Panel Foundation
phase: 2
wave: A
story_id: "2.A.A4"
status: Draft
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

_Leer — wird bei Implementation-Surprises befüllt (per `feedback_kickoff_deltas_only`
Amendment 2026-04-26)._
