# ADR-0007: Audio-Buffer-Backpressure-Policy (Broadcast-Channel Lag-Tolerance)

**Status:** Accepted
**Date:** 2026-04-19

## Context

NFR2 (PRD Zeile 736): „Audio-Capture-Thread droppt keine Samples während Hold-to-Talk, unabhängig von Downstream-Processing-Latency." ist **Producer-Side-Guarantee**. Die Policy für Consumer-Side (was passiert wenn ein Consumer langsamer ist als der Producer?) ist nicht festgelegt.

Architecture.md:319-320 mandatiert `tokio::sync::broadcast`-Channels im Core für `AudioEvent`-Enum. ADR-0006 (Proposed, selbe Session) fixiert AudioSource-Trait als Push-via-Broadcast-Publish: AudioSource-Impl owned `broadcast::Sender<AudioEvent>`, Consumer subscriben via `Sender::subscribe()`.

`tokio::sync::broadcast` hat Ring-Buffer-Semantik mit fester Kapazität: Sender überschreibt älteste Messages wenn der Ring voll ist und mindestens ein Consumer nicht mitgelesen hat. Der langsame Consumer sieht beim nächsten `recv().await` einen `RecvError::Lagged(n)` mit der Anzahl übersprungener Messages. Sender blockiert NICHT — NFR2 wird intrinsisch gewahrt.

Die offene Frage ist: **welche Kapazität**, und **welche Consumer-Side-Policy** bei `Lagged(n)`?

Rahmenbedingungen:
- **NFR11 (PRD Zeile 757):** „Klarvo recovered graceful von Groq-API-Failures: User kann Hotkey erneut triggern nach Upstream-Error ohne App-Neustart." — Consumer-Lag ist funktional ähnlich (transient Degradation, kein Crash).
- **NFR10 (PRD Zeile 756):** Runtime-Failures in Pipeline-Stages → `AppError` + Log, Hotkey bleibt funktional.
- **Phase-1-Scope-Lock:** Dev-internal Walking Skeleton, Andy + 1-2 Tester. Hold-to-Talk-Sessions sind erwartungsgemäß <1 Minute; keine Hour-Long-Streams. Memory-Budget ist nicht eng.
- **AudioEvent-Payload-Shape (per ADR-0006):** `Samples { data: Arc<[f32]>, ts_ms }` + `Level { rms, ts_ms }`. `Arc<[f32]>`-Clone auf Fan-out ist Refcount-only, nicht Buffer-Copy.

Decision-Space:
- **(a) `tokio::sync::broadcast` mit Lag-Tolerance:** architecture-conformant Default.
- **(b) Bounded `tokio::sync::mpsc` per Consumer:** manuelle Fan-out-Duplication, Capture-blockiert bei full-channel → NFR2-Violation.
- **(c) SPMC-Ringbuffer (`ringbuf`-Crate):** funktional äquivalent zu (a) aber custom-Crate.
- **(d) Unbounded `tokio::sync::mpsc`:** Memory-Growth-Risk, NFR2 nur oberflächlich erfüllt (RAM wächst statt Samples zu droppen — verschiebt das Problem, löst es nicht).

## Decision

**Gewählt: (a) `tokio::sync::broadcast` mit expliziter Lag-Tolerance-Policy.** Architecture-conformant zu architecture.md:320.

### Sub-Decisions

**1. Channel-Capacity: 256 Messages (konfigurierbar via Constructor-Parameter).**

Default `256`-Slots in `broadcast::channel::<AudioEvent>(256)`. Bei Chunk-Größen von ~1024 Samples (64 ms @ 16 kHz) entspricht das ~16 s Audio-Backlog pro Consumer bevor Lag auftritt. Dictation-Pipeline-Stages sollten weit unter 16 s lagen — wenn doch, ist das Indikator für Upstream-Failure (Groq-Timeout etc.) und Graceful-Degradation greift.

**Rationale:**
- Whisper-Cloud-Call (Groq-Endpoint) amortisiert bei ~1-3 s für 5-30 s Dictation — Consumer-Lag sollte <1 s sein in Happy-Path.
- `Level`-Events (UI-Meter) werden mit höherer Frequenz emittiert als `Samples`; 256 Slots buffern trotzdem hinreichend.
- Payload-Size `Arc<[f32]>` (Refcount-Pointer, 2 × usize = 16 Bytes auf x64) + `ts_ms` (8 Bytes) + Enum-Tag: ~32 Bytes pro Slot. 256 × 32 = ~8 KB Channel-Overhead pro AudioSource-Session. Vernachlässigbar.
- Konfigurierbar via AudioSource-Constructor-Argument (Capture-Session-Config), damit Phase-2+-Tuning (lange Dictation-Sessions, Slow-Mobile-Hardware) ohne ADR-Amendment möglich ist.

**2. Consumer-Side `RecvError::Lagged(n)`-Policy: Log-and-Continue.**

Consumer (STT-Aggregator, VAD-Task, UI-Meter) implementieren `RecvError::Lagged(n)`-Handling nach Pattern:

```rust
loop {
    match events.recv().await {
        Ok(event) => process(event),
        Err(RecvError::Lagged(n)) => {
            tracing::warn!(
                target: "klarvo.audio.backpressure",
                skipped = n,
                consumer = "stt_aggregator",
                "audio event consumer lagged; skipped events"
            );
            // Continue recv loop — do NOT propagate as AppError
        }
        Err(RecvError::Closed) => break,
    }
}
```

**Rationale:**
- Lag ist Transient-Degradation, kein fatal-Error. Erneuter Hotkey-Trigger funktioniert (NFR11-Analog).
- Log-Target `klarvo.audio.backpressure` macht Lag-Events Dogfooding-observable (Rolling-File-Log per architecture.md §Telemetrie, Zeile 268).
- Skipped-Count-Field ermöglicht Phase-2-Metric-Extraction ohne Schema-Change.

**3. Consumer-Side Sample-Drop-Impact auf STT-Accuracy: Acceptable in Phase 1.**

Wenn der STT-Aggregator-Consumer lagged, verliert die Transcription Samples aus dem gelaggten Window. Phase-1-Policy: das ist akzeptabel — ein gelaggter STT-Consumer ist Indikator für systemische Probleme (CPU-Starvation, Blocking-Call in falschem Thread, etc.), nicht Normal-Operation. Log-and-Continue macht das Problem observable. Phase-2+-Mitigation (falls empirisch relevant): erhöhte Channel-Capacity oder dediziertes Pre-Buffer-Pattern für STT-Stage.

**Explizit nicht-gewählte Mitigations:**
- **Kein separater STT-Buffer-Channel:** würde STT-spezifische Backpressure-Logik im Core benötigen — Scope-Creep.
- **Kein Lag→Error-Surface (Consumer-Side Abort):** würde bei jedem Jitter Hotkey-Session töten — widerspricht NFR11.

**4. Event-Type-Mischung im Channel: Single Broadcast, nicht split-per-EventType.**

`AudioEvent::Samples` und `AudioEvent::Level` teilen sich denselben Broadcast-Channel. Alternative wäre zwei separate Channels (ein `broadcast<SamplesEvent>`, ein `broadcast<LevelEvent>`). Rejected weil:
- Caller-Code einfacher (ein Subscribe statt zwei).
- `Level`-Events emittieren selten relative zu `Samples` — Mixed-Channel hat keinen merkbaren Slot-Waste.
- Consumer pattern-matchen im `recv()`-Arm; event-type-filter ist trivial.

**5. Capacity-Over-Discharge beobachtbar via Telemetrie.**

`tracing`-Event-Target `klarvo.audio.backpressure` ist einheitlich. Shell-Logs filterbar. Phase-2-Metric-Pipeline (falls introduced) kann aggregieren.

## Alternatives Considered

**(b) Bounded `tokio::sync::mpsc` per Consumer (manual Fan-out).**
Rejected hart: Fan-out-Fanning aus Capture-Thread erfordert entweder (i) Blocking `send()` auf allen Downstream-Channels — verletzt NFR2 bei langsamem Consumer, oder (ii) `try_send()` mit Drop-on-Full — funktional äquivalent zu Broadcast-Lag aber manuell implementiert. Zusätzlich bricht mit architecture.md:320 (broadcast-Mandat).

**(c) SPMC-Ringbuffer (`ringbuf`-Crate).**
Rejected: Funktional equivalent zu (a) — same Ring-Overwrite-Semantik, aber via 3rd-Party-Crate ohne tokio-Integration. `broadcast` ist bereits tokio-Core-Dep (zero-extra-Crate-Cost). Außerdem bricht architecture.md:320-Mandat.

**(d) Unbounded `tokio::sync::mpsc`.**
Rejected: Memory-Growth-Risk bei Slow-Consumer-Szenarien (Groq-Endpoint-Hang über 30 s, Samples akkumulieren unbounded). Erfüllt NFR2 nur scheinbar — Capture-Thread droppt keine Samples, aber RAM wächst bis OOM. Problemverschiebung, keine Lösung. Außerdem bricht architecture.md:320.

## Consequences

**Positiv:**
- NFR2 strikt erfüllt auf Producer-Side: `broadcast::Sender::send()` blockiert nie (non-blocking by design). Capture-Thread droppt garantiert keine Samples.
- Architecture-conformant zu architecture.md:320 — kein Abweichungs-ADR gegenüber Architecture-Doc nötig.
- Consumer-Side Log-and-Continue ist NFR11-kompatibel (Graceful-Degradation statt App-Crash).
- Backpressure-Events via `tracing`-Target observable → Dogfooding-Regression-Detection (NFR1-Analog).
- Channel-Capacity via Constructor-Arg konfigurierbar → Phase-2+-Tuning ohne Trait-/ADR-Changes.
- `Arc<[f32]>`-Payload (per ADR-0006) macht Fan-out-Cost minimal: Refcount-Bumps statt Sample-Copy.

**Negativ / akzeptierte Schulden:**
- Ein langsamer STT-Consumer verliert Transcription-Samples silently (nur Log, kein User-facing-Error). Phase-1-Acceptable per Sub-Decision #3. Phase-2+-Revisit falls empirisch relevant.
- 256-Slot-Default ist Educated-Guess für Phase-1-Hold-to-Talk-Workloads; echte Empirie kommt aus Dogfooding. Erste Calibration-Datenpunkt-Quelle sind Dogfooding-Logs aus Epic-2-Smoke-Tests.
- `broadcast`-Ring kann mehrere Sekunden Audio-Lag erlauben bevor Consumer lagged — bei Low-Power-Mobile (Phase 3) ggf. Memory-Pressure, aber weit unter kritisch (~8 KB Channel + max 256 × Arc<[f32]>-Refs).

**Epic-2-Story-Impacts:**
- **Story 2.2 (VAD + Pipeline-Entry-Aggregation):** STT-Sample-Aggregator implementiert `RecvError::Lagged`-Handler-Pattern oben.
- **Story 2.4 (End-to-End Headless Flow):** Integration-Test kann artificiell Slow-Consumer simulieren (tokio::time::sleep in recv-Loop) + asserten, dass Lag-Event geloggt wird und Pipeline weiterläuft.

**Forward-References Phase 2+:**
- **Epic 6 (Observability):** `klarvo.audio.backpressure`-Events können in strukturiertes Metric-Pipeline exportiert werden.
- **Phase 3 (Android):** AudioRecord-Shell-Impl nutzt gleichen Broadcast-Pattern; Capacity-Tuning ggf. mobilspezifisch.
- **Phase-2-Capacity-Revisit:** Empirische Calibration aus Dogfooding-Logs; Amendment falls 256 sich als falsch erweist.

## Open Questions (for Andy-review)

- **Q1:** Soll die Channel-Capacity als zentrale Konstante (`klarvo_core::audio::DEFAULT_AUDIOEVENT_CAPACITY = 256`) oder als AudioSource-Constructor-Arg-ohne-Default dokumentiert werden? Vorschlag: beide — Konstante existiert, Constructor nimmt `usize` und der Caller picked entweder Konstante oder eigenen Wert. Reviewer kann in Amendment fixieren.
- **Q2:** Consumer-Side `Lagged`-Events: sollen sie in einem dedizierten `SessionStats`-Struct aggregiert werden (für Dogfooding-Summary am Session-Ende) oder nur via `tracing` geloggt? Phase-1-Vorschlag: nur `tracing`, SessionStats ist Epic-6-Scope. Reviewer bestätigt/verwirft.

## Cross-References

- `output/planning-artifacts/architecture.md` §Audio-Pipeline-Abstraktion Zeile 319-320, §Telemetrie Zeile 268
- `output/planning-artifacts/prd.md` NFR2 + NFR10 + NFR11
- ADR-0006 (AudioSource-Trait-Signatur — determiniert broadcast-Ownership + Arc<[f32]>-Payload)
- `memory/project_event_ts_ms_convention` (ts_ms-Semantik bleibt unberührt)
- `memory/project_no_remote_telemetry` (Lag-Telemetry ist lokal-only, kein Remote-Sink)
- `memory/feedback_architecture_doc_authoritative` (Korollar: (a) ist Conformance — das ADR dokumentiert Sub-Decisions, nicht Architektur-Abweichung)

## Amendment 1 — 2026-04-19: Open Questions resolved (Status → Accepted)

**Finding:** Original ADR hat Q1 (Capacity-Surface — Konstante vs Constructor-Arg) und Q2 (Lag-Event-Aggregation-Scope — `tracing` only vs dediziertes `SessionStats`) als „for Andy-review" offen gelassen. Amendment resolved beide und flipt Status Proposed → Accepted.

**Resolution Q1 — Capacity-Surface: Beide (Konstante + Constructor-Arg).**

```rust
// In klarvo-core/src/audio/mod.rs:
pub const DEFAULT_AUDIOEVENT_CAPACITY: usize = 256;
```

AudioSource-Constructor (oder der Shell-Caller, der den `broadcast::channel` anlegt) nimmt `capacity: usize` Parameter (keine Keyword-Args in Rust). Caller picked entweder die Konstante oder eigenen Wert. **Tests referenzieren die Konstante** statt Magic-256, damit zukünftige Capacity-Tuning in einem einzigen Resolve-Punkt landet:

```rust
let (tx, _rx) = tokio::sync::broadcast::channel::<AudioEvent>(
    klarvo_core::audio::DEFAULT_AUDIOEVENT_CAPACITY,
);
```

**Resolution Q2 — Lag-Event-Aggregation: `tracing`-only in Phase 1.**

Phase-1-Scope beschränkt Observability von Backpressure-Lag-Events auf den `klarvo.audio.backpressure`-tracing-Target. Keine strukturierten Session-End-Summaries, keine dedizierte `SessionStats`-Struct.

**Forward-Ref:** SessionStats-Aggregation-Layer ist **Epic-6-Scope** (Observability), baut auf `klarvo.audio.backpressure`-tracing-target als Upstream-Input-Stream auf. Phase-1-Stories emittieren nur das Tracing-Event; Epic 6 kann es ohne Schema-Change in aggregierte Metriken überführen.

**Policy unchanged:** Broadcast-Channel-Choice (a), 256-Slot-Default, Log-and-Continue-Handler-Pattern, Event-Type-Mischung im Channel, NFR2-Producer-Guarantee bleiben wie im Original-Decision-Block. Amendment fixiert nur Konstanten-Location (Q1) und Telemetry-Aggregation-Boundary (Q2).

**Consequences for downstream:**
- Story 2.2 (STT-Aggregator + Pipeline-Entry): Consumer-Code verwendet `DEFAULT_AUDIOEVENT_CAPACITY` statt Magic-256; Log-and-Continue-Handler-Pattern wie im Decision-Block.
- Story 2.4 (E2E Headless Flow): Integration-Test kann Capacity-Override via Constructor-Arg prüfen — z. B. Slow-Consumer-Sim mit niedriger Capacity simuliert Lag-Events deterministisch reproduzierbar.
- Epic 6 (Observability) konsumiert `klarvo.audio.backpressure`-Target als Input-Stream für SessionStats-Metric-Aggregation — kein Phase-1-Scope, aber Amendment macht den Forward-Reference explizit.

**Source of finding:** Andy-Review 2026-04-19 (post-Opus-Delegate-Session, mid-Epic-2-Pre-Flight-Resolution).
