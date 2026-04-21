---
name: Story 3.11 — E2E-Integration-Test (Headless)
epic: 3
story_number: "3.11"
status: Draft
dependencies:
  - "3.10"
---

# Story 3.11: E2E-Integration-Test (Headless)

## Outcome

`klarvo-shell-orchestrator/tests/e2e_test.rs` exerciseert den vollständigen 7-Step Push-to-Talk
Cycle end-to-end mit realer Pipeline-Execution (`run_pipeline` + `klarvo-plugin-verbatim`) und
gemockten OS-Boundary-Layern. Tauri-frei, headless, CI-safe. Fünf Scenarios decken Happy-Path,
Multi-Cycle-Independence, Key-Repeat-Guard, Stray-Release und Pipeline-Mid-Fail-Recovery ab.

## Acceptance Criteria

### AC-A — Test-File-Location + Crate-Dependencies

**Given** Story 3.3 hat `klarvo-shell-orchestrator/` als Workspace-Member angelegt  
**When** Story 3.11 implementiert wird  
**Then**

- Test-File: `klarvo-shell-orchestrator/tests/e2e_test.rs` (neues Integration-Test-File neben
  den in Story 3.3 angelegten Unit-Tests)
- `klarvo-shell-orchestrator/Cargo.toml` `[dev-dependencies]` bekommt, falls noch nicht vorhanden:
  - `klarvo-plugin-verbatim` (Workspace-Path-Dep, für Real-Plugin-Instantiation)
  - `tokio` mit `features = ["test-util", "time"]` (falls nur `"test-util"` aus Story 3.3: `"time"` für
    `tokio::time::timeout` hinzufügen)
  - `klarvo-test-fixtures` (bereits Story-3.3-Dep)
- Delegate verifiziert den aktuellen `[dev-dependencies]`-Stand in `klarvo-shell-orchestrator/Cargo.toml`
  und fügt ausschließlich die fehlenden Einträge hinzu (keine Duplikate)
- Scope-Fence: nur `Cargo.toml` (additive `[dev-dependencies]`-Ergänzung) und `tests/e2e_test.rs`
  werden in dieser Story geändert; kein Eingriff in `src/`

### AC-B — Test-Setup-Helper `make_test_orchestrator_real_pipeline`

**Given** `tests/e2e_test.rs` existiert per AC-A  
**When** der Helper implementiert wird  
**Then**

- Helper-Signatur:
  ```rust
  fn make_test_orchestrator_real_pipeline() -> SessionOrchestrator
  ```
- Der Helper konstruiert folgende Kombination aus Real- und Mock-Dependencies:
  | Dep | Art | Begründung |
  |-----|-----|------------|
  | `PluginRegistry` + `klarvo-plugin-verbatim` | **Real** | Exerciseert echte Cleanup-Stage (passthrough-semantics validiert) |
  | `PipelineManifest` (STT+Cleanup, minimal) | **Real** | `parse_from_str` mit valider TOML |
  | `RmsVad` | **Real** | Phase-1-Default; `MockVadProvider` würde nur unsere Mock-Logic testen |
  | `SystemClock` | **Real** | ts_ms-Derivation nicht Mock-abhängig machen |
  | `MockAudioSource` | **Mock** | liefert synthetic Audio-Chunks (kein echtes Mikrofon) |
  | `MockSttProvider` | **Mock** | returniert fixierten Text; echtes STT = Cloud-Call = teuer + flaky |
  | `InMemoryOutputTarget` | **Real Fixture** | verifizierbar ohne Side-Effects |
  | `MockPasteBackend` | **Mock** | kein echtes Clipboard, OS-frei |
  | `MockErrorEmitter` | **Mock** | aufgezeichnete Errors inspizierbar |

- Manifest-TOML für den Helper (2-Stage STT+Cleanup):
  ```rust
  let manifest_toml = r#"
  schema_version = 1

  [[pipeline.stages]]
  type = "stt"
  plugin_id = "mock-stt"

  [[pipeline.stages]]
  type = "cleanup"
  plugin_id = "verbatim"
  "#;
  let manifest = Arc::new(klarvo_core::manifest::parse_from_str(manifest_toml)
      .expect("e2e test manifest must parse"));
  ```
- Registry-Construction:
  ```rust
  let mut registry = klarvo_core::registry::bootstrap();
  klarvo_plugin_verbatim::register(&mut registry);  // Real verbatim cleanup
  registry.register_stt("mock-stt", Arc::new(MockSttProvider::returning("hello world")));
  let registry = Arc::new(registry);
  ```
  `MockSttProvider::returning("hello world")` liefert bei jedem Aufruf `Ok("hello world".to_string())`.
  Delegate verifiziert die exakte `MockSttProvider`-Constructor-API aus `klarvo-test-fixtures/src/stt_provider.rs`.

- Rustdoc am Helper erklärt die Mock/Real-Entscheidung pro Dep:
  ```
  /// Constructs SessionOrchestrator with real pipeline (verbatim plugin) and mocked
  /// OS boundaries (Audio, STT, Paste, ErrorEmitter). Real VAD (RmsVad) and Clock
  /// (SystemClock) validate production-path behavior.
  ```

### AC-C — Scenario-1: Happy-Path End-to-End

**Given** `orchestrator` per AC-B  
**When** vollständiger Hotkey-Cycle ausgeführt wird  
**Then**

```rust
#[tokio::test]
async fn e2e_happy_path_delivers_and_pastes() {
    let (orch, output_target, paste_backend, error_emitter) =
        make_test_orchestrator_real_pipeline_with_handles();
    // ^ variant that returns handles to inspect outputs

    orch.on_press().await;
    // MockAudioSource emits synthetic loud-enough samples → RmsVad triggers SpeechStart
    // Wait for pipeline to complete:
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if output_target.last_delivered().is_some() { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("pipeline must complete within 5 seconds");
    orch.on_release().await;

    assert_eq!(output_target.last_delivered().as_deref(), Some("hello world"));
    assert_eq!(paste_backend.call_count(), 1);
    assert!(error_emitter.recorded().is_empty());
}
```

Alternativ: `on_release()` triggert den Pipeline-Start via Channel-Close (Closed-mid-Speech-Path
in `run_capture_session`); Delegate wählt den Test-Flow der mit dem `MockAudioSource`-Design
kompatibel ist. Wichtig: `pipeline_task` muss awaitable sein oder der Test pollt auf
`InMemoryOutputTarget`.

### AC-D — Scenario-2: Multi-Cycle-Independence

**Given** `orchestrator` per AC-B  
**When** zwei vollständige Press/Release-Cycles sequentiell ausgeführt werden  
**Then**

```rust
#[tokio::test]
async fn e2e_two_cycles_are_independent() {
    let (orch, output_target, paste_backend, error_emitter) = make_test_orchestrator_...();

    // Cycle 1
    orch.on_press().await;
    orch.on_release().await;
    wait_for_delivery(&output_target).await;

    // Cycle 2
    orch.on_press().await;
    orch.on_release().await;
    wait_for_delivery_count(&output_target, 2).await;

    assert_eq!(output_target.delivery_count(), 2);
    assert_eq!(paste_backend.call_count(), 2);
    assert!(error_emitter.recorded().is_empty(), "no errors across two cycles");
}
```

Assertions: 2 Deliveries, 2 Paste-Calls, kein State-Leak zwischen Cycles (d. h. zweiter Cycle
erzeugt einen unabhängigen `StageData::Text`-Output, nicht einen an den ersten appended).

### AC-E — Scenario-3: Key-Repeat-Guard

**Given** `orchestrator` per AC-B  
**When** `on_press` zweimal aufgerufen wird während bereits Recording  
**Then**

```rust
#[tokio::test]
async fn e2e_key_repeat_guard_prevents_double_start() {
    let (orch, output_target, paste_backend, error_emitter) = make_test_orchestrator_...();

    orch.on_press().await;
    orch.on_press().await;  // second press while Recording — must be discarded
    orch.on_release().await;
    wait_for_delivery(&output_target).await;

    // Exactly one CaptureHandle constructed, one delivery, no error
    assert_eq!(output_target.delivery_count(), 1);
    assert_eq!(paste_backend.call_count(), 1);
    assert!(error_emitter.recorded().is_empty());
}
```

`MockAudioSource.start_call_count()` kann als zusätzliche Assertion genutzt werden falls
`MockAudioSource` diesen Counter exposed (Delegate-Verification aus `klarvo-test-fixtures/src/audio_source.rs`).

### AC-F — Scenario-4: Stray-Release

**Given** frischer `orchestrator` im Idle-State (kein Press vorher)  
**When** `on_release` aufgerufen wird  
**Then**

```rust
#[tokio::test]
async fn e2e_stray_release_is_noop() {
    let (orch, output_target, paste_backend, error_emitter) = make_test_orchestrator_...();

    orch.on_release().await;  // release without prior press

    assert!(output_target.last_delivered().is_none());
    assert_eq!(paste_backend.call_count(), 0);
    assert!(error_emitter.recorded().is_empty(), "stray release must not emit errors");
    // no panic, no hang — test completes normally
}
```

### AC-G — Scenario-5: Pipeline-Mid-Fail + Recovery

**Given** `orchestrator` per AC-B, aber `MockSttProvider` returniert `Err` beim ersten Call  
**When** Cycle-1 (fail) und Cycle-2 (success) ausgeführt werden  
**Then**

```rust
#[tokio::test]
async fn e2e_pipeline_fail_in_cycle1_does_not_prevent_cycle2() {
    // MockSttProvider: first call returns Err(UpstreamUnavailable), second returns Ok("hello")
    let (orch, output_target, paste_backend, error_emitter) =
        make_test_orchestrator_with_queued_stt([
            Err(AppError { kind: AppErrorKind::UpstreamUnavailable, ... }),
            Ok("hello".to_string()),
        ]);

    // Cycle 1 — will fail
    orch.on_press().await;
    orch.on_release().await;
    wait_for_error_or_timeout(&error_emitter, Duration::from_secs(3)).await;

    assert!(!error_emitter.recorded().is_empty(), "cycle-1 must emit at least one error");
    assert_eq!(paste_backend.call_count(), 0, "cycle-1 must not paste");

    // Cycle 2 — must succeed (Orchestrator recovered to Idle after cycle-1)
    orch.on_press().await;
    orch.on_release().await;
    wait_for_delivery(&output_target).await;

    assert_eq!(output_target.delivery_count(), 1, "cycle-2 delivers");
    assert_eq!(paste_backend.call_count(), 1, "cycle-2 pastes");
}
```

`QueuedMockSttProvider` oder eine Variante von `MockSttProvider` die sequentielle Responses
aus einer Queue liefert (analog `klarvo_test_fixtures::QueuedMockSttProvider` in
`klarvo-test-fixtures/src/stt.rs`). Delegate verifiziert API-Shape und nutzt den existenten
`QueuedMockSttProvider` wenn kompatibel.

### AC-H — Test-Conventions + Timeout-Guards

**Given** alle Tests in `tests/e2e_test.rs`  
**When** `cargo test -p klarvo-shell-orchestrator` ausgeführt wird  
**Then**

- Alle Tests sind `#[tokio::test]` + `async`
- Alle Tests laufen **ohne** Tauri-App-Runtime — `klarvo-shell-orchestrator` hat keine Tauri-Dep
  (ADR-0012 SD-1); das ist der Load-bearing-Value dieser Test-Suite: Orchestrator-Behavior
  headless verifizierbar
- Jeder Test hat einen Timeout-Guard via `tokio::time::timeout(Duration::from_secs(5), ...)`:
  - Verhindert CI-Hang wenn Pipeline-Task ein Deadlock hat
  - Gibt bei Timeout `"pipeline must complete within 5s"` als Test-Fail-Message
- Helper `wait_for_delivery(target: &InMemoryOutputTarget)` ist ein lokaler Polling-Helper
  in `tests/e2e_test.rs`:
  ```rust
  async fn wait_for_delivery(target: &InMemoryOutputTarget) {
      tokio::time::timeout(Duration::from_secs(5), async {
          loop {
              if target.delivery_count() > 0 { return; }
              tokio::time::sleep(Duration::from_millis(10)).await;
          }
      }).await.expect("delivery must occur within 5 seconds");
  }
  ```
- Tests laufen ohne echtes Mikrofon, ohne Clipboard, ohne OS-Credential-Manager

### AC-I — Scope-Fence

**Given** Story 3.11 ist implementiert  
**When** `git diff HEAD --stat` geprüft wird  
**Then**

- Exakt geänderte Files:
  - `klarvo-shell-orchestrator/tests/e2e_test.rs` (neu)
  - `klarvo-shell-orchestrator/Cargo.toml` (nur `[dev-dependencies]`-Additions, falls nötig)
- Kein Eingriff in:
  - `klarvo-shell-orchestrator/src/` (kein Source-Code-Change)
  - andere Crates (`klarvo-core`, `shells/`, `klarvo-test-fixtures`, `klarvo-plugins/`)
- Tests sind additiv — kein existierender Test wird geändert oder gelöscht

## Technical Notes

### Warum Real-Pipeline + Mock-OS-Boundary-Split

Echtes STT + Real-Network wäre (a) kostenintensiv (jeder CI-Run erzeugt Groq-API-Kosten),
(b) flaky (Netz-Timeouts, Rate-Limits), (c) langsam (Cloud-Round-Trip > 1 s). OS-Boundary-Mock
(Audio/Paste) ist die pragmatische CI-taugliche Grenze. Der Payload-Pfad (Audio → STT-Aggregator
→ Verbatim-Cleanup → Output-Target → Paste) ist trotzdem vollständig covered durch:
`MockAudioSource → RmsVad → run_capture_session → (MockStt) → verbatim-Cleanup → InMemoryOutputTarget → MockPasteBackend`.

### Warum `RmsVad` statt `MockVadProvider`

`MockVadProvider` erlaubt vollständige Kontrolle der VAD-Decisions, ist aber eine Lüge über
die echte Signalverarbeitung. `RmsVad` mit synthetic-loud-Audio testet den realen Signalpfad
(Energy-Threshold-Detection), der in Production läuft. Wenn `RmsVad` im E2E-Test nicht
triggert, liegt das an einem echten Bug im VAD-Verhalten — ein `MockVadProvider` würde
denselben Bug verstecken.

### Warum separates `tests/e2e_test.rs` statt Erweiterung von Story-3.3-Tests

Story-3.3-Unit-Tests haben vollständig gemockte Pipelines (`MockSttProvider` + `MockCleanupStyle`).
Ihr Scope ist Orchestrator-State-Machine-Correctness (Key-Repeat-Guard, Stray-Release, Error-Paths).
Story-3.11-E2E-Scope ist Pipeline-Execution-Correctness (reale Verbatim-Plugin-Integration,
reale VAD-Integration). Separation ermöglicht klare Failure-Diagnose:
- Test in Story-3.3 schlägt fehl → Orchestrator-Bug
- Test in Story-3.11 schlägt fehl → Pipeline-Integration-Bug

### `make_test_orchestrator_*_with_handles` Varianten

Der Helper aus AC-B kann als eine Funktion mit einem Return-Tuple implementiert werden:
```rust
fn make_test_orchestrator_real_pipeline_with_handles() -> (
    SessionOrchestrator,
    Arc<InMemoryOutputTarget>,
    Arc<MockPasteBackend>,
    Arc<MockErrorEmitter>,
) { ... }
```

Für Scenario-5 (AC-G) braucht es eine Variante die `QueuedMockSttProvider` akzeptiert:
```rust
fn make_test_orchestrator_with_queued_stt(
    responses: impl IntoIterator<Item = Result<String, AppError>>
) -> (SessionOrchestrator, Arc<InMemoryOutputTarget>, Arc<MockPasteBackend>, Arc<MockErrorEmitter>)
{ ... }
```

Beide Helpers sind lokal in `tests/e2e_test.rs` definiert (nicht in `klarvo-test-fixtures` —
kein zweiter Consumer, `feedback_premature_abstraction_guard`).

### Phase-2+-Extensions (kein Scope)

Folgende Items sind bewusst aus Story-3.11-Scope ausgeschlossen:
- **Latenz-Assertions** (Pipeline < N ms): erfordert FakeClock-Timestamps + präzise Timing-Control
- **Concurrent-Hotkey-Scenarios** (zwei parallele Cycles): Phase-2-Multi-Session-Scope (ADR-0012)
- **Real-STT-Integration** (Groq-Whisper): flaky + costly, kein CI-Place

Diese Extensions sind Phase-2-Story-Kandidaten wenn entsprechende Test-Infrastructure vorhanden.

## Dependencies

- Story 3.10 (implizit vollständige Orchestrator-Wire-Up für valides Test-Setup; alle 3.1–3.9
  müssen implementiert sein damit der Orchestrator korrekt konstruierbar ist)
- Story 3.3 — `SessionOrchestrator` in `klarvo-shell-orchestrator/src/` (Impl)
- `klarvo-plugin-verbatim` — `register()` + `Verbatim`-Impl
- `klarvo-test-fixtures` — `MockAudioSource`, `MockSttProvider`, `QueuedMockSttProvider`,
  `InMemoryOutputTarget`, `MockPasteBackend` (Story-3.3/3.4-Scope), `MockErrorEmitter`, `FakeClock`
- ADR-0012 §SD-5 — Testability-Contract (headless, Mock-Dependencies)
- `memory/project_shell_session_lifecycle` — 7-Step-Topology (Test-Scenarios spiegeln Steps)
