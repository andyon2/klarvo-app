pub mod buffer;
pub mod events;
pub mod keys;
pub mod source;
pub mod vad; // existing, unchanged

pub use buffer::AudioBuffer;
pub use events::AudioEvent;
pub use source::{AudioError, AudioSource, CaptureConfig, CaptureHandle};

/// Default `tokio::sync::broadcast` channel capacity for `AudioEvent` streams.
/// At ~1024 samples per chunk (64 ms @ 16 kHz), 256 slots ≈ 16 s of
/// audio-backlog before a consumer lags. Pass to
/// `broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY)` or override for testing
/// (e.g., capacity-1 for deterministic lag-simulation in ADR-0007 backpressure
/// tests). Ref ADR-0007-Amendment-Q1.
pub const DEFAULT_AUDIOEVENT_CAPACITY: usize = 256;

/// Fixed audio sample-rate emitted by all `AudioSource` impls
/// (ADR-0006-Sub-Decision-2: 16 kHz mono f32 Whisper-standard).
/// Consumed by `run_capture_session` for `AudioBuffer`-construction.
/// Phase-2+ variable-rate would require ADR-0006-Amendment and
/// `AudioBuffer`-contract-revision.
pub const AUDIO_SAMPLE_RATE: u32 = 16_000;
