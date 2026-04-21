---
name: Story 3.7 — CpalAudioSource → Orchestrator Wire-Up
epic: 3
story_number: "3.7"
status: Draft
dependencies:
  - "3.3"
---

# Story 3.7: `CpalAudioSource` → Orchestrator Wire-Up

## Outcome

`shells/windows/src-tauri/src/audio.rs` liefert eine Factory-Funktion
`make_audio_source() -> Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>`, die
`CpalAudioSource` konstruiert und für Dependency-Injection in `SessionOrchestrator`
geeignet wrapped. Compile-Kompatibilität zwischen Factory-Return-Type und
Orchestrator-Constructor-Feld-Type ist via Compile-Test verifiziert.
Bootstrap-Nutzung erfolgt in Story 3.10.

## Acceptance Criteria

### AC-A — Factory-Function-Signatur

**Given** `CpalAudioSource` existiert in `klarvo-audio-cpal` (commit `37b57c1`, Story 2.5)  
**When** `make_audio_source()` implementiert wird  
**Then**

- Exakte Signatur:
  ```rust
  pub fn make_audio_source() -> Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>
  ```
- Die Factory konstruiert `CpalAudioSource` (Unit-Struct, kein Constructor-Argument —
  Delegate-Finding: `pub struct CpalAudioSource;`, keine `emitter`/`clock`-Deps im
  Konstruktor, Error-Emission erfolgt via `tracing::warn!` direkt im cpal-Callback)
- Return-Type ist identisch mit dem `audio_source`-Feld-Type in `SessionOrchestrator`
  (ref ADR-0012 §SD-2)

### AC-B — CpalAudioSource-Construction

**Given** `CpalAudioSource` ist ein Unit-Struct ohne Constructor-Argumente  
**When** die Factory aufgerufen wird  
**Then**

- Factory-Body:
  ```rust
  pub fn make_audio_source() -> Arc<tokio::sync::Mutex<Box<dyn AudioSource>>> {
      Arc::new(tokio::sync::Mutex::new(Box::new(CpalAudioSource)))
  }
  ```
- Audio-Config (Sample-Rate, Channel-Count) ist intern `CpalAudioSource`-Default:
  Default-Host → Default-Input-Device → Default-Input-Config. ShellConfig-getriebene
  Audio-Config ist Phase-2-Backlog.
- Rustdoc am Factory-Body enthält Phase-2-Forward-Reference:
  `// Phase-2: drive sample_rate/channels from ShellConfig (currently CpalAudioSource defaults).`

### AC-C — `#[cfg(target_os = "windows")]`-Gate

**Given** das Windows-Shell-Crate (`shells/windows/src-tauri/`) ist per Story 3.1 AC-E
Crate-weit mit `#[cfg(target_os = "windows")]` gegated  
**When** `audio.rs` hinzugefügt wird  
**Then**

- `audio.rs` braucht kein eigenes File-Level-Gate; das Crate-Level-Gate ist ausreichend
- `klarvo-audio-cpal` ist in `shells/windows/src-tauri/Cargo.toml` bereits als
  Target-spezifische Dependency gelistet: `[target.'cfg(target_os = "windows")'.dependencies]`;
  Delegate verifiziert; falls nicht: Cargo.toml-Add ist Scope dieser Story
- Ein Modul-Kommentar an `audio.rs`-Kopf expliziert:
  `// Windows-only: CpalAudioSource via klarvo-audio-cpal (WASAPI backend). No cross-platform path.`

### AC-D — Rustdoc-Contract

**Given** `make_audio_source()` ist implementiert  
**When** Rustdoc auf der Factory-Funktion geschrieben wird  
**Then**

- Rustdoc expliziert:
  - Feld-Type in `SessionOrchestrator::new(audio_source: Arc<Mutex<Box<dyn AudioSource>>>)`
  - Grund für `Arc<Mutex<Box<dyn AudioSource>>>` statt direktem Ownership:
    `// Mutex: AudioSource::start takes &mut self (ADR-0006 compile-time borrow-guard).`
    `// Arc: shared-ownership between Orchestrator and potential future diagnostics.`
  - Forward-Reference:
    `// Story 3.10 calls make_audio_source() in main.rs .setup() hook.`
  - Hinweis auf CpalAudioSource-Error-Model:
    `// CpalAudioSource emits capture errors via tracing::warn! (not ErrorEmitter).`
    `// Stream-error-path closes the broadcast channel (RecvError::Closed) → pipeline`
    `// terminates naturally. See klarvo-audio-cpal/src/source.rs error-callback.`

### AC-E — Compile-Test

**Given** `make_audio_source()` ist implementiert  
**When** der Test ausgeführt wird  
**Then**

- Ein Unit-Test oder inline `#[test]` in `audio.rs` verifiziert Trait-Object-Kompatibilität:
  ```rust
  #[test]
  fn audio_source_factory_produces_orchestrator_compatible_type() {
      let _audio: Arc<tokio::sync::Mutex<Box<dyn AudioSource>>> = make_audio_source();
      // Compile-check only; no runtime start (real cpal-device-init requires OS audio device).
      // If this compiles, the return type is compatible with SessionOrchestrator::new().
  }
  ```
- Test läuft via `cargo test -p <windows-shell-crate> audio_source_factory` (oder analog)
- `#[cfg(test)]` + `#[cfg(target_os = "windows")]` nötig falls Test in einem Cross-Platform-CI läuft

### AC-F — Scope-Fence

**Given** diese Story liefert nur die Factory  
**When** Code geschrieben wird  
**Then**

- Kein `SessionOrchestrator::new(...)`-Call in dieser Story — Bootstrap passiert in Story 3.10
- Kein `CpalAudioSource::start()`-Call in dieser Story — nur Factory-Wrapping
- Keine neuen i18n-Keys in dieser Story (ref AC-G)

### AC-G — No New i18n-Keys

Diese Story registriert keine neuen i18n-Keys. Errors, die `CpalAudioSource` selbst
emittiert, nutzen den `error.audio.*`-Präfix aus `klarvo-audio-cpal` (Epic-1/2-Scope).
CpalAudioSource emittiert Errors via `tracing::warn!` (nicht via `ErrorEmitter`-Trait) —
kein i18n-Key-Contract für cpal-Callback-Errors in Phase 1.

## Technical Notes

### Warum Factory statt direktem `CpalAudioSource` in Story 3.10

Bootstrap-Code in Story 3.10 (`main.rs`) wird lean: ein einzelner `make_audio_source()`-Call,
keine `Arc<tokio::sync::Mutex<Box<...>>>`-Wrapping-Boilerplate. Für Tests in Story 3.10
kann eine gemockte Factory-Variante verwendet werden ohne den Orchestrator-Constructor zu ändern.
Ergonomie-Gewinn ohne Phase-1-Overhead.

### CpalAudioSource-Error-Model (Abweichung vom ADR-0009-SD-3-Entwurf)

ADR-0009 §SD-3 identifiziert `CpalAudioSource`-Callback-Context als Primary-Consumer
des `ErrorEmitter`-Traits. Die tatsächliche Phase-1-Impl (`klarvo-audio-cpal/src/source.rs`,
commit `37b57c1`) emittiert stream-errors via `tracing::warn!` und schließt den
Broadcast-Channel (`*slot_err.lock().unwrap() = None`). Kein `ErrorEmitter`-Injection
im `CpalAudioSource`-Konstruktor.

**Konsequenz für Story 3.7:** Factory-Signatur ist simpler als in ADR-0009 §SD-3 skizziert.
`ErrorEmitter` + `Clock` sind keine Factory-Parameter.

**Konsequenz für ADR-0009:** SD-3 Primary-Consumer-Behauptung ist im Phase-1-Code nicht
materialisiert. Der `ErrorEmitter`-Trait wird stattdessen vom Orchestrator direkt genutzt
(Pipeline-Fail-Paths, AC-E in Story 3.3). ADR-0009-Amendment ist für SD-3-Primary-Consumer-Note
optional (die Amendment-1 konkretisiert bereits die Signatur; die Consumer-Lücke ist ein
Open-Observation für den Reviewer).

### Phase-2-Expansion

Sample-Rate und Channel-Count aus `ShellConfig` (Story 3.2) wären die natürliche
Phase-2-Erweiterung: `CaptureConfig { sample_rate: config.audio_sample_rate.unwrap_or(16000), ... }`.
Aktuell hardcoded in `CpalAudioSource::start` via `default_input_config()`.

### Cross-Reference: CpalAudioSource-Commit

`klarvo-audio-cpal/src/source.rs` wurde in commit `37b57c1` (Story 2.5) implementiert.
`CpalAudioSource` ist ein Unit-Struct; `start(&mut self, config: CaptureConfig)` ist die
einzige öffentliche Methode (via `AudioSource`-Trait).

## Dependencies

- Story 3.3 — `SessionOrchestrator` (Factory-Return-Type muss Orchestrator-Feld-Type matchen)
- `klarvo-audio-cpal` (commit `37b57c1`) — `CpalAudioSource`-Impl (authoritative Konstruktor-Shape)
- ADR-0012 §SD-2 — `SessionOrchestrator`-API-Surface (Feld-Types)
- ADR-0006 — `AudioSource`-Trait + `CaptureHandle` (Compile-Contract)
- `memory/project_shell_runtime_model` — Single tokio-Runtime (tokio::sync::Mutex ok)
