// Windows-only: CpalAudioSource via klarvo-audio-cpal (WASAPI backend). No cross-platform path.

use std::sync::Arc;

use klarvo_audio_cpal::CpalAudioSource;
use klarvo_core::audio::AudioSource;

/// Construct a `CpalAudioSource` wrapped for injection into `SessionOrchestrator`.
///
/// Return type matches `SessionOrchestrator::new(audio_source:
/// Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>)` (ADR-0012 §SD-2) exactly.
///
/// - `Mutex`: `AudioSource::start` takes `&mut self` (ADR-0006 compile-time borrow-guard).
/// - `Arc`: shared-ownership between Orchestrator and potential future diagnostics.
///
/// `CpalAudioSource` error model: capture errors are emitted via `tracing::warn!` (not
/// `ErrorEmitter`). Stream-error-path closes the broadcast channel (`RecvError::Closed`)
/// → pipeline terminates naturally. See `klarvo-audio-cpal/src/source.rs` error-callback.
///
/// Story 3.10 calls `make_audio_source()` in `main.rs` `.setup()` hook.
///
/// # Audio Config
///
/// Audio-Config (sample-rate, channels) uses `CpalAudioSource` defaults:
/// Default-Host → Default-Input-Device → Default-Input-Config.
// Phase-2: drive sample_rate/channels from ShellConfig (currently CpalAudioSource defaults).
pub fn make_audio_source() -> Arc<tokio::sync::Mutex<Box<dyn AudioSource>>> {
    Arc::new(tokio::sync::Mutex::new(Box::new(CpalAudioSource)))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

    #[test]
    fn audio_source_factory_produces_orchestrator_compatible_type() {
        let _audio: Arc<tokio::sync::Mutex<Box<dyn AudioSource>>> = make_audio_source();
        // Compile-check only; no runtime start (real cpal-device-init requires OS audio device).
        // If this compiles, the return type is compatible with SessionOrchestrator::new().
    }
}
