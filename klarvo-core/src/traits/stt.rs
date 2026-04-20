use async_trait::async_trait;

use crate::audio::AudioBuffer;
use crate::error::AppError;
use crate::pipeline::PipelineStage;

/// STT provider: transcribes a complete utterance [`AudioBuffer`] to a UTF-8 `String`.
///
/// # Intent
///
/// `SttProvider` is the audio-to-text boundary in the Phase-1 pipeline. Implementations
/// consume a full utterance [`AudioBuffer`] and return the best-effort transcription.
///
/// # Method Contract
///
/// - **Input pre-condition**: `AudioBuffer.samples` is non-empty; `sample_rate` is
///   supported by the upstream provider (e.g., 16_000 Hz for Groq Whisper). Callers
///   SHOULD validate sample_rate against provider constraints before dispatching.
/// - **Output**: UTF-8 text. May be an empty string on silence. Never `None`.
/// - **Error variants**: `AppError::kind::Network` (provider unreachable),
///   `AppError::kind::Auth` (401/403), `AppError::kind::RateLimit` (429),
///   `AppError::kind::UpstreamUnavailable` (5xx/timeout).
///
/// # Implementing
///
/// Implement [`PipelineStage`] with `Input = AudioBuffer, Output = String` and provide
/// an empty `impl SttProvider for YourType {}` to inherit the default [`Self::transcribe`].
/// Override `transcribe` only when pre/post-processing (resampling, language hints, etc.)
/// is required beyond the raw `process` call.
///
/// # Mock for Tests
///
/// Use `klarvo_test_fixtures::MockSttProvider` for unit-testing pipeline stages.
///
/// # Cross-References
///
/// - Reference implementation: `klarvo-plugin-groq` (Epic 1B Story 1B.4).
/// - Executor dispatch: `klarvo-core::pipeline::run_pipeline` (Story 1B.5).
/// - [`PipelineStageType::Stt`]: Compile-time registry variant (Story 1B.1).
/// - FR6: Compile-time Stage-Registry-Set via Cargo features + enum variants.
///
/// # Phase-1 Stability
///
/// Signature is **Phase-1-locked**. Post-close changes trigger Breaking-Change-Review.
#[async_trait]
pub trait SttProvider: PipelineStage<Input = AudioBuffer, Output = String> {
    /// Transcribe `audio` to text. Defaults to delegating to [`PipelineStage::process`].
    ///
    /// Override to add provider-specific pre/post-processing while preserving the
    /// standard [`PipelineStage`] contract for Executor dispatch.
    async fn transcribe(&self, audio: AudioBuffer) -> Result<String, AppError> {
        self.process(audio).await
    }
}
