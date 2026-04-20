use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// PCM mono f32 samples emitted by an `AudioSource` impl.
    ///
    /// `ts_ms`: Timestamp of chunk START, caller-monotone ms since
    /// session-start (ref ADR-0001, memory/project_event_ts_ms_convention).
    /// AudioSource-impls hold one `Instant` captured in `start()` and derive
    /// `ts_ms = instant.elapsed().as_millis() as u64` for each emitted chunk.
    Samples { data: Arc<[f32]>, ts_ms: u64 },
    /// RMS level for UI meter (0.0..=1.0).
    Level { rms: f32, ts_ms: u64 },
}
