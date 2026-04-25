# ADR-0012: Orchestrator-Owner für 7-Step Push-to-Talk-Cycle

**Status:** Accepted
**Date:** 2026-04-21

## Context

`memory/project_shell_session_lifecycle` definiert für Epic-3 eine **7-Step per-Hotkey-Cycle Topology**: Press → `broadcast::channel` → `CpalAudioSource::start` → spawn STT-Aggregator → Release → `run_pipeline` → `output.deliver` → `Ctrl+V` via `SendInput` → handle-Drop post-deliver. Shared-Long-Lived-Objekte (`PluginRegistry`, `KeyStore`, `Manifest`, i18n-Tables) bleiben Bootstrap-once; nur Capture, Channel und Aggregator sind per-Cycle.

Offene Frage: **wer hält diese State-Machine?** Gegeben (a) die State-Transitions non-trivial sind, (b) mehrere Error-Pfade existieren (ADR-0009 Shell-Error-Bridge, ADR-0010 AppErrorKind), (c) Testbarkeit explizit als Architektur-Ziel gilt (PRD-§Core-Library-Contract „Headless-Testability-Pflicht"), (d) ADR-0009 SD-3 eine intentionale Scope-Asymmetrie Core↔Shell etabliert hat, braucht es eine klare Ownership-Entscheidung vor Story-3.X-Writing.

**Decision-Drivers:**
- **Testbarkeit**: Mock-Dependencies (`MockAudioSource`, In-Memory-OutputTarget, FakeClock) müssen injizierbar sein; State-Machine soll headless ablaufen können, ohne Tauri-App-Instanz oder echten Hotkey-Press.
- **Shell-Asymmetrie-Konformität**: ADR-0009 hat bereits festgelegt, dass Core Tauri-agnostisch bleibt (`klarvo-core` hat keine tauri-Dep). Orchestrator muss dem passend zugeschnitten sein.
- **Phase-3-Android-Wiederverwendbarkeit**: Identische 7-Step-Topology ist auf Android anwendbar (`memory/project_shell_session_lifecycle`); nur die Press-Trigger-Source (AccessibilityService statt Hotkey) und das Paste-Backend (AccessibilityService statt SendInput) ändern sich. Android-Shell nutzt keinen Tauri-Stack.
- **Maintenance-Skalierung**: Epic-3-Scope enthält mehrere Stories (Hotkey-Wire-Up, Audio-Capture-Integration, Auto-Paste, Error-Bridge-Wiring). State-Machine-Ort prägt die Testbarkeits-Ergonomie aller dieser Stories.

**Nicht entscheidungs-relevant Phase 1:**
- Multi-Session-State (per `memory/project_shell_session_lifecycle` explizit ausgeschlossen — Cycle-freshness per Hotkey-Press).
- Session-Queueing / Ratelimiting auf User-Seite (nicht Phase-1-PRD-Scope).

**Scope-Fence:** Dieser ADR entscheidet **Ownership und Crate-Placement** des Orchestrators + Impl-Trait-Shape. NICHT Scope: konkrete Story-AC-Formulierungen, Key-Repeat-Guard-Implementation-Details (ADR-0011 §Open Questions), Runtime-State-Persistence über App-Neustart hinweg (Phase-1 NFR11: neuer Hotkey-Press = neue Session, kein Persistenz-Bedarf).

## Decision

**Gewählt: Option C — neues Crate `klarvo-shell-orchestrator` (pure-Rust, Tauri-frei, mockable).**

### Sub-Decision 1: Crate-Placement

Neues Workspace-Member `klarvo-shell-orchestrator/` (analog zu `klarvo-audio-cpal/` als Platform-Impl-Crate).

```
klarvo-shell-orchestrator/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── session.rs        # 7-Step-Topology-Impl
```

Dependencies: `klarvo-core` (Traits + Types), `tokio` (broadcast + async), `async-trait`, `tracing`, `thiserror`. **Keine `tauri`-Dep**, **keine `tauri-specta`-Dep** — analog zu `klarvo-audio-cpal`.

### Sub-Decision 2: Orchestrator-API-Surface

```rust
// klarvo-shell-orchestrator/src/lib.rs (Skizze, Story-3.X-Scope)

use std::sync::Arc;
use klarvo_core::{
    audio::AudioSource,
    error_emitter::ErrorEmitter,
    output::OutputTarget,
    registry::PluginRegistry,
    pipeline::Manifest,
    clock::Clock,
};

pub struct SessionOrchestrator {
    registry: Arc<PluginRegistry>,
    manifest: Arc<Manifest>,
    audio_source: Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>,
    output_target_id: String,
    paste_backend: Arc<dyn PasteBackend>,        // neu; Phase-1-Impl in shells/windows
    error_emitter: Arc<dyn ErrorEmitter>,
    clock: Arc<dyn Clock>,
    session_state: Arc<tokio::sync::Mutex<SessionState>>,
}

impl SessionOrchestrator {
    pub async fn on_press(&self);    // 7-Step 1..3 (Press-Phase)
    pub async fn on_release(&self);  // 7-Step 4..7 (Release-Phase), non-blocking spawn
}
```

`PasteBackend` ist ein weiterer Core-Trait (neue Core-Erweiterung, Story-3.X-Scope), den die Windows-Shell via `SendInput` implementiert und die Android-Shell via AccessibilityService. Damit bleibt `SessionOrchestrator` platform-agnostisch — der FR21-Ctrl+V-Schritt (7-Step-Step-6) läuft über Dependency-Injection.

Alle Dependencies sind Trait-Objects → per Unit-Test via Mocks ersetzbar.

### Sub-Decision 3: Windows-Shell-Integration

`shells/windows/src-tauri/src/main.rs` konstruiert den Orchestrator zur Bootstrap-Zeit und hält ihn in Tauri-managed-State (`tauri::State<Arc<SessionOrchestrator>>`). Die Global-Shortcut-Plugin-Callbacks (ADR-0011 SD-2) delegieren direkt an `orchestrator.on_press()` / `on_release()` via `tauri::async_runtime::spawn`.

Konkret:
```rust
// shells/windows/src-tauri/src/main.rs
let orch = Arc::new(SessionOrchestrator::new(
    registry,
    manifest,
    Arc::new(tokio::sync::Mutex::new(Box::new(CpalAudioSource::new()))),
    "clipboard".to_string(),
    Arc::new(WinSendInputPasteBackend::new()),  // Shell-scoped Impl
    Arc::new(TauriErrorEmitter::new(app.handle().clone())),
    Arc::new(SystemClock::default()),
));
```

`WinSendInputPasteBackend` lebt in `shells/windows/src-tauri/src/paste.rs` (Shell-Scope, Win32-Dep erlaubt). Android-Äquivalent Phase-3 in `shells/android/..`.

### Sub-Decision 4: Error-Path-Integration mit ADR-0009

Orchestrator benutzt die ADR-0009-Scope-Asymmetrie 1:1:
- `ErrorEmitter`-Trait aus `klarvo-core` (via DI) für Result-Chain-Fails in `on_release` (pipeline-fail, output-deliver-fail).
- `CpalAudioSource` bekommt seinen eigenen `Arc<dyn ErrorEmitter>`-Clone für cpal-Callback-Errors (ADR-0009 SD-3 Primary-Consumer).
- Beide pointen auf dieselbe `TauriErrorEmitter`-Instanz (Shell-Scope), die `app.error`-Events emittiert.

Keine neue Error-Infrastruktur im Orchestrator — er konsumiert die ADR-0009-Pattern.

### Sub-Decision 5: Testability-Contract

Unit-Tests für `SessionOrchestrator` laufen in `klarvo-shell-orchestrator/tests/` mit:
- `klarvo_test_fixtures::MockAudioSource` (Arc-wraped)
- `klarvo_test_fixtures::InMemoryOutputTarget` (wird Phase-1 bereits existieren, ref Story 2.4)
- `klarvo_test_fixtures::MockPasteBackend` (neu, Story-3.X-Scope) — instrumentiert „wurde Paste-Call gemacht mit Text X".
- `klarvo_test_fixtures::FakeClock` (existiert Phase-0-Gate)
- Ein fake `ErrorEmitter`, der emittete Errors in `Vec<AppError>` collected.

Assertions:
- Happy-Path: `on_press` → `on_release` → MockPaste bekommt korrekten Text; ErrorEmitter-Vec ist leer.
- Mid-Session-Failure: MockAudioSource liefert `AudioError` via ErrorEmitter; Orchestrator terminiert sauber, kein Panic, kein Deadlock.
- Idempotenz: `on_press` während bereits aktiver Session → wird verworfen (ADR-0011 SD-3 Key-Repeat-Guard); `on_release` ohne aktive Session → No-Op.

## Alternatives Considered

**(A) Tauri-Command als Entry-Point, State in AppHandle.**

Fair-Argumentation:
- Minimal-Infrastruktur — keine neue Crate, kein neuer Trait (PasteBackend). State wird direkt in `tauri::State<...>` gehalten, State-Machine lebt in `shells/windows/src-tauri/src/commands.rs`.
- Zugang zu `AppHandle` erlaubt directes Event-Emission ohne DI-Detour.

Rejected:
- **Testability bricht den PRD-Core-Contract.** PRD §Headless-Testability-Pflicht: „Jede Phase-1-Story hat ein AC ‚läuft in headless integration test ohne Shell'". Eine in `shells/windows/` sitzende State-Machine ist nur mit Tauri-App-Runtime testbar — genau der Anti-Pattern, gegen den `memory/feedback_architecture_doc_authoritative` Korollar positioniert.
- **Phase-3-Android-Wiederverwendung geht verloren.** Android-Shell hat kein Tauri; 7-Step-Topology müsste in Kotlin oder separat reimplementiert werden → Code-Duplikat analog zu v1 ~2000-LOC-Duplikat (ref Product-Brief §v1-Deep-Scan). Das ist der v1-Failure-Mode, den v2 strukturell verhindert.
- **Shell-Asymmetrie-Verletzung** (ADR-0009 SD-3-Rationale): tauri-specta-Dep war dort intentional auf Shell-Scope begrenzt; eine tauri-gebundene State-Machine zieht die Grenze genau wieder ein in den Orchestrator.

**(B) Background-Task mit Channel-Receiver, Orchestrator-Struct in existing `klarvo-core` oder in `shells/windows/src-tauri/`.**

Fair-Argumentation:
- Keine neue Crate. State-Machine könnte in `klarvo-core/src/orchestrator.rs` leben oder in Shell-Crate.
- Tokio-nativ, `broadcast::Receiver` als Press/Release-Event-Source.

Rejected:
- **`klarvo-core`-Placement bricht Scope.** Core soll minimale Surface haben (PRD-FR1 — 5 Phase-1-Traits); Shell-Orchestration-Logic gehört nicht ins Core. Pivotiert die Crate-Grenze in Richtung "Core-macht-alles".
- **Shell-Placement hat dieselben Testability-Probleme wie (A).**
- **Channel-statt-Function-Call macht API awkward.** Der Hotkey-Plugin emittet bereits Press/Release-Events als Callbacks (ADR-0011 SD-2); einen zusätzlichen Broadcast-Channel dazwischen zu legen nur um "entkoppelt" zu sein, ist speculative Abstraction — kein Proven-Duplication-Signal (ref `memory/feedback_premature_abstraction_guard`). Direct-Method-Call auf Orchestrator ist klarer.
- **Background-Task-Ownership unklar:** wer spawned ihn, wer stopped ihn bei App-Exit? Zusätzliche Lifetime-Frage ohne Gegenwert.

## Consequences

**Positiv:**
- **Headless-Testability erfüllt** den PRD-Core-Contract strukturell — Unit-Tests laufen im normalen `cargo test -p klarvo-shell-orchestrator`-Flow ohne Tauri-App-Runtime.
- **Shell-Asymmetrie gewahrt** (ADR-0009-Konsistenz): Orchestrator ist Tauri-agnostisch, Tauri-spezifische Impls (`TauriErrorEmitter`, `WinSendInputPasteBackend`) leben in `shells/windows/`.
- **Phase-3-Android-Wiederverwendung:** identisches Orchestrator-Crate plus Android-spezifische Trait-Impls. Die 7-Step-Topology bleibt ein einziger Code-Pfad.
- **Klare Crate-Boundary, Precedent-konform:** `klarvo-audio-cpal` ist Platform-Impl-Crate für AudioSource; `klarvo-shell-orchestrator` ist Platform-agnostic Coordinator-Crate. Architektur-Konsistenz-gewinn.
- **Dependency-Injection zwingt explizite Dependencies** — Trait-Objects statt versteckter globaler State. Folgt `memory/feedback_premature_abstraction_guard` insofern als die Traits (`AudioSource`, `OutputTarget`, `ErrorEmitter`, `Clock`) bereits im Core existieren; `PasteBackend` ist Real-Second-Consumer-motiviert (Windows SendInput + Android AccessibilityService), also gerechtfertigte Neu-Einführung.

**Negativ / akzeptierte Schulden:**
- **Eine neue Crate mehr:** `klarvo-shell-orchestrator` erweitert den Workspace. Maintenance-Overhead ist niedrig (kleines, wohldefiniertes Scope), Präzedenz-Konsistenz mit `klarvo-audio-cpal`.
- **Neuer Core-Trait `PasteBackend`:** Trait-Surface-Wachstum im Core. Gerechtfertigt, weil zweiter Consumer (Android-Phase-3) empirisch absehbar ist; `feedback_premature_abstraction_guard` verlangt Second-Consumer-Motivation — hier explizit im Phase-Plan verankert.
- **Windows-Shell-Dependency-Wiring:** `shells/windows/src-tauri/Cargo.toml` bekommt `klarvo-shell-orchestrator` als Dep, `shells/windows/src-tauri/src/main.rs` hat einen neuen Bootstrap-Block. One-time-cost.

**Epic-3-Story-Impacts:**
- **Story 3.X (Orchestrator-Crate-Bootstrap):** Neue Story, legt `klarvo-shell-orchestrator/` an mit `SessionOrchestrator`-Struct, State-Machine-Enum, 7-Step-Happy-Path-Test.
- **Story 3.X (PasteBackend-Trait):** Core-Add `klarvo-core/src/output/paste.rs` (oder analog), Trait + `async fn paste(&self) -> Result<(), AppError>`. Test-Fixture `MockPasteBackend`.
- **Story 3.1 (Tauri-Skeleton-Bootstrap):** Zusätzlicher Bootstrap-Code, der den Orchestrator in `tauri::State<...>` hält.
- **Story 3.X (Hotkey-Wire-Up, ADR-0011):** Callback-Branches rufen `orchestrator.on_press()`/`on_release()` aus `tauri::async_runtime::spawn`.
- **Story 3.X (Audio-Capture-Integration):** `CpalAudioSource` wird in den Orchestrator injiziert; kein Shell-Direct-Call mehr.
- **Story 3.X (Auto-Paste FR21):** `WinSendInputPasteBackend` in `shells/windows/src-tauri/src/paste.rs` — Shell-lokale Impl des Core-Traits.

**Phase-3-Android-Impacts (forward-looking):**
- Android-Shell bootstrapet denselben `SessionOrchestrator` via JNI (Kotlin hält Handle-Reference), konstruiert `AndroidAudioSource` + `AccessibilityPasteBackend`. 7-Step-Topology-Code ist shared.

**Phase-2+-Impacts:**
- Zweiter Hotkey-Slot (Phase-2): der Orchestrator nimmt zusätzliche Press/Release-Calls; ggf. Slot-Identifier als Parameter. State-Machine erweitert sich, nicht restrukturiert.
- Toggle/AutoStop-Recording-Modes (Phase-2): State-Machine bekommt weitere Zustände (`Toggled-On`), gleicher Orchestrator-Ort.

## Open Questions

- **Key-Repeat-Guard-Implementation:** `AtomicBool` vs. `SessionState`-Enum. Story-3.X-Impl-Detail; beides ist mit Orchestrator-Owner-Wahl kompatibel.
- **PasteBackend-Trait-Shape:** `async fn paste(&self) -> Result<(), AppError>` reicht Phase-1; Phase-3-Android könnte `paste_with_accessibility_context(&self, ctx)` brauchen — `#[non_exhaustive]` + Trait-Additive-Extension preserviert das.
- **Orchestrator-Shutdown bei App-Exit:** aktueller Stand — Drop-semantic reicht (keine persistenten Tasks nach Hotkey-Cycle-Ende). Bei Phase-2-Long-Lived-Toggle-Sessions revisit.

## Cross-References

- `output/planning-artifacts/architecture.md` §8 Audio-Pipeline-Abstraktion, §4 Error-Shape :673-675
- `output/planning-artifacts/prd.md` FR12 (Hold-to-Talk), FR17 (Delivery), FR21 (Auto-Paste), NFR11 (Retry-Semantik), §Core-Library-Contract (Headless-Testability)
- `docs/adr/0006-audiosource-trait-signature.md` (AudioSource + CaptureHandle)
- `docs/adr/0008-shell-adapter-interface-shape.md` (Shell-Orchestrated-Post-Pipeline-Delivery — Orchestrator ruft OutputTarget + PasteBackend getrennt)
- `docs/adr/0009-shell-error-bridge-pattern.md` (Scope-Asymmetrie Core↔Shell — Orchestrator-Placement folgt demselben Prinzip)
- `docs/adr/0011-hotkey-backend.md` (ADR-0011 Callbacks dispatchen an Orchestrator)
- `memory/project_shell_session_lifecycle` (7-Step-Topology — die State-Machine, die der Orchestrator hält)
- `memory/project_shell_runtime_model` (Single-Runtime-Constraint)
- `memory/feedback_premature_abstraction_guard` (PasteBackend-Trait-Einführung via Second-Consumer-Motivation)
- `memory/feedback_architecture_doc_authoritative` (Korollar: Tauri-Core-Trennung nicht aufweichen)

## Next Actions

1. Andy review + accept → Status `Proposed` → `Accepted`.
2. Story-3.X-Orchestrator-Crate-Bootstrap.
3. Story-3.X-PasteBackend-Trait-Add (Core).
4. Story-3.1-Bootstrap-Update: Orchestrator in Tauri-managed-State.
