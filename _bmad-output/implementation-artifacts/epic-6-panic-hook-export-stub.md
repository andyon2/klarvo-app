---
name: Story 6.3 — Panic-Hook + telemetry::export-Stub
epic: 6
story_number: "6.3"
status: backlog
dependencies:
  - "6-1-telemetry-logging-rolling-file"
---

# Story 6.3: Panic-Hook + `telemetry::export`-Stub

Status: backlog

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

- [ ] Panic-Hook implementieren (AC-1)
  - [ ] `install_panic_hook` in `logging.rs`
  - [ ] Payload-Extraktions-Helper (Refactor für AC-6 Testbarkeit)
- [ ] Windows-Shell-Bootstrapper anpassen (AC-2)
  - [ ] `main.rs`: `install_panic_hook()` direkt nach `init_tracing`
- [ ] i18n-Keys (AC-3)
  - [ ] `error.telemetry.export.unimplemented` in `REQUIRED_KEYS`
  - [ ] Übersetzungen in `de.json` + `en.json`
  - [ ] `cargo xtask required-keys-drift` grün
- [ ] Export-Stub (AC-4)
  - [ ] `klarvo-core/src/telemetry/export.rs` anlegen
  - [ ] `pub mod export;` in `mod.rs`
- [ ] Tests (AC-5 + AC-6)
- [ ] Manueller Smoke-Test + Documentation in Completion Notes (AC-7)

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

claude-opus-4-7[1m] (create-story 2026-05-01)

### Debug Log References

### Completion Notes List

### File List
