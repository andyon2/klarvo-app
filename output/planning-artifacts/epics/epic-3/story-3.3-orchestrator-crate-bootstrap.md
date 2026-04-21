---
name: Story 3.3 — klarvo-shell-orchestrator Crate-Bootstrap
epic: 3
story_number: "3.3"
status: Draft
dependencies:
  - "3.4"
---

# Story 3.3: `klarvo-shell-orchestrator` Crate-Bootstrap

## Outcome

Neues Workspace-Member `klarvo-shell-orchestrator/` mit `SessionOrchestrator`-Struct und
vollständiger `on_press`/`on_release`-State-Machine; Tauri-frei, DI-mockable. Unit-Tests für
Happy-Path, Idempotenz-Guard, Stray-Release und Pipeline-Failure sind grün.

## Acceptance Criteria

### AC-A — Crate-Setup + Dependency-Constraint

**Given** das Workspace enthält noch kein `klarvo-shell-orchestrator`-Crate  
**When** `klarvo-shell-orchestrator/Cargo.toml` angelegt wird  
**Then**

- `Cargo.toml` listet exakt diese Dependencies (per ADR-0012 §SD-1):
  - `klarvo-core` (workspace path-dep)
  - `tokio` (workspace, features `["sync", "rt", "macros"]`)
  - `async-trait` (workspace)
  - `tracing` (workspace)
  - `thiserror` (workspace)
- `[dev-dependencies]` listet `klarvo-test-fixtures` (workspace path-dep) und
  `tokio` mit feature `"test-util"` für `#[tokio::test]`
- Das Crate enthält **keine** `tauri`-Dep und **keine** `tauri-specta`-Dep; ein CI-check
  via `cargo metadata --no-deps` o.ä. kann das verifizieren — Rustdoc-Kommentar am
  Crate-Root dokumentiert diese Invariante:
  `// No tauri/tauri-specta dependency — orchestrator is platform-agnostic (ADR-0012 SD-1).`
- Workspace-Root `Cargo.toml` `members`-List bekommt `"klarvo-shell-orchestrator"` Eintrag

### AC-B — `SessionOrchestrator`-Struct-Shape

**Given** das Crate existiert per AC-A  
**When** `klarvo-shell-orchestrator/src/lib.rs` und `src/session.rs` implementiert werden  
**Then**

- `SessionOrchestrator` hat folgende Felder (alle per `Arc` injiziert, Trait-Objects für DI):
  ```rust
  pub struct SessionOrchestrator {
      registry: Arc<PluginRegistry>,
      manifest: Arc<PipelineManifest>,
      audio_source: Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>,
      output_target_id: String,
      paste_backend: Arc<dyn PasteBackend>,
      error_emitter: Arc<dyn ErrorEmitter>,
      clock: Arc<dyn Clock>,
      vad: Arc<tokio::sync::Mutex<Box<dyn VadProvider>>>,
      session_state: Arc<tokio::sync::Mutex<SessionState>>,
  }
  ```
  `VadProvider` ist DI-Dep (analog `AudioSource`) — `run_capture_session` nimmt
  `&mut dyn VadProvider`, der Orchestrator hält den Provider für die Session-Lifetime
- `pub fn new(...)` nimmt alle obigen Deps als Args; keine Default-Impl (kein
  Default-Trait-Derive)
- `SessionOrchestrator` ist `pub` und `Send + Sync` (alle Fields sind `Arc<...>`)

### AC-C — `SessionState`-Enum + `on_press`-Happy-Path

**Given** `SessionOrchestrator::new(...)` konstruiert wurde  
**When** `on_press(&self)` aufgerufen wird während State = `Idle`  
**Then**

- `SessionState` ist deklariert als:
  ```rust
  enum SessionState {
      Idle,
      Recording {
          capture_handle: CaptureHandle,
          pipeline_task: tokio::task::JoinHandle<()>,
      },
  }
  ```
  `CaptureHandle` ref `klarvo-core/src/audio/source.rs` (per ADR-0006)
- `on_press` flow:
  1. Lock `session_state`; wenn bereits `Recording` → verwerfen (Key-Repeat-Guard, AC-D)
  2. `let (tx, rx) = tokio::sync::broadcast::channel::<AudioEvent>(AUDIO_CHANNEL_CAPACITY)`
     (Capacity-Konstante: Delegate-Choice, z.B. `64` — analog ADR-0007-Backpressure-Policy)
  3. `let capture_handle = audio_source.lock().await.start(CaptureConfig { events: tx, ... }).await?`
     auf Fehler: `error_emitter.emit_error("error.audio.start_failed", clock.now_ms()).await`;
     State bleibt Idle
  4. Spawn Pipeline-Task (AC-E)
  5. State → `Recording { capture_handle, pipeline_task }` setzen, Lock release
- `on_press` ist `pub async fn on_press(&self)` (kein Return-Value; Errors werden via
  `error_emitter` emittiert, nicht als Return-Error propagiert — fire-and-forget)

### AC-D — `on_press`-Idempotenz-Guard (Key-Repeat-Defense)

**Given** `on_press` wurde aufgerufen und State = `Recording`  
**When** `on_press` erneut aufgerufen wird (Key-Repeat, ADR-0011 §SD-3)  
**Then**

- Der zweite Call wird **verworfen**: kein neues CaptureHandle, kein neues Pipeline-Task-Spawn
- `tracing::debug!(...)` logt den Discard-Grund:
  `"on_press called while recording; discarding (key-repeat-guard)"`
- Keine Error-Emission — Key-Repeat ist kein Fehler, nur ein No-Op
- Symmetrisch: `on_release` während `Idle` (kein aktives Recording) → No-Op +
  `tracing::debug!` ohne Error-Emission

### AC-E — Pipeline-Task + `on_release`-Flow

**Given** State = `Recording { capture_handle, pipeline_task }`  
**When** `on_release(&self)` aufgerufen wird  
**Then**

- `on_release` flow:
  1. Lock `session_state`; wenn `Idle` → No-Op (AC-D-Symmetrie)
  2. Extrahiere `Recording { capture_handle, pipeline_task }` aus dem State
  3. State → `Idle` setzen, Lock release (vor dem `drop` — Lock nicht über async-Boundary halten)
  4. `drop(capture_handle)` → signalisiert AudioSource-Task zu stoppen → Broadcast-Sender
     wird beim Audio-Task-Exit geschlossen → `run_capture_session` in Pipeline-Task bekommt
     `RecvError::Closed`
  5. `on_release` returniert sofort (non-blocking); Pipeline-Task läuft asynchron weiter

- **Pipeline-Task** (spawned in `on_press`):
  ```
  let result = run_capture_session(rx, &mut *vad.lock().await, &manifest, &registry).await;
  match result {
      Ok(Some(stage_data)) => {
          // Step 6: deliver to OutputTarget
          let text = extract_text(stage_data);  // Delegate-Choice für StageData::Text-Extraktion
          if let Some(target) = registry.output(&output_target_id) {
              if let Err(e) = target.deliver(&text).await {
                  error_emitter.emit_error(&e.user_message.unwrap_or_default(), clock.now_ms()).await;
              } else {
                  // Step 7: paste
                  if let Err(e) = paste_backend.paste().await {
                      error_emitter.emit_error(&e.user_message.unwrap_or_default(), clock.now_ms()).await;
                  }
              }
          } else {
              error_emitter.emit_error("error.config.output_target_not_found", clock.now_ms()).await;
          }
      }
      Ok(None) => { /* accidental hotkey trigger — swallow silently, per orchestrator.rs doc */ }
      Err(e) => {
          error_emitter.emit_error(&e.user_message.unwrap_or_default(), clock.now_ms()).await;
      }
  }
  ```
  Neuer i18n-Key `error.config.output_target_not_found` wird in `locales/en.json` +
  `locales/de.json` registriert (analog Story 3.2 AC-G Pattern)

### AC-F — Unit-Tests in `klarvo-shell-orchestrator/tests/`

**Given** das Crate ist implementiert per AC-A bis AC-E  
**When** `cargo test -p klarvo-shell-orchestrator` ausgeführt wird  
**Then**

- **Test 1 — Happy-Path (press → release → paste received):**
  ```
  Given: MockAudioSource (synthetic 1 chunk), MockVadProvider (returns SpeechStart→SpeechEnd),
         MockSttProvider (returns "hello"), MockCleanupStyle (verbatim passthrough),
         InMemoryOutputTarget, MockPasteBackend::new(), MockErrorEmitter::new()
  When: on_press().await; wait 50ms; on_release().await; wait for pipeline_task.await
  Then: InMemoryOutputTarget.last_delivered() == Some("hello");
        MockPasteBackend.was_called() == true;
        MockErrorEmitter.recorded() is empty
  ```
- **Test 2 — Idempotent-Press (Key-Repeat-Guard):**
  ```
  When: on_press().await; on_press().await (second call during Recording)
  Then: audio_source.start() was called exactly once (single CaptureHandle);
        no error emitted
  ```
- **Test 3 — Stray-Release (release before press):**
  ```
  Given: SessionOrchestrator in Idle state (fresh)
  When: on_release().await
  Then: no panic, no error emitted, state remains Idle
  ```
- **Test 4 — Pipeline-Failure (STT returns Err):**
  ```
  Given: MockSttProvider configured to return UpstreamUnavailable error
  When: on_press().await; wait; on_release().await; wait for task
  Then: MockErrorEmitter.recorded() contains at least one entry;
        MockPasteBackend.was_called() == false;
        state == Idle (no deadlock or hang)
  ```
- Alle Tests nutzen `#[tokio::test]` und `klarvo_test_fixtures`-Mocks
- Tests laufen ohne Tauri-App-Instance, ohne echtes Mikrofon, ohne Clipboard

### AC-G — Rustdoc-Contract

**Given** die Public-API von `SessionOrchestrator`  
**When** `cargo doc -p klarvo-shell-orchestrator` läuft  
**Then**

- Crate-Level-Rustdoc (`lib.rs`-Modul-Kommentar) beschreibt die 7-Step-Topology mit
  Cross-Ref: `// 7-Step Push-to-Talk Cycle: memory/project_shell_session_lifecycle`
- `on_press` + `on_release` haben Rustdoc mit:
  - Expected pre-conditions (State-Machine-Invariant)
  - Error-paths (AppErrorKind-Variants + i18n-Key-Präfix `error.audio.*`, `error.config.*`)
  - Non-blocking-Garantie bei `on_release`
- `SessionState`-Enum-Variants dokumentieren ihre Invariante (Idle = kein aktives Recording,
  Recording = CaptureHandle + laufender Task)

## Technical Notes

### `tokio::spawn` vs. `tauri::async_runtime::spawn`

`klarvo-shell-orchestrator` ist Tauri-frei, daher plain `tokio::spawn` im Pipeline-Task.
In `shells/windows/src-tauri/` (Story 3.x Bootstrap-Integration) wird `tauri::async_runtime::spawn`
für den Hotkey-Callback-Dispatch verwendet, aber das ist Shell-Scope. Der Orchestrator selbst
spawned via `tokio::spawn` — das ist korrekt, weil der Orchestrator unter Tauri's managed
tokio-Runtime läuft (`memory/project_shell_runtime_model`: Single Tauri-managed tokio-Runtime).

### `Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>` Rationale

`tokio::sync::Mutex` (nicht `std::sync::Mutex`) ist required, weil `.start()` auf AudioSource
`async` ist (ADR-0006). `std::sync::Mutex` würde `.lock()` in einem async-Kontext erfordern,
was Deadlock-Risk bei Tokio-Runtime birgt. `Box<dyn AudioSource>` (nicht Arc) für audio_source
und vad: die Wrapper-Arc hält das Lock, inneres `Box` für Trait-Object.

### `run_capture_session` als Core-Funktion

`run_capture_session` existiert bereits in `klarvo-core/src/pipeline/orchestrator.rs`. Der
`SessionOrchestrator` ruft sie auf — er re-implementiert sie nicht. Die Funktion nimmt
`broadcast::Receiver<AudioEvent>` (vom `on_press`-Kanal), `&mut dyn VadProvider`, Manifest und
Registry. Der Orchestrator hält alle diese Deps als Constructor-Args.

### SessionOrchestrator ist `pub struct`

`SessionOrchestrator::new()` wird von `shells/windows/src-tauri/src/main.rs` (Story 3.x
Bootstrap-Integration) konstruiert. Die Struct ist `pub` aus `klarvo-shell-orchestrator`
re-exported.

### Pipeline-Task-Shutdown

`pipeline_task: tokio::task::JoinHandle<()>` im `SessionState::Recording`-Variant trägt
folgenden Rustdoc-Kommentar in der Impl:

```rust
/// Phase-2 Toggle-Mode revisit: graceful await/abort on App-Exit
/// (ADR-0012 Open-Questions §Orchestrator-Shutdown-bei-App-Exit).
```

Phase-1-Drop-on-State-Transition-Semantik ist ausreichend: tokio-Runtime-Drop cancelt
pending Tasks safely (tokio-shutdown-contract). Bei Phase-1-Hold-to-Talk ist kein
laufender Task außerhalb des 7-Step-Cycles zu erwarten. Long-Lived-Toggle-Sessions
(Phase-2-Backlog) würden den Cleanup-Pfad revisiten — daher der Marker.

### i18n-Key `error.audio.start_failed`

Neuer Key in `locales/en.json` + `locales/de.json`. Der Orchestrator-Scope registriert Keys
für seine Error-Emission-Sites: `error.audio.start_failed` (AudioSource::start Fail),
`error.config.output_target_not_found` (Registry-Lookup-Miss). Andere Keys (STT, Cleanup,
Paste) werden von ihren jeweiligen Stories registriert.

## Dependencies

- Story 3.4 (PasteBackend-Trait muss in `klarvo-core` existieren bevor Orchestrator-Crate
  kompiliert)
- ADR-0012 §SD-1 — Crate-Placement (Workspace-Member, keine Tauri-Dep)
- ADR-0012 §SD-2 — API-Surface (`on_press`/`on_release`, DI-Shape)
- ADR-0012 §SD-3 — Windows-Shell-Integration (Story 3.x Bootstrap: Orchestrator in
  `tauri::State`)
- ADR-0012 §SD-5 — Testability-Contract (Test-Doubles, headless)
- ADR-0011 §SD-3 — Key-Repeat-Guard (Idempotenz)
- `memory/project_shell_session_lifecycle` — 7-Step-Topology authoritativ
- `memory/project_shell_runtime_model` — Single tokio-Runtime, broadcast non-blocking
- `feedback_scaffold_fail_soft_pattern` — structured AppError statt `todo!()`/panic
