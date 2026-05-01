---
name: Story 6.1 — telemetry::logging — tracing-subscriber + rolling-file appender
epic: 6
story_number: "6.1"
status: ready-for-dev
dependencies: []
---

# Story 6.1: `telemetry::logging` — tracing-subscriber + rolling-file appender

Status: ready-for-dev

## Story

Als Core-Dev / Shell-Dev
möchte ich `klarvo-core::telemetry::logging` mit `tracing-subscriber` + `tracing-appender` implementieren und den Windows-Shell-Bootstrapper damit initialisieren,
damit alle `tracing!`-Events in eine Rolling-Log-Datei unter `%APPDATA%\klarvo\logs\` geschrieben werden und DEBUG/TRACE-Events im Release-Build gefiltert sind (PII-Protection per NFR5).

## Kontext und Motivation

**Ausgangslage:** `tracing::info!()`, `tracing::error!()` etc. werden seit Epic 1A/2/3/4 durchgehend verwendet, aber es ist kein Subscriber konfiguriert — alle Events gehen ins Void. FR37 (Rolling-File-Log) aus Epic 6 schließt diese Lücke.

**Kein Telemetry-Modul in klarvo-core:** `klarvo-core/src/lib.rs` hat kein `pub mod telemetry`, obwohl `architecture.md §9 Observability` und die Dateistruktur-Spec `klarvo-core/src/telemetry/{mod.rs, logging.rs, export.rs}` beschreiben. Das Modul wird in Story 6.1 erstellt (`export.rs` folgt in Story 6.3).

**Forcing-Sentinel-Hinweis:** `xtask/src/verify_release.rs::check_tracing_subscriber_sentinel` schlägt fehl sobald `tracing-subscriber` als Dependency aufgenommen wird. **Story 6.1 löst diesen Sentinel NICHT auf** — das ist Story 6.2 (verify_release-Filter-Gate). Story 6.1 lässt `cargo xtask verify-release` daher temporär rot. Das ist gewollt: der Sentinel forciert genau diese Sequenz (6.1 fügt Dep hinzu → 6.2 ersetzt Sentinel durch echten Gate). Andere CI-Gates (`cargo build`, `cargo test`, `cargo xtask lint-events` etc.) bleiben grün.

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

### AC-6: Headless-Test in `klarvo-core`

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
- [ ] Headless-Test (AC-6)

## Dev Notes

### Bestehende Tracing-Nutzung

`tracing = "0.1"` ist im Workspace. Alle Crates nutzen `tracing::info!()`, `tracing::error!()` etc. bereits — es fehlt nur der Subscriber. Story 6.1 installiert ihn; kein bestehender Code muss geändert werden (ausser `main.rs` + `lib.rs` + Cargo.toml-Files).

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

### `RELEASE_MAX_LEVEL`-Konstante als Story-6.2-Sentinel

Die `RELEASE_MAX_LEVEL`-Konstante (AC-4) dient nicht nur dem Subscriber-Setup, sondern wird in Story 6.2 als Source-Grep-Sentinel im neuen `verify_release`-Gate verwendet. Der genaue Token-Name `RELEASE_MAX_LEVEL` ist deshalb load-bearing — bitte nicht abkürzen oder umbenennen. Spec Story 6.2 AC-2.

### `set_global_default` darf nur einmal aufgerufen werden

`tracing_subscriber::registry()...init()` installiert den Subscriber global. In Tests mit mehreren `init_tracing`-Aufrufen schlägt der zweite mit `SetGlobalDefaultError` fehl. Lösung: Test testet nur den Fehler-Pfad VOR der Installation (AC-6). Für Tests die tatsächlich Tracing testen: `#[cfg(test)]`-Subscriber via `tracing_subscriber::fmt().with_test_writer().init()` in einem `#[test]`-Scope (aber das ist Story 6.3 oder spätere Story).

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

`telemetry/export.rs` wird in Story 6.3 erstellt (FR40 Export-Stub + FR39 Panic-Hook). Noch NICHT in Story 6.1 anlegen.

`xtask/src/verify_release.rs` wird in Story 6.2 angefasst (Sentinel-Replacement). Noch NICHT in Story 6.1 anfassen — Story 6.1 lässt den Sentinel temporär feuern (gewollt, siehe Kontext-Sektion).

### References

- [architecture.md §4 Telemetrie / §4a Release-Hardening] — Rolling-file spec, Release-Filter-Requirement
- [architecture.md §9 Observability] — `klarvo-core/src/telemetry/` Dateistruktur-Spec
- [prd.md §Journey Requirements: Rolling-File-Log] — `%APPDATA%/klarvo/logs/`, max 10 MB, 5 Rotations
- [prd.md FR37] — Structured logs in Rolling-File mit konfigurierbarer Verbosity
- [prd.md NFR5] — Kein Audio/Text im Log
- [memory/project_no_remote_telemetry.md] — BYOK-Narrativ, kein Sentry
- [memory/feedback_scaffold_fail_soft_pattern.md] — fail-soft returns structured error / None, nie panic
- [shells/windows/src-tauri/src/config.rs:104-108] — Pattern für APPDATA-Env-Var (Referenz für main.rs-Init)
- Story 6.2 (verify_release-Filter-Gate) konsumiert die in Story 6.1 etablierte `RELEASE_MAX_LEVEL`-Konstante

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story 2026-05-01)

### Debug Log References

### Completion Notes List

### File List
