//! VAD-Gate + Pipeline-Entry-Aggregation orchestrator.
//!
//! # Phase-1 Limitation
//!
//! Phase-1 limitation: buffering begins on `VadDecision::SpeechStart` — no pre-buffer before
//! first speech-onset. The first syllable may be clipped, particularly with high-threshold
//! RMS-VAD. Revisit-Trigger: Silero-VAD-Plugin-Introduction in Phase 2+, which has inherent
//! decision-lag requiring pre-buffer design (see ADR-0001 VadProvider-Trait for phase-2
//! extension-points).
//!
//! # Return Semantics
//!
//! Returns `Ok(Some(StageData))` when a speech segment was detected, VAD-gated, and passed
//! through `run_pipeline`. Returns `Ok(None)` when the broadcast-channel closed before any
//! speech was detected (accidental hotkey-trigger — caller swallows silently). Returns
//! `Err(AppError)` only on pipeline-stage-failures (STT-timeout, cleanup-error,
//! plugin-not-found) or `VadProvider`-impl-errors. `RecvError::Lagged` is NOT propagated as
//! error — Log-and-Continue per ADR-0007.
//!
//! # VAD Internality
//!
//! `VadProvider` is Core-internal (ref architecture.md §Plugin-System: 'VAD bleibt
//! Core-intern'). It is NOT registered in `PluginRegistry` — passed as `&mut dyn VadProvider`
//! parameter. Caller constructs the concrete impl (e.g. `RmsVadProvider` in production,
//! `MockVadProvider` in tests) and owns its lifecycle.
//!
//! # Multi-Utterance
//!
//! This function handles exactly one SpeechStart→SpeechEnd cycle per call.
//! Multi-utterance-per-hold is Phase-2+ scope. Caller re-invokes for each hold-to-talk
//! activation.
//!
//! # OutputTarget Forward-Reference
//!
//! OutputTarget-Delivery (`output.deliver(text)`) is Story 2.5 + Story 2.4 (E2E Headless
//! Flow) scope — not included here. Callers extract `StageData::Text(text)` from the
//! `Ok(Some(...))` result and dispatch to the registered `OutputTarget` plugin.

use tokio::sync::broadcast::error::RecvError;

use crate::audio::{AudioBuffer, AudioEvent, AUDIO_SAMPLE_RATE};
use crate::audio::vad::{VadDecision, VadProvider};
use crate::error::AppError;
use crate::manifest::PipelineManifest;
use crate::pipeline::stage_data::StageData;
use crate::pipeline::executor::run_pipeline;
use crate::registry::PluginRegistry;

/// Consume `AudioEvent` samples from `receiver`, VAD-gate them via `vad`, accumulate
/// speech into an [`AudioBuffer`], and pipe through `run_pipeline` on speech-end.
///
/// See module-level doc for return semantics, VAD-internality contract, and scope
/// limitations.
pub async fn run_capture_session(
    mut receiver: tokio::sync::broadcast::Receiver<AudioEvent>,
    vad: &mut dyn VadProvider,
    manifest: &PipelineManifest,
    registry: &PluginRegistry,
) -> Result<Option<StageData>, AppError> {
    vad.reset();

    let mut accumulating = false;
    let mut ts_ms_start: u64 = 0;
    let mut accumulated: Vec<f32> = Vec::new();
    // Tracked for Closed-mid-Speech ts_ms_end estimation (D3).
    let mut last_chunk_ts_ms: u64 = 0;
    let mut last_chunk_len: usize = 0;

    // Diagnostic counters: surface "why was this session empty?" on the Ok(None)
    // path. Zero samples means audio source never delivered; samples-seen with
    // max_rms below VAD threshold means mic-input too quiet.
    let mut samples_seen: usize = 0;
    let mut max_rms: f32 = 0.0;

    loop {
        match receiver.recv().await {
            Ok(AudioEvent::Samples { data, ts_ms }) => {
                let samples: &[f32] = data.as_ref();
                samples_seen += 1;
                if !samples.is_empty() {
                    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
                    let rms = (sum_sq / samples.len() as f32).sqrt();
                    if rms > max_rms {
                        max_rms = rms;
                    }
                }
                let decision =
                    vad.process(samples, ts_ms).await.map_err(AppError::from)?;

                match decision {
                    VadDecision::SpeechStart { ts_ms: start_ts } => {
                        accumulating = true;
                        ts_ms_start = start_ts;
                        accumulated.clear();
                        accumulated.extend_from_slice(samples);
                        last_chunk_ts_ms = ts_ms;
                        last_chunk_len = samples.len();
                    }
                    VadDecision::Speech => {
                        if accumulating {
                            accumulated.extend_from_slice(samples);
                            last_chunk_ts_ms = ts_ms;
                            last_chunk_len = samples.len();
                        }
                    }
                    VadDecision::SpeechEnd { ts_ms: end_ts, .. } => {
                        if accumulating {
                            let buffer = AudioBuffer {
                                samples: std::mem::take(&mut accumulated),
                                sample_rate: AUDIO_SAMPLE_RATE,
                                ts_ms_start,
                                ts_ms_end: end_ts,
                            };
                            let result = run_pipeline(
                                manifest,
                                registry,
                                StageData::Audio(buffer),
                            )
                            .await?;
                            return Ok(Some(result));
                        }
                    }
                    VadDecision::Silence => {} // no buffering, no state change
                }
            }
            Ok(AudioEvent::Level { .. }) => {} // no VAD-processing needed for level events

            Err(RecvError::Lagged(n)) => {
                // ADR-0007 Sub-Decision-2: Log-and-Continue, never propagate as Err.
                tracing::warn!(
                    target: "klarvo.audio.backpressure",
                    skipped = n,
                    consumer = "capture_session",
                    "audio event consumer lagged; skipped events"
                );
            }

            Err(RecvError::Closed) => {
                // Closed-mid-Speech: treat as SpeechEnd-equivalent.
                // Hotkey-Release-mid-Speech is a normal user-action. Treating
                // Closed-mid-Speech as SpeechEnd-equivalent delivers the partial
                // transcription rather than silently discarding buffered audio.
                if accumulating && !accumulated.is_empty() {
                    let estimated_end = last_chunk_ts_ms
                        + (last_chunk_len as u64 * 1000 / AUDIO_SAMPLE_RATE as u64);
                    let buffer = AudioBuffer {
                        samples: accumulated,
                        sample_rate: AUDIO_SAMPLE_RATE,
                        ts_ms_start,
                        ts_ms_end: estimated_end,
                    };
                    let result =
                        run_pipeline(manifest, registry, StageData::Audio(buffer)).await?;
                    return Ok(Some(result));
                }
                // Closed-before-SpeechStart: VAD never detected speech.
                // Either an accidental hotkey-trigger (very short press) OR the
                // mic input never exceeded the VAD threshold (gain too low,
                // wrong device, muted). Log diagnostic counters so the user
                // can tell the two apart from the rolling-file log.
                tracing::info!(
                    target: "klarvo.audio.capture",
                    samples_seen,
                    max_rms = max_rms,
                    "capture closed without speech detection (Ok(None))"
                );
                return Ok(None);
            }
        }
    }
}
