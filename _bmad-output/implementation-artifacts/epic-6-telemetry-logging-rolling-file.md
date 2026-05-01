---
name: Story 6.1 — telemetry::logging — tracing-subscriber + rolling-file appender + release-filter gate
epic: 6
story_number: "6.1"
status: ready-for-dev
dependencies: []
---

# Story 6.1: `telemetry::logging` — tracing-subscriber + rolling-file appender + release-filter gate

Status: ready-for-dev

## Story

Als Core-Dev / Shell-Dev
möchte ich `klarvo-core::telemetry::logging` mit `tracing-subscriber` + `tracing-appender` implementieren und den Windows-Shell-Bootstrapper damit initialisieren,
damit alle `tracing!`-Events in eine Rolling-Log-Datei unter `%APPDATA%\klarvo\logs\` geschrieben werden, DEBUG/TRACE-Events im Release-Build gefiltert sind (PII-Protection per NFR5), und der `verify_release`-Sentinel durch einen echten Release-Filter-Gate ersetzt wird.

## Kontext und Motivation

**Ausgangslage:** `tracing::info!()`, `tracing::error!()` etc. werden seit Epic 1A/2/3/4 durchgehend verwendet, aber es ist kein Subscriber konfiguriert — alle Events gehen ins Void. FR37 (Rolling-File-Log) + FR38 (no remote telemetry) aus Epic 6 schließen diese Lücke.

**Forcing-Sentinel:** `xtask/src/verify_release.rs::check_tracing_subscriber_sentinel` schlägt fehl wenn `tracing-subscriber` als Dependency vorhanden ist. Das ist ein Forcing-Sentinel (CI-Gate-Philosophy, `memory/feedback_ci_gate_philosophy.md`): "wer `tracing-subscriber` hinzufügt, muss den echten Filter-Check implementieren und den Sentinel löschen." Story 6.1 löst diesen Sentinel ein.

**Kein Telemetry-Modul in klarvo-core:** `klarvo-core/src/lib.rs` hat kein `pub mod telemetry`, obwohl `architecture.md §9 Observability` und die Dateistruktur-Spec `klarvo-core/src/telemetry/{mod.rs, logging.rs, export.rs}` beschreiben. Das Modul wird in Story 6.1 erstellt (`export.rs` folgt in Story 6.2).

## Acceptance Criteria

### AC-1: Workspace-Dependencies hinzugefügt

**Given** `Cargo.toml` (Workspace-Root) hat kein `tracing-subscriber`- und kein `tracing-appender`-Eintrag,
**When** Story 6.1 committed ist,
**Then** enthält `Cargo.toml` (Workspace `[dependencies]`):
- `tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt", "registry"] }`
- `tracing-appender = "0.2"`

Und `klarvo-core/Cargo.toml` referenziert beide als `tracing-subscriber.workspace = true` + `tracing-appender.workspace = true`.

### AC-2: `telemetry`-Modul in klarvo-core erstellt

**Given** `klarvo-core/src/lib.rs` hat kein `pub mod telemetry`,
**When** Story 6.1 committed ist,
**Then** existiert:
- `klarvo-core/src/telemetry/mod.rs` mit `pub mod logging;`
- `klarvo-core/src/telemetry/logging.rs` (Impl — AC-3/4/5)
- `klarvo-core/src/lib.rs` enthält `pub mod telemetry;`

### AC-3: `init_tracing`-Funktion mit rolling-file appender

**Given** `logging.rs` existiert,
**When** `init_tracing(log_dir: &Path) -> Option<tracing_appender::non_blocking::WorkerGuard>` aufgerufen wird,
**Then**:
- Rolling-file appender via `tracing_appender::rolling::RollingFileAppender::builder()` mit DAILY-Rotation, filename-prefix `klarvo`, max. 5 gehaltene Log-Files, Verzeichnis = `log_dir`
- Non-blocking writer via `tracing_appender::non_blocking(file_appender)` — gibt `(NonBlocking, WorkerGuard)` zurück
- Subscriber via `tracing_subscriber::registry()` + fmt-Layer mit `with_ansi(false)` + LevelFilter (AC-4)
- `tracing::subscriber::set_global_default(subscriber)` oder `.init()` — global installiert
- Rückgabe: `Some(guard)` bei Erfolg, `None` bei Fehler (fail-soft: `create_dir_all` + Builder-Fehler werden gelogt über `eprintln!` und resultieren in keinem Subscriber — kein Panic)
- `None`-Rückgabe heißt: App startet ohne File-Logging; kein Absturz

### AC-4: Release-Filter — kein DEBUG/TRACE im Release-Build

**Given** `init_tracing` wird in einem Release-Build (`cfg(not(debug_assertions))`) aufgerufen,
**Then** ist der LevelFilter `LevelFilter::INFO` (INFO, WARN, ERROR passieren; DEBUG, TRACE werden gedroppt).

**Given** `init_tracing` wird in einem Debug-Build aufgerufen,
**Then** ist der LevelFilter `LevelFilter::DEBUG` (alle Events ausser TRACE passieren; TRACE bleibt optional via RUST_LOG oder Env-Var).

Konkret: in `logging.rs` gibt es zwei sichtbare Konstanten oder cfg-Blöcke:
```rust
#[cfg(not(debug_assertions))]
const RELEASE_MAX_LEVEL: tracing_subscriber::filter::LevelFilter =
    tracing_subscriber::filter::LevelFilter::INFO;

#[cfg(debug_assertions)]
const RELEASE_MAX_LEVEL: tracing_subscriber::filter::LevelFilter =
    tracing_subscriber::filter::LevelFilter::DEBUG;
```
Diese Konstanten dienen auch dem xtask-Filter-Gate (AC-6) als Sentinel.

**Module-doc in `logging.rs` muss explicit enthalten:**
> NFR5: Audio-Daten (PCM-Samples, Rohaudio) und Transkriptions-Text (STT-Output, LLM-Output) DÜRFEN NICHT geloggt werden — weder in DEBUG- noch in TRACE-Events. Logging beschränkt sich auf Metadata: Event-Typen, Error-Keys, Latency-Werte (ts_ms), Plugin-IDs, Byte-Counts.

### AC-5: Windows-Shell bootstrappt tracing vor tauri::Builder

**Given** `shells/windows/src-tauri/src/main.rs` hat kein Tracing-Init,
**When** Story 6.1 committed ist,
**Then**:
- Am Anfang von `fn main()`, VOR `let specta_builder = ...` und VOR `tauri::Builder::default()`:
  ```rust
  let log_dir = std::env::var("APPDATA")
      .map(|d| std::path::PathBuf::from(d).join("klarvo").join("logs"))
      .unwrap_or_else(|_| std::env::temp_dir().join("klarvo").join("logs"));
  let _tracing_guard = klarvo_core::telemetry::logging::init_tracing(&log_dir);
  ```
- `_tracing_guard` (NICHT `_` alleine — Unterstrich-Prefix hält den Guard am Leben bis `main()` endet, einfacher `_` würde ihn sofort droppen)
- Bootstrap-Kommentar aktualisiert: Step 0 bleibt `TauriErrorEmitter::new`, davor wird Logging-Init dokumentiert

**Note zum Timing:** `APPDATA`-Env-Var ist identisch zu `config::resolve_config_path()` (Step 1 im bisherigen Bootstrap). Logging-Init ist vor Step 0 und hat keinen Tauri-Kontext — nur `APPDATA`. Dieser Pfad-Ansatz ist konsistent mit `config.rs::resolve_config_path()`.

### AC-6: `verify_release`-Sentinel ersetzt durch echten Release-Filter-Gate

**Given** `xtask/src/verify_release.rs::check_tracing_subscriber_sentinel` schlägt fehl wenn `tracing-subscriber` present,
**When** Story 6.1 committed ist,
**Then**:
- Funktion `check_tracing_subscriber_sentinel` ist **gelöscht**
- Neue Funktion `check_tracing_release_filter(metadata: &Metadata) -> Result<(), String>`:
  1. **Check 1 — tracing-subscriber present:**
     `metadata.packages.iter().any(|p| p.name == "tracing-subscriber")` — wenn NICHT present: `Err("tracing-subscriber missing — rolling-file logging requires it; add to workspace Cargo.toml")`
  2. **Check 2 — release-filter sentinel in source:**
     Liest `{workspace_root}/klarvo-core/src/telemetry/logging.rs` via `locate_workspace_root()` + `std::fs::read_to_string`
     Prüft ob Datei `RELEASE_MAX_LEVEL` AND `LevelFilter::INFO` AND `not(debug_assertions)` enthält — wenn nicht: `Err("release-level filter (LevelFilter::INFO behind cfg(not(debug_assertions))) not found in telemetry/logging.rs — PII-Protection: DEBUG/TRACE must not reach release builds. Spec: architecture.md §4a.")`
- `run()`-Funktion ruft `check_tracing_release_filter` statt `check_tracing_subscriber_sentinel`
- Module-doc (`//!`) von `verify_release.rs` aktualisiert: Check #2 beschreibt den neuen Filter-Gate statt des Sentinels
- Unit-Tests aktualisiert (alte Sentinel-Tests gelöscht, neue Tests für Check 1 + Check 2 hinzugefügt)

### AC-7: `cargo xtask verify-release` grün nach Story 6.1

**Given** Story 6.1 ist committed,
**When** `cargo xtask verify-release --skip-cross-compile` läuft,
**Then** exitiert mit Code 0 (keine Violations).

### AC-8: Headless-Test in `klarvo-core`

**Given** `logging.rs` ist implementiert,
**Then** existiert in `klarvo-core/src/telemetry/logging.rs` (oder `tests/`-Module) mindestens ein Headless-Test:
- `init_tracing_with_nonexistent_dir_returns_none`: Ruft `init_tracing(&PathBuf::from("/nonexistent/path/that/cannot/be/created"))` — erwartet `None` ohne Panic
- Hinweis: `set_global_default` kann pro Test-Binary nur einmal aufgerufen werden; Test-Teardown ist nicht möglich. Der Test darf nur den Fehler-Pfad (pre-subscriber-install) testen.

## Tasks / Subtasks

- [ ] Workspace-Deps hinzufügen (AC-1)
  - [ ] `Cargo.toml` (root): `tracing-subscriber`, `tracing-appender` eintragen
  - [ ] `klarvo-core/Cargo.toml`: workspace-refs eintragen
- [ ] `telemetry`-Modul erstellen (AC-2)
  - [ ] `klarvo-core/src/telemetry/mod.rs` anlegen
  - [ ] `klarvo-core/src/lib.rs`: `pub mod telemetry;` eintragen
- [ ] `logging.rs` implementieren (AC-3 + AC-4)
  - [ ] `RELEASE_MAX_LEVEL`-Konstante mit cfg-Gates
  - [ ] `init_tracing(log_dir: &Path) -> Option<WorkerGuard>` fail-soft
  - [ ] NFR5-Kommentar in Module-doc
- [ ] Windows-Shell-Bootstrapper anpassen (AC-5)
  - [ ] `main.rs`: `_tracing_guard` vor specta_builder
  - [ ] Bootstrap-Kommentar im Logging-Block
- [ ] `verify_release.rs` Sentinel ersetzen (AC-6)
  - [ ] Alte Funktion + Tests löschen
  - [ ] `check_tracing_release_filter` implementieren (2 Checks)
  - [ ] Module-doc aktualisieren
  - [ ] Unit-Tests für neuen Gate
- [ ] `cargo xtask verify-release` grün (AC-7)
- [ ] Headless-Test (AC-8)

## Dev Notes

### Bestehende Tracing-Nutzung

`tracing = "0.1"` ist im Workspace. Alle Crates nutzen `tracing::info!()`, `tracing::error!()` etc. bereits — es fehlt nur der Subscriber. Story 6.1 installiert ihn; kein bestehender Code muss geändert werden (ausser `main.rs` + `lib.rs` + `verify_release.rs`).

### tracing-appender Builder API

`tracing_appender::rolling::RollingFileAppender::builder()` (verfügbar seit 0.2.3) hat:
- `.rotation(Rotation::DAILY)` — täglich neue Datei (HOURLY/NEVER ebenfalls verfügbar; NEVER für Tests sinnvoll)
- `.filename_prefix("klarvo")` — generiert `klarvo.YYYY-MM-DD` als Dateinamen
- `.max_log_files(5)` — hält max. 5 Rotations-Files; ältere werden gelöscht
- `.build(directory) -> Result<RollingFileAppender, InitError>`

`tracing_appender::non_blocking(appender)` gibt `(NonBlocking, WorkerGuard)` zurück. Der `WorkerGuard` flusht den Writer beim Drop. Wird der Guard vor App-Ende gedroppt, gehen Logs verloren.

### `disallowed_methods`-Lint (Epic 5.5)

`clippy::disallowed_methods` in `klarvo-core` + `klarvo-windows-shell` (`.clippy.toml`) verbietet `.unwrap()` + `.expect()` in Production-Code. In `logging.rs` (klarvo-core) und `main.rs` (windows-shell) daher:
- Statt `.expect("msg")` → `.ok()?` oder `.unwrap_or_else(|e| { eprintln!(...); return None; })`
- Statt `.unwrap()` → `?`-Operator oder expliziten `match`

`xtask`-Crate ist nicht vom Lint betroffen (kein Production-Crate).

### WorkerGuard in main.rs

Naming ist kritisch:
```rust
let _tracing_guard = init_tracing(...);  // KORREKT: lebt bis Ende von main()
let _ = init_tracing(...);               // FALSCH: wird sofort gedroppt (Rust-Semantik)
```

`Option<WorkerGuard>` — kein `.unwrap()`. Pattern: `let _tracing_guard = init_tracing(&log_dir);` (kein Destructuring nötig).

### verify_release: `locate_workspace_root()`

In `verify_release.rs` bereits vorhanden:
```rust
fn locate_workspace_root() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(Path::to_path_buf)
}
```

Für Check 2 in `check_tracing_release_filter`:
```rust
let root = locate_workspace_root().ok_or("could not locate workspace root")?;
let logging_src = root.join("klarvo-core/src/telemetry/logging.rs");
let content = std::fs::read_to_string(&logging_src)
    .map_err(|e| format!("cannot read telemetry/logging.rs: {e}"))?;
if !content.contains("RELEASE_MAX_LEVEL") || !content.contains("not(debug_assertions)") {
    return Err("release-level filter missing in telemetry/logging.rs — ...".into());
}
```

### `set_global_default` darf nur einmal aufgerufen werden

`tracing_subscriber::registry()...init()` installiert den Subscriber global. In Tests mit mehreren `init_tracing`-Aufrufen schlägt der zweite mit `SetGlobalDefaultError` fehl. Lösung: Test testet nur den Fehler-Pfad VOR der Installation (AC-8). Für Tests die tatsächlich Tracing testen: `#[cfg(test)]`-Subscriber via `tracing_subscriber::fmt().with_test_writer().init()` in einem `#[test]`-Scope (aber das ist Story 6.2 oder spätere Story).

### tracing-subscriber features

Default-Features von `tracing-subscriber 0.3` inkludieren `fmt` und `ansi`. Mit `default-features = false, features = ["fmt", "registry"]`:
- `fmt`: FmtSubscriber + fmt::Layer
- `registry`: `tracing_subscriber::registry()` Basis
- Ohne `ansi`: kein farbiger Output im File-Log (bei `with_ansi(false)` ohnehin deaktiviert, aber ohne das Feature spart man den ansi-crate-Overhead)

Falls `default-features = false` zu Build-Problemen führt, Feature-Flags auf `features = ["fmt"]` reduzieren und testen.

### NFR5 — was NICHT geloggt werden darf

Audio-Samples (PCM-Daten), STT-Output-Text, LLM-Output-Text. Diese dürfen weder in DEBUG- noch TRACE-Events landen. Konkrete gefährliche Pattern die der Dev-Agent vermeiden muss:
```rust
tracing::debug!(audio_data = ?samples, ...);   // VERBOTEN
tracing::trace!(transcript = %text, ...);       // VERBOTEN
tracing::debug!(payload_bytes = samples.len()); // ERLAUBT (Metadaten)
tracing::info!(ts_ms = ts, stage = "stt");      // ERLAUBT
```

### Log-Timestamps

Aus `architecture.md §4 Naming Patterns` (Zeile 478): **Log-Timestamps sind ISO-8601** (menschliche Konsumenten). Der fmt-Layer von tracing-subscriber formatiert Timestamps standardmäßig per `time`-crate oder `chrono`. Default-Format von `tracing_subscriber::fmt` ohne extra Config: humantime-style. Für ISO-8601 explizit: `fmt::layer().with_timer(tracing_subscriber::fmt::time::OffsetTime::local_rfc_3339())` — optional für Phase 1, aber empfohlen.

### Project Structure Notes

Neue Dateien:
- `klarvo-core/src/telemetry/mod.rs`
- `klarvo-core/src/telemetry/logging.rs`

Geänderte Dateien:
- `klarvo-core/src/lib.rs` (1 Zeile: `pub mod telemetry;`)
- `klarvo-core/Cargo.toml` (2 Zeilen workspace-refs)
- `Cargo.toml` (2 Zeilen workspace-deps)
- `shells/windows/src-tauri/src/main.rs` (~5-8 Zeilen)
- `xtask/src/verify_release.rs` (Sentinel-Delete + neuer Gate + Tests)

`telemetry/export.rs` wird in Story 6.2 erstellt (FR40 Export-Stub + FR39 Panic-Hook). Noch NICHT in Story 6.1 anlegen.

### References

- [architecture.md §4 Telemetrie / §4a Release-Hardening] — Rolling-file spec, Release-Filter-Requirement
- [architecture.md §9 Observability] — `klarvo-core/src/telemetry/` Dateistruktur-Spec
- [prd.md §Journey Requirements: Rolling-File-Log] — `%APPDATA%/klarvo/logs/`, max 10 MB, 5 Rotations
- [prd.md FR37/FR38] — Structured logs + No remote telemetry
- [prd.md NFR5] — Kein Audio/Text im Log
- [memory/feedback_ci_gate_philosophy.md] — Forcing-Sentinel-Pattern, keine Stub-Checks
- [memory/project_no_remote_telemetry.md] — BYOK-Narrativ, kein Sentry
- [xtask/src/verify_release.rs:190-206] — aktueller Sentinel (zu ersetzen)
- [xtask/src/verify_release.rs:137-139] — `locate_workspace_root()`
- [memory/feedback_scaffold_fail_soft_pattern.md] — fail-soft returns structured error / None, nie panic
- [shells/windows/src-tauri/src/config.rs:104-108] — Pattern für APPDATA-Env-Var (Referenz für main.rs-Init)

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story 2026-05-01)

### Debug Log References

### Completion Notes List

### File List
