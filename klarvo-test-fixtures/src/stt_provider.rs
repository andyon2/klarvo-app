use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use klarvo_core::audio::AudioBuffer;
use klarvo_core::error::AppError;
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::SttProvider;

/// Test fixture implementing `SttProvider`. Returns a fixed transcription string regardless
/// of audio content. Use for pipeline-wiring tests where STT-accuracy is not under test.
pub struct MockSttProvider {
    response: String,
    /// If set, the AudioBuffer received by transcribe() is stored here for test assertions.
    /// Populated by [`MockSttProvider::with_capture`].
    captured: Option<Arc<Mutex<Option<AudioBuffer>>>>,
}

impl MockSttProvider {
    /// Construct a mock that returns `text` for every transcription call.
    pub fn returning(text: impl Into<String>) -> Self {
        Self { response: text.into(), captured: None }
    }

    /// Construct a mock that returns `text` AND stores the received `AudioBuffer` in
    /// `captured` for post-call assertion (Decision-Point D1: ts_ms-Assertion-Mechanism).
    ///
    /// Only the most recent `AudioBuffer` is stored — overwrites on repeated calls.
    pub fn with_capture(text: impl Into<String>, captured: Arc<Mutex<Option<AudioBuffer>>>) -> Self {
        Self { response: text.into(), captured: Some(captured) }
    }
}

#[async_trait]
impl PipelineStage for MockSttProvider {
    type Input = AudioBuffer;
    type Output = String;

    async fn process(&self, input: AudioBuffer) -> Result<String, AppError> {
        if let Some(cap) = &self.captured {
            *cap.lock().unwrap() = Some(input);
        }
        Ok(self.response.clone())
    }

    fn stage_type(&self) -> &'static str {
        "stt"
    }
}

#[async_trait]
impl SttProvider for MockSttProvider {}
