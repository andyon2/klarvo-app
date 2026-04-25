# ADR-0006: AudioSource-Trait-Signatur (Push-via-Broadcast-Publish)

**Status:** Accepted
**Date:** 2026-04-19

## Context

Epic 2 (End-to-End Dictation Pipeline, FR12-17 + FR29) führt den Audio-Capture-Pfad ein: Hotkey → `AudioSource`-Trait-Impl (cpal-based auf Windows) → VAD + Pipeline-Executor → STT-Plugin → Cleanup → Shell-Delivery.

Architecture.md §Audio-Pipeline-Abstraktion (Zeile 319-320) mandatiert:
- **Trait-Location:** `AudioSource`-Trait im Core (`klarvo-core/src/audio/source.rs` per Directory-Structure, Zeile 903+); Implementations in Shells (cpal-Win, AudioRecord-Android).
- **Event-Flow:** `tokio::sync::broadcast`-Channels im Core für `AudioEvent`-Enum (Samples, VAD-State, Level). Bridge-Layer serialisiert nach außen (Tauri-Channel auf Win, JNI-Callback auf Android).

Die Trait-Signatur selbst ist in architecture.md nicht festgelegt — das ist der Decision-Space, den dieses ADR schließt.

Rahmenbedingungen:
- **NFR2 (PRD Zeile 736):** „Audio-Capture-Thread droppt keine Samples während Hold-to-Talk, unabhängig von Downstream-Processing-Latency." — Producer-Side-Guarantee.
- **NFR3 (PRD Zeile 737):** `ts_ms` session-relative monotone Caller-Clock (ref `memory/project_event_ts_ms_convention`).
- **ADR-0001 Precedent:** VadProvider-Signatur ist `async fn process(&mut self, samples: &[f32], ts_ms: u64) -> Result<VadDecision, PluginError>`. VAD ist Consumer des Sample-Streams, AudioSource ist Producer.
- **Executor-Entry (per `memory/project_executor_stage_data_shape`):** `run_pipeline(&manifest, &registry, input: StageData) -> Result<StageData, AppError>` nimmt `StageData::Audio(AudioBuffer)` als Initial-Input für STT-Stage. AudioSource produziert den Buffer, der als `StageData::Audio` in die Pipeline geht (via Shell-Capture-Loop, nicht direkt — siehe Consequences).

Decision-Space enumerate:
- **(a) Sync-Pull:** `fn next_buffer(&mut self) -> Result<Vec<f32>, AudioError>` — blocking
- **(b) Async-Pull:** `async fn next_buffer(&mut self) -> Result<Vec<f32>, AudioError>` — tokio-integriert
- **(c) Push-via-Broadcast-Publish:** AudioSource bekommt `broadcast::Sender<AudioEvent>` im Constructor injected, publisht autonom in dediziertem Thread
- **(d) Callback-based:** AudioSource nimmt `impl Fn(&[f32])` im Constructor

## Decision

**Gewählt: (c) Push-via-Broadcast-Publish.** AudioSource-Impl owned den Publish-Side-Handle und emittiert `AudioEvent`-Varianten autonom während einer laufenden Capture-Session.

**Trait-Signatur (in `klarvo-core/src/audio/source.rs`, re-exported via `klarvo_core::traits::AudioSource`):**

```rust
#[async_trait]
pub trait AudioSource: Send + 'static {
    /// Start an audio-capture session. The returned `CaptureHandle` owns
    /// the capture-side resources (OS-audio-stream, capture-thread) and
    /// stops capture on drop. Samples and level events are published to
    /// the broadcast-channel provided via `config.events`.
    ///
    /// `config.sample_rate` and `config.channels` are advisory — the
    /// implementation SHOULD resample/downmix to 16 kHz mono f32 before
    /// emitting `AudioEvent::Samples` (Whisper-standard). Implementations
    /// that cannot honor the advisory return `AudioError::UnsupportedFormat`.
    ///
    /// `ts_ms` on emitted events is caller-provided session-relative
    /// monotone (ref ADR-0001, memory/project_event_ts_ms_convention). The
    /// AudioSource-impl holds one `Instant::now()` captured at start and
    /// derives `ts_ms` from `elapsed().as_millis()` for each emitted chunk.
    async fn start(
        &mut self,
        config: CaptureConfig,
    ) -> Result<CaptureHandle, AudioError>;
}

pub struct CaptureConfig {
    /// Advisory sample-rate. Impls resample to 16 kHz if possible.
    pub sample_rate: u32,
    /// Advisory channel-count. Impls downmix to mono if possible.
    pub channels: u16,
    /// Broadcast-sender; AudioSource publishes AudioEvent variants here.
    pub events: tokio::sync::broadcast::Sender<AudioEvent>,
}

/// Drop-guard that stops the capture-session and releases OS resources.
pub struct CaptureHandle { /* opaque */ }

#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// PCM mono f32 samples at 16 kHz (Whisper-standard).
    Samples { data: Arc<[f32]>, ts_ms: u64 },
    /// RMS level for UI meter (0.0..=1.0).
    Level { rms: f32, ts_ms: u64 },
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError { /* variants deferred to impl-story */ }
```

### Sub-Decisions

**1. Event-Enum-Scope (`AudioEvent::Samples` + `Level`, nicht VadState).**
`VadDecision` (ADR-0001) wird NICHT auf dem AudioSource-Broadcast emittiert. VAD ist Consumer des Sample-Streams, kein Producer. Die Bridge- und Shell-Layer können VadDecision auf einem separaten Channel oder als Pipeline-Side-Effect propagieren — separates Epic-6/Epic-2-Concern, außerhalb AudioSource-Trait-Surface.

**2. Sample-Format fixed (16 kHz mono f32), nicht configurable.**
Whisper-Cloud (Groq) und alle Phase-2-STT-Plugins (OpenAI Whisper, DeepSeek-Speech) erwarten 16 kHz mono. Configurable Sample-Rate im Trait würde jeden Consumer zwingen, Rate-Awareness zu implementieren (Anti-Pattern analog ADR-0001 §Resolved-Q1). `CaptureConfig.sample_rate`/`.channels` bleiben als advisory Felder für Impl-Flexibilität (z. B. wenn Hardware nur 44.1/48 kHz kann und Impl intern resamplet), aber Emitted-Event-Format ist fixed.

**3. `Arc<[f32]>` statt `Vec<f32>` für Samples.**
`tokio::sync::broadcast` cloned Messages pro Consumer. `Vec<f32>`-Clone kopiert den gesamten Buffer (bei Chunk-Size 1024 Samples = 4 KB pro Event, bei Multi-Consumer-Fan-out N×4 KB). `Arc<[f32]>`-Clone ist ein Refcount-Bump. Referenz-Pattern analog zu Phase-0-JNI-Spike (ADR-0003) — Data-Plane mit Arc-basiertem Sample-Sharing.

**4. `CaptureHandle`-RAII-Drop statt expliziter `stop()`-Methode.**
Drop-on-HandleDropped ist panic-safe (per `memory/feedback_test_raii_cleanup_pattern`-Precedent) und eliminiert State-Maschine `started`/`stopped` aus Trait-Surface. Shell-Shutdown-Path droppt Handle, Capture-Thread terminiert sauber. Consumer-Seiten sehen `RecvError::Closed` auf Broadcast und können geordnet beenden.

**5. `&mut self` statt `&self`.**
Analog zu ADR-0001 §Resolved-Q5: Verhindert paralleles Starten einer einzelnen AudioSource-Instance. Multi-Session = mehrere Instances. Windows-cpal-Impl hält interne State-Machine (Stream-Objekt, Thread-Handle), `&mut self` macht Borrow-Checker zum Compile-Time-Guard.

**6. Trait-Object-Compatibility NOT required.**
Anders als `SttProvider`/`CleanupStyle`/`VadProvider`/`PipelineStage` (4-Trait-Data-Flow-Stability-Ring, boxed in PluginRegistry) ist AudioSource **Infrastructure-Category** (analog `KeyStore` aus Epic 1C per `memory/project_keystore_trait_surface`). Consumer ist genau EIN Shell-Binary pro Target (Windows-Shell nutzt cpal-Impl, Android-Shell nutzt AudioRecord-Impl — nie beide im selben Binary). `#[async_trait]` bleibt für API-Konsistenz mit anderen Traits und um Impl-Flexibilität (Async-Init möglich), aber `Box<dyn AudioSource>` ist nicht Trait-Invariant.

## Alternatives Considered

**(a) Sync-Pull (`fn next_buffer -> Result<Vec<f32>, _>`).**
Rejected: Caller muss Blocking-Pull-Loop in dediziertem Thread fahren → verliert tokio-Integration, macht VAD/STT-Pipeline-Composition umständlicher. Erzwingt außerdem einen Downstream-Buffer irgendwo (Capture-Thread → Producer-Thread-Bridge), was NFR2 indirekt verletzt (Downstream-Buffer-Overflow = Sample-Drop am Capture-Thread-Boundary).

**(b) Async-Pull (`async fn next_buffer -> Result<Vec<f32>, _>`).**
Rejected: `tokio::sync::broadcast` in architecture.md:320 ist explizit mandatiert. Async-Pull würde den Broadcast-Channel obsolete machen — Caller pulled direkt vom AudioSource, kein Multi-Consumer-Fan-out mehr möglich (VAD + WAV-Recorder + UI-Meter + STT-Stage wollen alle denselben Sample-Stream). Bridge-Layer (JNI-Callback, Tauri-Channel) erwartet Broadcast-Semantik für Event-Fan-out.

**(d) Callback-based (`Fn(&[f32])`).**
Rejected: Single-Consumer by construction, kein Multi-Fan-out ohne manuelles Callback-Sharding im Impl. Außerdem konfligiert Sync-Closure mit Async-Consumer-Chains (Closures können nicht `.await`en in generischem Context ohne `Box<dyn Future>` zu constructen).

## Consequences

**Positiv:**
- Architecture-conformant zu architecture.md:319-320 (broadcast-Channel explizit mandatiert).
- NFR2 natively erfüllbar: Broadcast-Channel-Producer blockiert nur wenn ALLE Consumer lagged sind (und bei `send`-Fehler kann Producer die Sample-Emission weiter betreiben, Capture-Thread droppt niemals Samples — siehe ADR-0007 für Backpressure-Details).
- Multi-Consumer-Fan-out built-in: VAD-Stage, UI-Level-Meter, optionaler WAV-Recorder, STT-Chunk-Buffer können parallel auf denselben Event-Stream lauschen.
- `Arc<[f32]>` Zero-Copy-Fan-out: N Consumer × 1 Sample-Chunk = 1 Allocation + N Refcount-Bumps.
- `CaptureHandle`-RAII eliminiert Lifecycle-State-Maschine aus Trait-Surface.

**Negativ / akzeptierte Schulden:**
- Multi-Consumer-Fan-out macht Pipeline-Entry nicht-trivial: der E2E-Dictation-Flow in Epic 2 Story 2.4 muss einen Consumer-Task spawnen, der Samples aus dem Broadcast-Channel in einen `StageData::Audio(AudioBuffer)` aggregiert bevor `run_pipeline` called wird. Das ist **nicht** im AudioSource-Trait repräsentiert, sondern Shell-Wiring-Responsibility. Forward-Ref zu `memory/project_executor_stage_data_shape` — AudioBuffer-Aggregation ist Pipeline-Entry-Adapter, kein Executor-Internal.
- `tokio::sync::broadcast` Consumer-Lag-Semantik ist per-Consumer: ein langsamer Consumer verpasst Samples ohne den Producer zu blockieren. Für STT-Pipeline ist das kritisch — Details und Policy in ADR-0007.
- `CaptureConfig.events`-Injection bindet AudioSource-Construct an Channel-Ownership-Pattern; Shell muss Sender **vor** AudioSource-Init constructen. Akzeptiert als Pattern.
- Trait is not object-safe via `Box<dyn AudioSource>` in Registry-Style Collections (analog `KeyStore`). Impls werden per `cfg(target_os)` selektiert, nicht per Registry-Lookup.

**Epic-2-Story-Impacts:**
- **Story 2.1 (Windows-Hotkey + Audio-Capture-Wire-up):** Konstruiert cpal-based `AudioSource`-Impl, injected `broadcast::Sender<AudioEvent>`.
- **Story 2.2 (VAD + Pipeline-Entry-Aggregation):** Consumer des Broadcast-Channels; aggregiert Samples zu `AudioBuffer` für `run_pipeline`-Initial-Input.
- **Story 2.4 (End-to-End Headless Flow):** Integration-Test konstruiert eine Test-AudioSource-Impl (spielt WAV-Fixture ab) → emittiert auf Test-Broadcast-Channel → E2E-Pipeline läuft.

**Forward-References Phase 2+:**
- Android-Shell-Impl (Phase 3) liefert `AudioRecordAudioSource` in `shells/android/` — gleiche Trait, Platform-specific Impl.
- Configurable Output-Format (44.1 kHz, stereo Samples für zukünftige Offline-Whisper mit Noise-Reduction-Pre-Processing) wäre Additive-Field-Extension in `CaptureConfig`, ohne Trait-Signatur-Break.

## Open Questions (for Andy-review)

- **Q1:** `AudioEvent::Samples { data: Arc<[f32]>, ts_ms: u64 }` — soll `ts_ms` den **Start** des Chunks beschreiben oder **End**? ADR-0001 ließ das für `SpeechStart`/`SpeechEnd` explizit dokumentiert, aber `process()`-Parameter-ts_ms war semantisch „now". Vorschlag: Chunk-Start-ts_ms (gleiche Semantik wie AudioLevel-Producer in JNI-Spike `klarvo-bridge-jni/src/commands.rs:66,70`). Reviewer kann in Amendment fixieren.
- **Q2:** Default-Chunk-Size (samples-pro-Event) ist nicht in `CaptureConfig` — sollte er? Windows-cpal-Impl kann Buffer-Size nicht beliebig wählen (OS-Audio-Driver-Constraint), Android-AudioRecord kann genauer konfigurieren. Vorschlag: Impl-internal, nicht im Trait-Surface. Reviewer kann in Amendment entscheiden.

## Cross-References

- `output/planning-artifacts/architecture.md` §Audio-Pipeline-Abstraktion Zeile 319-320, §Project-Directory-Structure Zeile 903+
- `output/planning-artifacts/prd.md` FR12-17 + NFR2 + NFR3
- ADR-0001 (VadProvider-Trait-Signatur — Precedent für ts_ms-Semantik + `&mut self`)
- ADR-0003 (JNI-Dual-Surface — Precedent für Arc-basiertes Sample-Sharing im Data-Plane)
- ADR-0007 (Audio-Buffer-Backpressure-Policy — determiniert Broadcast-Lag-Tolerance-Semantik)
- `memory/project_event_ts_ms_convention` (ts_ms-Semantik Core-weit)
- `memory/project_executor_stage_data_shape` (Pipeline-Entry-Signature `run_pipeline(..., StageData)`)
- `memory/project_keystore_trait_surface` (Infrastructure-Trait-Precedent — AudioSource analog)

## Amendment 1 — 2026-04-19: Open Questions resolved (Status → Accepted)

**Finding:** Original ADR hat Q1 (`ts_ms` chunk-start vs chunk-end) und Q2 (Default-Chunk-Size-Location) explizit als „for Andy-review" offen gelassen. Amendment resolved beide und flipt Status Proposed → Accepted.

**Resolution Q1 — `ts_ms` auf `AudioEvent::Samples` = Chunk-START.**

Präzedenz JNI-Spike (`klarvo-bridge-jni/src/commands.rs:66,70`, ref ADR-0003 + `memory/project_event_ts_ms_convention`) ist konsistent: AudioLevel-Producer emittiert `ts_ms` als Event-Emission-Zeitpunkt = Chunk-Start. Downstream-Consumer kann Chunk-End trivial via `start + (data.len() as u64 * 1000 / 16000)` rechnen.

Rustdoc-Line-Mandate für `AudioEvent::Samples.ts_ms` (zu setzen in Story 2.1 Impl):

> „timestamp of chunk START, caller-monotone ms since session-start (ref ADR-0001, memory/project_event_ts_ms_convention)"

**Resolution Q2 — Default-Chunk-Size = Impl-internal, NICHT im Trait-Surface.**

Windows-cpal kann Buffer-Size nicht frei konfigurieren (OS-Audio-Driver-Constraint); Android-AudioRecord hat etwas mehr Freiheit. Chunk-Size im Trait zu forcen würde Consumer zur Rate-Adaptation zwingen — Anti-Pattern konsistent zu Sub-Decision #2 (Sample-Format-Fixed, nicht configurable).

Pro-Impl-Rustdoc dokumentiert konkreten Wert. Beispiel-Clause für Story 2.1 (cpal-Impl):

> „emits chunks of ~1024 samples = 64 ms @ 16 kHz, subject to OS-audio-driver granularity"

Consumer (Story 2.2 STT-Aggregator) buffern via broadcast-Recv-Loop zu Pipeline-Input-Buffer beliebiger Größe.

**Policy unchanged:** Trait-Signatur, `CaptureConfig`-Shape, `CaptureHandle`-RAII, `Arc<[f32]>`-Sample-Payload, 16-kHz-mono-f32-Fixed-Emit-Format bleiben wie im Original-Decision-Block. Amendment fixiert nur `ts_ms`-Event-Level-Präzision (Q1) und Chunk-Size-Layering-Boundary (Q2).

**Consequences for downstream:**
- Story 2.1 (cpal-Impl Windows-Shell) Rustdoc verpflichtend mit obigen Clauses auf `AudioEvent::Samples.ts_ms` + Impl-Chunk-Size-Doc.
- Story 2.2 (STT-Aggregator + Pipeline-Entry) konsumiert Chunks in beliebiger OS-Granularity via `broadcast`-Recv-Loop; aggregiert selbst zu `AudioBuffer` für `run_pipeline`-Initial-Input.
- ADR-0007 256-Slot-Capacity bleibt gültig — basiert auf ~1024-Sample-Chunks (~64 ms @ 16 kHz), konsistent mit beiden Resolutionen.

**Source of finding:** Andy-Review 2026-04-19 (post-Opus-Delegate-Session, mid-Epic-2-Pre-Flight-Resolution).

## Amendment 2 — 2026-04-20: cpal-Impl location clarified (klarvo-audio-cpal/ workspace-root crate)

**Finding:** Sub-Decision 6 stated "Impls live in `shells/windows-tauri/` (Epic 3 cpal)" and Story 2.1 spec (epics.md) Rustdoc clause (e) read *"Windows-cpal-Impl (`WindowsCpalAudioSource`) is Epic 3 scope (`shells/windows-tauri/`)"*.

Story 2.5 Pre-Flight-Scope-Lock (2026-04-20) surfaced two problems with this:
1. `shells/windows/` (actual dir name) does not exist as a compilable Rust crate yet — it is Epic-3-scope. Placing Story 2.5 implementation there would be scope-creep.
2. `cpal` is a cross-platform library (WASAPI / ALSA / CoreAudio host-dispatch at runtime). `CpalAudioSource` has no inherent shell-coupling — it only depends on `cpal`, `tokio::sync::broadcast`, and `klarvo-core::audio`. Library-extraction to a standalone workspace crate is cleanly possible.

**Resolution — Sub-Decision 6: impl location updated.**

> **Windows-cpal-Impl (`CpalAudioSource`) lives in `klarvo-audio-cpal/` workspace-root crate — library-style, cross-platform-compilable (`cpal` handles host-dispatch to WASAPI/ALSA/CoreAudio at runtime). Shell-Consumer (Epic 3 `shells/windows/`) imports `klarvo-audio-cpal::CpalAudioSource` via normal crate-dep and instantiates under `#[cfg(target_os = "windows")]`.**
>
> **`klarvo-audio-cpal/` itself contains no `cfg(target_os)`-gates. The crate builds on Linux-CI: cpal is cross-platform-compilable; device-enumeration returns `None` on non-WASAPI hosts, which is expected in unit-test contexts. Unit-tests cover Sample-Rate-Math and RAII-Drop-Behavior without real hardware.**
>
> **Android-AudioRecord-Impl remains shell-coupled (`shells/android/`): JNI-Lifetimes bind it to the Android Shell binary, making library-extraction non-viable. This asymmetry is intentional — cpal is library-pattern, JNI is shell-pattern.**

**Crate-naming rationale:** `klarvo-audio-cpal/` not `klarvo-plugins/klarvo-plugin-cpal/` — the `klarvo-plugin-` prefix is Registry-Member-Convention (`SttProvider` / `CleanupStyle` / `OutputTarget`). `AudioSource` is Infrastructure-Category per Sub-Decision 6; a mismatched `plugin-` prefix would blur the category boundary for future readers.

**`cfg(target_os)` placement:** Selection stays at Shell-Consumer level (Epic 3), not inside `klarvo-audio-cpal/`. Shell-wiring pattern (Epic 3):
```rust
#[cfg(target_os = "windows")]
use klarvo_audio_cpal::CpalAudioSource;
```
No `cargo-feature platform-windows` — `cfg(target_os)` is automatic and sufficient per original Sub-Decision 6 intent.

**Struct rename: `WindowsCpalAudioSource` → `CpalAudioSource`.** `cpal` is cross-platform; the `Windows` prefix was premature and derived from the now-revised shell-location assumption. `CpalAudioSource` names the backend, not the platform.

**Story 2.1 Rustdoc clause (e) — corrigendum** (applied to `epics.md` spec in this commit):

- Original: *"Windows-cpal-Impl (`WindowsCpalAudioSource`) is Epic 3 scope (`shells/windows-tauri/`). Android-AudioRecord-Impl is Phase-3 scope."*
- Corrected: *"`CpalAudioSource` is Story-2.5-scope (`klarvo-audio-cpal/` workspace-root crate); Shell-integration (instantiation in `shells/windows/`) is Epic-3-scope. Android-AudioRecord-Impl is Phase-3 scope (`shells/android/`)."*

Since Story 2.1 code is not yet implemented (spec-only commits as of 2026-04-20), no source-file rustdoc edit is needed — the correction is in the epics.md spec that Implementers read during Story 2.1 execution.

**Policy unchanged:** Push-via-Broadcast-Publish decision, `CaptureHandle`-RAII, `Arc<[f32]>` payload, 16-kHz-mono-f32-Fixed-Emit-Format, Infrastructure-Category classification, non-PluginRegistry-membership, `&mut self`, `#[async_trait]` — all unchanged from original Decision-Block and Amendment 1.

**Consequences for downstream:**
- Story 2.5: implements `CpalAudioSource` in new `klarvo-audio-cpal/` workspace-root crate. `cargo test -p klarvo-audio-cpal` runs on Linux-CI (unit-tests for Sample-Rate-Math + RAII-Drop-Behavior, no audio devices needed).
- Epic 3 (`shells/windows/`): imports `klarvo-audio-cpal` as a crate-dep; instantiates `CpalAudioSource` under `#[cfg(target_os = "windows")]`. Shell-wiring is the only Epic-3-scope addition.
- Phase 3 (`shells/android/`): `AndroidAudioSource` (concrete API-choice TBD Phase 3) remains shell-coupled as before.

**Source of finding:** Story 2.5 Pre-Flight-Scope-Lock 2026-04-20 (Andy-Resolution + Sonnet-Scope-Analysis).
