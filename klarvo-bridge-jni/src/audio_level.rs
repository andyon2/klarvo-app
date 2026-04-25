use serde::{Deserialize, Serialize};

/// Audio RMS level event emitted from the data-plane at ~20 Hz.
///
/// Rust-only type. Not exported via uniffi — transport to Kotlin is through
/// the raw-jni callback (`streams::emit_audio_level`), not the control-plane.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioLevel {
    pub rms: f32,
    pub ts_ms: u64,
}
