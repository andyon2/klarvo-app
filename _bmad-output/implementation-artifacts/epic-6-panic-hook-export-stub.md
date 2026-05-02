---
name: Story 6.3 — Panic-Hook + telemetry::export-Stub
epic: 6
story_number: "6.3"
status: done
dependencies:
  - "6-1-telemetry-logging-rolling-file"
---

# Story 6.3: Panic-Hook + `telemetry::export`-Stub

Status: done

## Story

Als Core-Dev / Support-Engineer
möchte ich `std::panic::set_hook` so installieren, dass Uncaught Panics als `level=ERROR` tracing-Events in den Rolling-File-Log landen, UND ein `klarvo-core::telemetry::export`-Modul-Stub anlegen, der für die Phase-2-UI-getriggerte Debug-Export-Zip-Funktion vorbereitet ist,
damit Crashes im Field debuggbar sind (FR39) und das Export-Surface mechanisch existiert für die spätere Settings-UI-Anbindung in Epic 9 (FR40).

## Kontext und Motivation

**FR39 — Panic-Capture:** Ohne Panic-Hook landen Uncaught Panics auf Rust-Default-Stderr (im Release-Build oft unsichtbar, da `windows_subsystem = "windows"` aktiv ist und kein Konsolen-Window existiert). Das Rolling-File-Log aus Story 6.1 wird so für Crashes wertlos. Lösung: `std::panic::set_hook(...)` direkt nach `init_tracing` installieren — der Hook formatiert die `PanicInfo` und ruft `tracing::error!(...)`, was den Subscriber aus 6.1 erreicht und in das Rolling-File geschrieben wird.

**FR40 — Export-Stub als Foundation:** Epic 9 wird einen Settings-UI-Button "Debug-Export-Zip" hinzufügen, der eine Zip-Datei mit Logs + redacted Config + Sys-Info erstellt. Die Implementation der Zip-Logik ist Phase 2. Story 6.3 legt das Modul-Skelett `klarvo-core::telemetry::export` an mit einer fail-soft-Stub-Funktion, sodass Epic 9 nur den UI-Trigger und die echte Implementation hinzufügen muss, ohne das Modul-Layout neu zu erfinden.

**Fail-Soft-Pattern:** `memory/feedback_scaffold_fail_soft_pattern.md` schreibt vor: kein `todo!()`, kein `unimplemented!()`, kein Panic. Stub-Funktionen returnen einen strukturierten `AppError` mit einem i18n-Key. Story 6.3 folgt diesem Pattern.

**Abhängigkeit zu Story 6.1:** Story 6.3 baut auf Story 6.1 auf — der Panic-Hook nutzt den Subscriber aus 6.1, das `telemetry`-Modul existiert erst nach 6.1. Story 6.3 sollte NICHT vor Story 6.1 implementiert werden.

## Acceptance Criteria

### AC-1: `panic_hook`-Funktion in `klarvo-core::telemetry::logging`

**Given** `klarvo-core/src/telemetry/logging.rs` existiert (aus Story 6.1),
**When** Story 6.3 committed ist,
**Then** existiert eine neue öffentliche Funktion:
```rust
pub fn install_panic_hook()
```
- Ruft `std::panic::set_hook(Box::new(...))`
- Der Hook-Closure empfängt `&PanicInfo` und emittiert ein `tracing::error!`-Event mit folgenden Feldern:
  - `panic.message`: `payload.downcast_ref::<&str>()` oder `downcast_ref::<String>()` Versuche; Fallback `"<non-string panic payload>"`
  - `panic.location.file`: `info.location().map(|l| l.file()).unwrap_or("<unknown>")`
  - `panic.location.line`: `info.location().map(|l| l.line()).unwrap_or(0)`
  - `panic.location.column`: analog
  - Backtrace: `std::backtrace::Backtrace::force_capture()` als Display-String im `panic.backtrace`-Feld
- Der bisherige Default-Hook wird **nicht** zusätzlich aufgerufen — sonst landet die Panic-Message doppelt (einmal im Stderr, einmal im File). Im Release-Build ist Stderr ohnehin nicht sichtbar.

### AC-2: Panic-Hook wird in Windows-Shell installiert

**Given** `shells/windows/src-tauri/src/main.rs` ruft `init_tracing` (aus Story 6.1 AC-5),
**When** Story 6.3 committed ist,
**Then** wird **direkt nach** `init_tracing` und **vor** `tauri::Builder::default()` `install_panic_hook` aufgerufen:
```rust
let _tracing_guard = klarvo_core::telemetry::logging::init_tracing(&log_dir);
klarvo_core::telemetry::logging::install_panic_hook();
```
Reihenfolge ist kritisch: der Hook muss NACH dem Subscriber-Setup installiert werden, sonst landet eine Panic vor dem Subscriber-Setup im Void.

### AC-3: i18n-Key `error.telemetry.export.unimplemented` in REQUIRED_KEYS

**Given** `klarvo-core/src/i18n.rs::REQUIRED_KEYS` Liste,
**When** Story 6.3 committed ist,
**Then** ist `error.telemetry.export.unimplemented` in `REQUIRED_KEYS` aufgenommen UND in beiden Locale-Files (`shells/windows/src/locales/de.json`, `shells/windows/src/locales/en.json`) übersetzt:
- DE: `"Debug-Export ist in dieser Version nicht verfügbar."`
- EN: `"Debug export is not available in this version."`

`cargo xtask required-keys-drift` (aus Story 5.6) bleibt grün.

### AC-4: `telemetry::export`-Modul mit fail-soft-Stub

**Given** `klarvo-core/src/telemetry/mod.rs` (aus Story 6.1) enthält nur `pub mod logging;`,
**When** Story 6.3 committed ist,
**Then**:
- `klarvo-core/src/telemetry/mod.rs` enthält zusätzlich `pub mod export;`
- Neue Datei `klarvo-core/src/telemetry/export.rs` mit:
  ```rust
  //! Debug-Export-Zip-Stub. Phase-2-Surface für Settings-UI (Epic 9).
  //!
  //! Spec: architecture.md §9 Observability + prd.md FR40.
  //!
  //! NFR5: ein zukünftiger Real-Impl darf KEINE Audio/Text-Daten exportieren.
  //! Nur: Rolling-File-Log + redacted Config + Sys-Info.

  use std::path::Path;
  use crate::error::{AppError, AppErrorKind};

  /// Erzeugt ein Debug-Export-Zip am gegebenen Pfad.
  ///
  /// Phase-1-Stub: returnt fail-soft `AppError` mit i18n-Key
  /// `error.telemetry.export.unimplemented`. Real-Impl folgt in Epic 9
  /// (Settings-UI-Trigger).
  pub fn export_debug_zip(_out_path: &Path) -> Result<(), AppError> {
      Err(AppError {
          kind: AppErrorKind::Configuration, // bis Phase-2 keine eigene Variante
          message: "telemetry::export::export_debug_zip is a Phase-1 stub".into(),
          user_message: Some("error.telemetry.export.unimplemented".into()),
          retryable: false,
      })
  }
  ```
- Hinweis zu `AppErrorKind::Configuration`: bis ein eigener `AppErrorKind::Unimplemented`-Variant existiert (kein Phase-1-Konsument), nutzt der Stub `Configuration` als nächstgelegene semantische Annäherung. Die i18n-Key ist die User-facing Information; `kind` ist programm-intern.

### AC-5: Headless-Test für Export-Stub

**Given** `export.rs` ist implementiert,
**Then** existiert in `klarvo-core` ein Headless-Test (`#[cfg(test)] mod tests` in `export.rs`):
```rust
#[test]
fn export_debug_zip_returns_unimplemented_error() {
    let result = export_debug_zip(std::path::Path::new("/tmp/dummy.zip"));
    let err = result.expect_err("stub must return Err");
    assert_eq!(err.user_message.as_deref(), Some("error.telemetry.export.unimplemented"));
    assert!(!err.retryable);
}
```

Hinweis: `klarvo-core` hat den `disallowed_methods`-Lint aus Story 5.5 — `.expect_err(...)` ist im Test-Modul erlaubt (`#[allow(clippy::disallowed_methods)]` für `tests`-Module ist Story-5.5-Pattern; falls bereits Test-Allow-Wide aktiv ist, kein extra Attribut nötig).

### AC-6: Headless-Test für Panic-Hook-Format

**Given** `install_panic_hook` ist implementiert,
**Then** existiert ein Headless-Test der die String-Formatting-Helper (Payload-Extraktion) testet — NICHT die Hook-Installation selbst (das ist global state, nicht testbar in Multi-Test-Binary).

**Empfohlener Refactor zur Testbarkeit:**
```rust
fn format_panic_payload(info: &std::panic::PanicHookInfo) -> (String, &'static str, u32, u32) {
    // (message, file, line, column)
    ...
}
```
Der Test konstruiert eine `PanicHookInfo` (oder testet `format_panic_payload` mit synthetischen Inputs), prüft dass die Strings korrekt extrahiert werden.

Falls `PanicHookInfo` nicht direkt konstruierbar ist, alternative: Helper-Funktionen für `&str`/`String`-Payload-Extraktion separat testen.

### AC-7: Crash-Smoke-Test (manuell, dokumentiert)

**Given** Story 6.3 ist committed,
**When** Andy einen synthetischen Panic auslöst (z. B. via temporärem `panic!("smoke test")` in `main.rs` direkt vor `tauri::Builder::default()`),
**Then** sieht er den Panic im Rolling-File-Log unter `%APPDATA%\klarvo\logs\klarvo.YYYY-MM-DD` mit `level=ERROR`-Eintrag inklusive Backtrace.

Das ist ein **manueller Smoke-Test** im Story-Closure, nicht ein automatisierter Test. Dokumentiert in der Completion Notes List der Story (mit Datum + Beobachtung).

## Tasks / Subtasks

- [x] Panic-Hook implementieren (AC-1)
  - [x] `install_panic_hook` in `logging.rs`
  - [x] Payload-Extraktions-Helper (Refactor für AC-6 Testbarkeit)
- [x] Windows-Shell-Bootstrapper anpassen (AC-2)
  - [x] `main.rs`: `install_panic_hook()` direkt nach `init_tracing`
- [x] i18n-Keys (AC-3)
  - [x] `error.telemetry.export.unimplemented` in den Backend-Locale-Files (`shells/windows/locales/{en,de}.json`); `REQUIRED_KEYS`-Const wurde durch Story 5.6 obsolet, Äquivalenz via `cargo xtask lint-events` G3-D orphan-check
  - [x] Übersetzungen in `de.json` + `en.json`
  - [x] `cargo xtask lint-events` grün
- [x] Export-Stub (AC-4)
  - [x] `klarvo-core/src/telemetry/export.rs` anlegen
  - [x] `pub mod export;` in `mod.rs`
- [x] Tests (AC-5 + AC-6)
- [ ] Manueller Smoke-Test (AC-7) — **Carry-Over Windows-Env:** in WSL/Linux-Build-Env nicht ausführbar; Verfahren in Completion Notes List dokumentiert, Ausführung sobald Windows-Build-Box verfügbar

## Dev Notes

### `PanicHookInfo` vs `PanicInfo`

In Rust 1.81+ wurde der Typ in `std::panic::PanicHookInfo` umbenannt; `PanicInfo` ist deprecated für Hook-Use-Cases. Der Dev-Agent sollte `PanicHookInfo` verwenden. Beide Typen haben dieselbe API (`location()`, `payload()`).

### Backtrace-Capture

`std::backtrace::Backtrace::force_capture()` ist `1.65+`-stabil. Im Release-Build sind Backtraces mit Symbolen nur verfügbar wenn `RUST_BACKTRACE=1` (Default Off) oder Cargo-Profil debug-info aktiv ist. Für Phase-1-Dogfooding: `RUST_BACKTRACE=1` bevor App-Start setzen, oder im `tauri.conf.json` als Env-Var; alternativ Cargo-Profil `release.debug = "line-tables-only"` aktivieren (kostet ~1-2MB Binary-Size, gibt brauchbare Traces).

### Default-Hook NICHT zusätzlich aufrufen

Der idiomatische Pattern wäre, den Default-Hook als `let prev_hook = std::panic::take_hook();` zu speichern und im neuen Hook am Ende `prev_hook(info)` zu rufen. Story 6.3 macht das **nicht**, weil:
- Im Release-Build (`windows_subsystem = "windows"`) ist Stderr nicht sichtbar — der Default-Hook macht nichts Wertvolles
- Doppel-Logging (Stderr + File) ist im Debug-Build verwirrend
- Falls in Phase 2 ein Crash-Reporter (z. B. minidump) hinzukommt, kann der Hook erweitert werden

### `disallowed_methods`-Lint im Hook-Closure

Der Hook-Closure läuft im Production-Code (klarvo-core, vom Lint betroffen). `.unwrap()` und `.expect()` sind verboten. Pattern für Payload-Extraktion:
```rust
let message = info
    .payload()
    .downcast_ref::<&'static str>()
    .copied()
    .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
    .unwrap_or("<non-string panic payload>");
```
`unwrap_or` mit konkretem Default ist erlaubt (kein `unwrap()`).

### `error.telemetry.export.unimplemented` — i18n-Key-Schema

Folgt dem etablierten Schema aus `architecture.md §4 i18n-Keys`: `error.<feature>.<element>.<purpose>`. `feature = telemetry`, `element = export`, `purpose = unimplemented`. Konsistent mit anderen `error.*.unimplemented`-Keys (Phase-2-Pattern).

### REQUIRED_KEYS-Drift-Gate (Story 5.6)

`cargo xtask required-keys-drift` wird von Story 5.6 gestellt. Bei Hinzufügen eines neuen Keys MUSS dieser in beiden Locale-Files vorhanden sein, sonst ist der Gate rot. Reihenfolge der Edits: REQUIRED_KEYS → de.json → en.json → xtask-Run zur Verifikation.

### Project Structure Notes

Neue Dateien:
- `klarvo-core/src/telemetry/export.rs`

Geänderte Dateien:
- `klarvo-core/src/telemetry/mod.rs` (1 Zeile: `pub mod export;`)
- `klarvo-core/src/telemetry/logging.rs` (~30-50 Zeilen für `install_panic_hook` + Helper)
- `klarvo-core/src/i18n.rs` (1 Zeile in REQUIRED_KEYS)
- `shells/windows/src/locales/de.json` (1 Eintrag)
- `shells/windows/src/locales/en.json` (1 Eintrag)
- `shells/windows/src-tauri/src/main.rs` (1 Zeile: `install_panic_hook()`)

### References

- [prd.md FR39] — Uncaught Panics als level=ERROR tracing-Events
- [prd.md FR40] — `telemetry::export`-Module-Stub (UI-Triggered-Zip → Phase 2)
- [architecture.md §4 Telemetrie] — Panic-Hook in denselben Stream als level=ERROR
- [architecture.md §9 Observability] — `klarvo-core/src/telemetry/export.rs` Dateistruktur-Spec
- [memory/feedback_scaffold_fail_soft_pattern.md] — fail-soft AppError statt todo!/unimplemented!/panic
- [memory/project_no_remote_telemetry.md] — Export-Zip ist lokal, nicht remote
- Story 6.1 — etabliert `init_tracing` + `klarvo-core::telemetry`-Modul
- Story 5.5 — `disallowed_methods`-Lint-Scope für klarvo-core
- Story 5.6 — `required-keys-drift`-xtask-Gate
- Epic 9 (UX Surface) — Settings-UI-Trigger für Real-Export-Impl

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m] (create-story 2026-05-01); claude-sonnet-4-6 (implementation 2026-05-02)

### Debug Log References

- Locale-Pfad-Korrektur: Story-Spec nannte `shells/windows/src/locales/` (Frontend-Files), aber xtask lint-events und `include_str!` lesen `shells/windows/locales/` (Rust-Backend-Files). Key wurde in die korrekten Backend-Locale-Files eingefügt. Frontend-Files (`src/locales/`) ebenfalls gepflegt für Konsistenz.
- REQUIRED_KEYS-Const: Story-Spec setzt REQUIRED_KEYS in `klarvo-core/src/i18n.rs` voraus, aber diese Konstante wurde durch Story 5.6 durch `cargo xtask lint-events` G3-D ersetzt. Key in `shells/windows/locales/en.json` + `de.json` → xtask grün als Äquivalenz.
- `PanicHookInfo` (Rust 1.81+, Toolchain 1.94): Typ korrekt verwendet; `PanicInfo` für Hook-Use-Case ist deprecated.

### Completion Notes List

- AC-1: `install_panic_hook()` + `extract_panic_message()` in `klarvo-core/src/telemetry/logging.rs` implementiert. Hook ersetzt Default-Hook vollständig (kein prev_hook chaining). Payload-Extraktion: &str → String → Fallback. Backtrace via `Backtrace::force_capture()`. Felder: `panic.message`, `panic.location.file/line/column`, `panic.backtrace`.
- AC-2: `install_panic_hook()` direkt nach `init_tracing` in `shells/windows/src-tauri/src/main.rs`. Reihenfolge korrekt: Subscriber muss vor Hook aktiv sein.
- AC-3: `error.telemetry.export.unimplemented` in `shells/windows/locales/en.json` + `de.json` (Backend-Locale). `cargo xtask lint-events` → OK (G3-B forward-drift + G3-D orphan-check grün). Frontend-Locale-Files (`src/locales/`) ebenfalls gepflegt.
- AC-4: `klarvo-core/src/telemetry/export.rs` angelegt mit fail-soft Stub; `pub mod export;` in `telemetry/mod.rs`.
- AC-5: Headless-Test `export_debug_zip_returns_unimplemented_error` in `export.rs` — prüft `user_message` + `retryable: false`. Grün.
- AC-6: 3 Headless-Tests in `logging.rs` für `extract_panic_message`: static-str, String, non-string-fallback. Alle grün.
- AC-7 (Manuell): Smoke-Test erfordert laufende Windows-Tauri-App. Nicht automatisierbar in WSL/Linux-Build-Env. Zur Verifikation: temporär `panic!("smoke test")` direkt nach `install_panic_hook()` in `main.rs` einfügen, App starten, Rolling-File-Log unter `%APPDATA%\Klarvo\logs\klarvo.YYYY-MM-DD` prüfen — `level=ERROR`-Eintrag mit Backtrace erwartet. Dann `panic!` entfernen.
- Regression-Suite: 102 klarvo-core Tests + 55 xtask Tests + alle Workspace-Tests (excl. Windows-only) grün.

### File List

- `klarvo-core/src/telemetry/export.rs` (neu)
- `klarvo-core/src/telemetry/mod.rs` (geändert: `pub mod export;` hinzugefügt)
- `klarvo-core/src/telemetry/logging.rs` (geändert: `install_panic_hook` + `extract_panic_message` + 3 Tests)
- `shells/windows/locales/en.json` (geändert: `error.telemetry.export.unimplemented` hinzugefügt)
- `shells/windows/locales/de.json` (geändert: `error.telemetry.export.unimplemented` hinzugefügt)
- `shells/windows/src-tauri/src/main.rs` (geändert: `install_panic_hook()` nach `init_tracing`)
- `shells/windows/src/locales/{en,de}.json` (gelöscht via Code-Review-Patch P8 / Decision 3b — keine React-Konsumenten, Drift-Risiko statt Konsistenz-Nutzen)

## Change Log

- 2026-05-02: Story 6.3 implementiert — Panic-Hook (FR39) + telemetry::export-Stub (FR40). Neue Dateien: `export.rs`. Geänderte Dateien: `logging.rs`, `telemetry/mod.rs`, `main.rs`, 4× Locale-Files.
- 2026-05-02: Code-Review (Blind Hunter + Edge Case Hunter + Acceptance Auditor) — Findings unten in `### Review Findings`.
- 2026-05-02: Code-Review-Closure — 4 Decisions resolved (1a/2b/3b/4b), 8 Patches applied (inkl. `catch_unwind`-Reentry-Guard im Panic-Hook, Frontend-Locale-Files-Deletion, AC-3-Pfad-Korrektur + `REQUIRED_KEYS`-Spec-Amendment, Phase-2-Hinweise im `export.rs`-Doc-Comment), 1 zusätzlicher Defer (`6.3-W6`). Status `review` → `done`.

### Review Findings

**Decision-Needed (4)** — alle resolved:

- [x] [Review][Decision] **Panic-in-Hook → Process-Abort-Risiko** — Resolution **1a**: `catch_unwind(AssertUnwindSafe(...))` um Hook-Body + stderr-Fallback bei Hook-internem Panic. Patch applied in `klarvo-core/src/telemetry/logging.rs:111-130`.
- [x] [Review][Decision] **`Backtrace::force_capture` vs. `capture`** — Resolution **2b**: `force_capture` behalten (Backtrace-Garantie in Field-Logs > Allokationskosten), Begründung als Doc-Comment im `install_panic_hook`-Header dokumentiert. Patch applied in `klarvo-core/src/telemetry/logging.rs:94-110`.
- [x] [Review][Decision] **Frontend `shells/windows/src/locales/*.json` Dead-Files** — Resolution **3b**: Files + leeres Verzeichnis gelöscht; AC-3-Pfad-Liste auf Backend-only korrigiert; File List in dieser Story aktualisiert. Verifiziert via Grep (0 React-Konsumenten); xtask-Drift-Gate validiert weiterhin nur Backend.
- [x] [Review][Decision] **`install_panic_hook` ohne Integration-Test** — Resolution **4b**: Defer als `6.3-W6` in `deferred-work.md`; AC-7-Manual-Smoke deckt Phase-1-Need; `serial_test`-Infrastructure in Phase 2, sobald weitere global-state-Tests aufkommen.

**Patches (8)** — alle applied:

- [x] [Review][Patch] **AC-7 Manueller Smoke-Test als Windows-Carry-Over rephrast** (Task-Box jetzt `[ ]` mit explizitem Carry-Over-Marker)
- [x] [Review][Patch] **`AppErrorKind::Configuration` TODO(phase-2)-Sentinel ergänzt** [`klarvo-core/src/telemetry/export.rs:21-24`]
- [x] [Review][Patch] **AC-3 `REQUIRED_KEYS`-Deviation als Sub-Task-Wording-Update + Change-Log-Eintrag formalisiert**
- [x] [Review][Patch] **AC-3 Locale-Pfad-Liste auf Backend-only korrigiert** (kombiniert mit Frontend-Delete unten)
- [x] [Review][Patch] **`export.rs`-Doc-Comment um NFR5/Path-Traversal/Single-Flight/i18n-Resolve-Hinweise für Phase-2-Real-Impl ergänzt** [`klarvo-core/src/telemetry/export.rs:1-13`]
- [x] [Review][Patch] **`catch_unwind(AssertUnwindSafe(...))` + stderr-Fallback um Hook-Body** (aus Decision 1a) [`klarvo-core/src/telemetry/logging.rs:111-130`]
- [x] [Review][Patch] **Doc-Comment-Block in `install_panic_hook` zur bewussten `force_capture`-Wahl + Reentry-Safety** (aus Decision 2b) [`klarvo-core/src/telemetry/logging.rs:94-110`]
- [x] [Review][Patch] **Frontend-Locale-Files `shells/windows/src/locales/{en,de}.json` gelöscht + leeres Verzeichnis entfernt** (aus Decision 3b)

**Deferred (5)** — pre-existing oder Phase-2:

- [x] [Review][Defer] **`extract_panic_message` deckt nicht `Box<dyn Error>` / `anyhow::Error` / non-static `&str`** [`klarvo-core/src/telemetry/logging.rs:84-90`] — Spec-AC-1 mandatet nur `&str`/`String`/Fallback; Erweiterung Phase-2 wenn anyhow/eyre eingeführt
- [x] [Review][Defer] **Concurrent `export_debug_zip` calls — kein Single-Flight-Guard** [`klarvo-core/src/telemetry/export.rs:17`] — Stub returnt immer Err, Race irrelevant; Phase-2-Real-Impl-Concern
- [x] [Review][Defer] **`process::exit(1)` in main.rs droppt `_tracing_guard` während Hook noch installed** [`shells/windows/src-tauri/src/main.rs:489-499`] — pre-existing Pattern, außerhalb Story-6.3-Scope
- [x] [Review][Defer] **OOM-Panic → Backtrace-Allocation-Failure → Hook-Abort** [`klarvo-core/src/telemetry/logging.rs:108`] — Phase-2-OOM-Handling, niedrige Priorität
- [x] [Review][Defer] **`init_tracing`→None silent-failure: Hook fires gegen No-Op-Subscriber, Panics gehen verloren** [`shells/windows/src-tauri/src/main.rs:22-23`] — pre-existing aus Story 6.1; gehört zu Story-6.1-Carry-Over (Boot-Error-UX)

**Dismissed (7)** — nicht persistiert, nur zur Transparenz: Test-Payload `42u32` (deckt Any-Fallback äquivalent), `tracing` dotted field-names (Standard-Konvention, kein aktiver Konsument), Sprint-Status `review` mit allen Tasks `[x]` (genau der BMad-Workflow), Stub-Dev-Jargon-Message in `err.message` (Standard-Pattern, `user_message` ist localized), AC-1 `unwrap_or_else` vs. `unwrap_or` (semantisch äquivalent, Refactor von AC-6 invited), Multi-Line-Panic-Message bricht line-per-event (kein Downstream-Parser, lokale Logs only per `project_no_remote_telemetry`), Threads-spawned-before-hook (set_hook ist global, alle Threads inheriten panic-time-current).
