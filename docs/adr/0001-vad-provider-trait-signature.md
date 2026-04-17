# ADR-0001: VadProvider-Trait-Signatur (Gate-Events, nicht Sample-Transform)

**Status:** Proposed
**Date:** 2026-04-18

## Context

Step-4-Revision (Architecture-Doc §VAD-Split, ca. Zeile 887) hat festgelegt:
- RMS-VAD bleibt Core-intern (Safety-Net, keine ML-Deps)
- ML-VADs (Silero, Candle) kommen als Plugins via dediziertem `VadProvider`-Trait
- Rationale: `AudioFilter` transformiert Samples, VAD emittiert Gate-Events — Semantik-Mismatch

Trait-Signatur-Details wurden im Phase-0-JNI-Spike-Fenster verortet, aber Pre-Flight möchte einen validen Startpunkt vor Scaffold.

## Decision

`VadProvider`-Trait emittiert **Gate-Events**, keine transformierten Samples. Minimal-Interface (zu finalisieren im JNI-Spike):

```rust
#[async_trait]
pub trait VadProvider: Send + Sync {
    /// Feed samples, get gate state. Stateful across calls (provider holds model).
    async fn process(&mut self, samples: &[f32]) -> Result<VadDecision, PluginError>;

    /// Reset stream state (new recording session).
    fn reset(&mut self);
}

pub enum VadDecision {
    Silence,
    SpeechStart { ts_ms: u64 },
    Speech,
    SpeechEnd { ts_ms: u64, duration_ms: u64 },
}
```

## Consequences

**Positiv:**
- `AudioFilter` bleibt semantisch sauber (Sample-in/Sample-out)
- Gate-Events sind direkt als `AudioEvent::VadState` (siehe Architecture-Doc §Audio-Pipeline-Abstraktion) broadcastbar
- Stateful-Design erlaubt Silero-ONNX-Buffering ohne Trait-Änderung

**Negativ / offen:**
- `ts_ms`-Source nicht festgelegt — Sample-Count / Wall-Clock / Caller-Provided? → im JNI-Spike klären (Audio-Level-Meter zeigt dasselbe Problem)
- `VadDecision` als Enum vs. Stream-of-Events: Enum ist einfacher, aber bei parallel-Events (z. B. Speech-End + sofort neuer Speech-Start im nächsten Sample) muss process zweimal gerufen werden
- `&mut self` vs. `&self` + interior-mutability — `&mut` ist ergonomischer, verhindert aber paralleles Füttern. Für VAD ist sequenzielle Verarbeitung korrekt → `&mut self` akzeptabel

## Next Action

Finale Signatur während Phase-0-JNI-Spike committen. Danach ADR auf `Accepted` setzen.
