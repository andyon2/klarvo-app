pub mod events;
pub mod keys;
pub mod source;
pub mod vad; // existing, unchanged

pub use events::AudioEvent;
pub use source::{AudioError, AudioSource, CaptureConfig, CaptureHandle};

/// Default `tokio::sync::broadcast` channel capacity for `AudioEvent` streams.
/// At ~1024 samples per chunk (64 ms @ 16 kHz), 256 slots ≈ 16 s of
/// audio-backlog before a consumer lags. Pass to
/// `broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY)` or override for testing
/// (e.g., capacity-1 for deterministic lag-simulation in ADR-0007 backpressure
/// tests). Ref ADR-0007-Amendment-Q1.
pub const DEFAULT_AUDIOEVENT_CAPACITY: usize = 256;
