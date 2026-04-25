# ADR-0001: VadProvider-Trait-Signatur (Gate-Events, nicht Sample-Transform)

**Status:** Accepted
**Date:** 2026-04-18

## Context

Step-4-Revision (Architecture-Doc §VAD-Split, ca. Zeile 887) hat festgelegt:
- RMS-VAD bleibt Core-intern (Safety-Net, keine ML-Deps)
- ML-VADs (Silero, Candle) kommen als Plugins via dediziertem `VadProvider`-Trait
- Rationale: `AudioFilter` transformiert Samples, VAD emittiert Gate-Events — Semantik-Mismatch

Trait-Signatur-Details wurden im Phase-0-JNI-Spike-Fenster verortet. JNI-Spike (ADR-0003) hat den Data-Plane-Pfad validiert — `VadDecision`-Events können 1:1 über den gleichen Kanal wie `AudioLevel` emittiert werden. Damit ist der Trait unblocked.

## Decision

`VadProvider`-Trait emittiert **Gate-Events**, keine transformierten Samples. Finale Signatur (in `klarvo-core/src/audio/vad/provider.rs`, re-exported via `klarvo_core::traits::VadProvider`):

```rust
#[async_trait]
pub trait VadProvider: Send + Sync {
    /// Process one chunk of PCM mono f32 samples. Emits a gate state after
    /// consuming this chunk. Stateful across calls; providers may buffer
    /// samples or hold per-stream state until `reset` is invoked.
    ///
    /// `ts_ms` is a caller-provided session-relative monotonic timestamp
    /// (same semantics as `AudioLevel::ts_ms` in klarvo-bridge-jni). The
    /// provider does NOT derive timestamps from sample counts or
    /// wall-clock — ts_ms ownership belongs to the audio pipeline.
    async fn process(
        &mut self,
        samples: &[f32],
        ts_ms: u64,
    ) -> Result<VadDecision, PluginError>;

    /// Reset per-stream state at recording-session boundaries.
    fn reset(&mut self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadDecision {
    Silence,
    SpeechStart { ts_ms: u64 },
    Speech,
    SpeechEnd { ts_ms: u64, duration_ms: u64 },
}
```

### Resolved Open Questions (aus dem Proposed-Stub)

**1. `ts_ms`-Source: Caller-Provided.**

Entscheidung angleichen an JNI-Spike (`klarvo-bridge-jni/src/commands.rs`, Producer-Task):
```rust
let start = Instant::now();
// ...
let ts_ms = start.elapsed().as_millis() as u64;
```
Der Audio-Producer hält **eine** session-relative monotone Clock und attached den Zeitstempel an jeden emittierten Sample-Chunk / Level-Event. `AudioLevel::ts_ms` und `VadDecision::ts_ms` teilen dadurch identische Semantik.

Rationale gegen Alternativen:
- **Sample-Count-intern:** bricht, wenn die Sample-Rate sich zwischen Plugins unterscheidet (Silero = 16 kHz vs. evtl. Native-Recording = 48 kHz). Der VAD müsste die Rate kennen, was ihn an Audio-Konfiguration koppelt.
- **Wall-Clock (`SystemTime::now`):** bricht die monotone-Garantie (Clock-Skew, NTP-Jumps) und wäre inkonsistent mit AudioLevel.
- **Caller-Provided:** Keine Audio-Config im Trait, beide Event-Typen (Level + VAD) stammen aus derselben Clock-Domain — Timeline-Korrelation im Consumer-Code trivial.

**2. Error-Shape: `PluginError` reuse, kein eigener VadError.**

Core-weite Error-Konsistenz über Plugin-Surfaces. RMS-Stub und Silero-Plugin werfen über denselben Typ, Downstream-`?`-Propagation via `From<PluginError> for AppError` bereits vorhanden.

**3. Kein `config()` / `frame_size()` im Trait.**

Plugin-spezifische Konfiguration läuft über Constructor-Parameter. RMS-Stub hat keinen relevanten Config-Shape (hartcodierter Threshold, Phase-2-Kalibrierung), Silero braucht sample_rate + frame_size und bekommt sie im `SileroVad::new(cfg)`. Ein generisches `config()` am Trait würde einen `Any`-Shape oder Plugin-spezifische Assoziated-Types erzwingen — beides ist Overkill für Phase 0. Der Caller muss Frame-Length-Constraints (z. B. Silero's 512 Samples @ 16 kHz) beim Konstruieren des Plugins kennen, nicht zur Trait-Interaction.

**4. Enum vs. Stream-of-Events: Enum akzeptiert.**

Bei parallelen Events (Speech-End + sofortiger Speech-Start im nächsten Sample) kommen beide Decisions aus zwei separaten `process()`-Calls — kein Informations-Loss, da der Caller ohnehin Sample-Chunks feedet, nicht einzelne Samples.

**5. `&mut self`: akzeptiert.**

Verhindert paralleles Füttern eines einzelnen VAD-Instances, was für sequenzielle Stream-Verarbeitung korrekt ist. Multi-Stream → mehrere Instances.

## Consequences

**Positiv:**
- `AudioFilter` bleibt semantisch sauber (Sample-in/Sample-out)
- Gate-Events sind direkt als `AudioEvent::VadState` (siehe Architecture-Doc §Audio-Pipeline-Abstraktion) broadcastbar — gleicher Pfad wie AudioLevel
- Stateful-Design erlaubt Silero-ONNX-Buffering ohne Trait-Änderung
- Caller-Provided-ts_ms entkoppelt VAD von Audio-Konfiguration; Plugin-Author muss nicht an Clock denken
- RMS-Safety-Net (`klarvo-core::audio::vad::rms::RmsVad`) implementiert den Trait compliant → Compile-Time-Beweis, dass die Signatur praktisch ist

**Negativ / akzeptierte Schulden:**
- `Mutex<Option<Listener>>` im Bridge-Data-Plane-Hotpath (ADR-0003) gilt auch für VAD-Events — Multi-Listener-Skalierung ist separates ADR
- RMS-Threshold `0.01` (~ -40 dBFS) ist ein Stub; echte Kalibrierung (Noise-Floor-Estimation, Hysterese) ist Phase-2-Arbeit

## Implementation

- `klarvo-core/src/audio/vad/provider.rs` — finaler Trait + `VadDecision`
- `klarvo-core/src/audio/vad/rms.rs` — `RmsVad` Safety-Net, 7 Unit-Tests
- `klarvo-core/src/traits/vad.rs` — Re-Export (`crate::traits::VadProvider`-Pfad stabil)
- `klarvo-core/Cargo.toml` — `tokio` als dev-dep für `#[tokio::test]`

Tests: `cargo test -p klarvo-core` → 7 passed; Workspace: `cargo check --workspace` grün; Clippy: `cargo clippy -p klarvo-core --all-targets -- -D warnings` grün.
