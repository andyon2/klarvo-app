use async_trait::async_trait;

use crate::audio::AudioEvent;

/// Errors from `AudioSource::start` and the capture session.
///
/// `#[non_exhaustive]` — Phase-3 Android-impl will add `PermissionDenied`;
/// match arms must use `_ =>` to stay forward-compatible.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("audio device unavailable")]
    DeviceUnavailable,
    #[error("unsupported audio format")]
    UnsupportedFormat,
    // `msg` avoids thiserror 2.x auto-source-detection on fields named `source`
    // (String doesn't implement std::error::Error so thiserror would reject `source: String`).
    #[error("capture interrupted: {msg}")]
    CaptureInterrupted { msg: String },
    #[error("resample failed: {msg}")]
    ResampleFailed { msg: String },
    #[error("device configuration error: {msg}")]
    DeviceConfigError { msg: String },
}

pub struct CaptureConfig {
    /// Advisory sample-rate. Impls resample to 16 kHz if possible.
    pub sample_rate: u32,
    /// Advisory channel-count. Impls downmix to mono if possible.
    pub channels: u16,
    /// Broadcast-sender; AudioSource publishes AudioEvent variants here.
    ///
    /// Caller constructs the channel via
    /// `tokio::sync::broadcast::channel(klarvo_core::audio::DEFAULT_AUDIOEVENT_CAPACITY)`
    /// and holds the corresponding `Receiver` for downstream consumption.
    /// AudioSource implementations publish `AudioEvent` variants autonomously
    /// during an active capture-session.
    pub events: tokio::sync::broadcast::Sender<AudioEvent>,
}

/// Drop-guard that stops the capture-session and releases OS resources. Hold
/// this value for the lifetime of the capture session; dropping it signals the
/// capture-thread to terminate. Downstream consumers observe
/// `RecvError::Closed` on their broadcast-receivers after the handle is
/// dropped. Panic-safe: `Drop` fires unconditionally on scope-exit, including
/// via panic-unwind (ref `memory/feedback_test_raii_cleanup_pattern`).
///
/// Internally an opaque `Box<dyn Any + Send>` so that impl-crates
/// (`klarvo-audio-cpal`, Phase-3 Android) can store platform-specific guards
/// without pulling cpal/JNI into `klarvo-core`. The `Any + Send` vtable
/// dispatches `Drop` correctly for any concrete guard type.
pub struct CaptureHandle {
    _guard: Box<dyn std::any::Any + Send>,
}

impl CaptureHandle {
    pub fn new<G: Send + 'static>(guard: G) -> Self {
        Self { _guard: Box::new(guard) }
    }
}

// Safety: CaptureHandle has no &self methods; _guard is only accessed at Drop
// time by the owning thread. Implementing Sync is sound because there is no
// way to observe shared mutable state through &CaptureHandle.
unsafe impl Sync for CaptureHandle {}

/// Infrastructure-Trait (per ADR-0006, Accepted 2026-04-19). Not part of the
/// 4-Trait-Data-Flow-Stability-Ring (`PipelineStage` / `SttProvider` /
/// `CleanupStyle` / `VadProvider`). `AudioSource` is Infrastructure-Category,
/// analogous to `KeyStore` (Epic 1C): one Shell-Binary-scoped impl per
/// platform — never registry-looked-up. Impls live in `shells/windows/`
/// (Epic 3 cpal) and `shells/android/` (Phase 3 AudioRecord). Core holds
/// only the Trait.
///
/// **Production** implementations MUST set `ts_ms` on emitted
/// `AudioEvent::Samples` to the chunk-START timestamp, derived from a single
/// `Instant` captured at the start of `start()`, as session-relative monotone
/// milliseconds (ref ADR-0001, `memory/project_event_ts_ms_convention`).
/// Downstream consumers can compute chunk-end as
/// `ts_ms + (data.len() as u64 * 1000 / 16_000)`.
///
/// Implementations SHOULD resample and downmix to 16 kHz mono f32 before
/// emitting `AudioEvent::Samples` (Whisper-standard per ADR-0006-Sub-Decision-2).
/// If the hardware cannot satisfy the advisory `CaptureConfig.sample_rate` /
/// `.channels`, return `AudioError::UnsupportedFormat`. Emitted chunk-size is
/// implementation-internal (example: ~1024 samples = 64 ms @ 16 kHz, subject
/// to OS-audio-driver granularity per ADR-0006-Amendment-Q2).
///
/// `&mut self` prevents parallel-invocation of `start()` on a single
/// `AudioSource` instance, making the borrow-checker a compile-time guard
/// against overlapping capture-sessions. Multi-session requires multiple
/// `AudioSource` instances (per ADR-0006-Sub-Decision-5, analogous to
/// ADR-0001 §Resolved-Q5 for `VadProvider`).
///
/// `CpalAudioSource` is Story-2.5-scope (`klarvo-audio-cpal/` workspace-root
/// crate); Shell-integration (instantiation in `shells/windows/`) is
/// Epic-3-scope. Android-AudioRecord-Impl is Phase-3 scope (`shells/android/`).
/// Phase-1 Core-tests use `klarvo_test_fixtures::MockAudioSource`. `AudioBuffer`
/// (aggregate-type for `StageData::Audio`) and `audio/buffer.rs` are Story 2.2
/// scope; `AudioEvent::Samples` chunks are the raw stream that Story 2.2
/// aggregates. (ref ADR-0006 Amendment 2 — location+naming corrigendum)
#[async_trait]
pub trait AudioSource: Send + 'static {
    async fn start(
        &mut self,
        config: CaptureConfig,
    ) -> Result<CaptureHandle, AudioError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_error_send_sync_static() {
        fn _assert<T: Send + Sync + 'static>() {}
        _assert::<AudioError>();
        _assert::<CaptureConfig>();
        _assert::<CaptureHandle>();
    }
}
