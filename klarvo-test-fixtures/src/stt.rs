use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use klarvo_core::audio::AudioBuffer;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::SttProvider;

/// Test fixture implementing [`SttProvider`] with a sequential canned-transcription queue.
///
/// Each `transcribe` call returns the next element from `canned_transcriptions` in order.
/// An exhausted queue returns an `Internal` error. Use with [`assert_stt_call_count`].
///
/// For simple single-response fixtures use `klarvo_test_fixtures::MockSttProvider::returning()`.
pub struct QueuedMockSttProvider {
    canned_transcriptions: Vec<String>,
    call_count: Arc<AtomicUsize>,
}

impl QueuedMockSttProvider {
    /// Construct with a fixed sequence of transcriptions returned in order.
    pub fn with_transcriptions(canned_transcriptions: Vec<String>) -> Self {
        Self {
            canned_transcriptions,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Total number of times `transcribe`/`process` has been called.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PipelineStage for QueuedMockSttProvider {
    type Input = AudioBuffer;
    type Output = String;

    async fn process(&self, _input: AudioBuffer) -> Result<String, AppError> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        self.canned_transcriptions.get(idx).cloned().ok_or_else(|| AppError {
            kind: AppErrorKind::Internal,
            message: format!("QueuedMockSttProvider canned queue exhausted at call {idx}"),
            user_message: None,
            retryable: false,
        })
    }

    fn stage_type(&self) -> &'static str {
        "stt"
    }
}

#[async_trait]
impl SttProvider for QueuedMockSttProvider {}

// Object-safety compile-test: Box<dyn SttProvider> must compile.
#[allow(dead_code)]
fn _obj_safe_stt(_x: Box<dyn SttProvider>) {}

/// Assert that `mock` was called exactly `expected` times.
pub fn assert_stt_call_count(mock: &QueuedMockSttProvider, expected: usize) {
    assert_eq!(
        mock.call_count(),
        expected,
        "QueuedMockSttProvider: call count {actual} != expected {expected}",
        actual = mock.call_count()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf() -> AudioBuffer {
        AudioBuffer { samples: vec![0.0; 16], sample_rate: 16_000, ts_ms_start: 0, ts_ms_end: 1 }
    }

    #[tokio::test]
    async fn returns_canned_values_in_order() {
        let mock = QueuedMockSttProvider::with_transcriptions(vec![
            "hello".to_string(),
            "world".to_string(),
        ]);
        assert_eq!(mock.transcribe(buf()).await.unwrap(), "hello");
        assert_eq!(mock.transcribe(buf()).await.unwrap(), "world");
    }

    #[tokio::test]
    async fn call_count_increments() {
        let mock = QueuedMockSttProvider::with_transcriptions(vec!["a".to_string(), "b".to_string()]);
        let _ = mock.transcribe(buf()).await;
        assert_stt_call_count(&mock, 1);
        let _ = mock.transcribe(buf()).await;
        assert_stt_call_count(&mock, 2);
    }

    #[tokio::test]
    async fn exhausted_queue_returns_error() {
        let mock = QueuedMockSttProvider::with_transcriptions(vec!["only".to_string()]);
        let _ = mock.transcribe(buf()).await.unwrap();
        assert!(mock.transcribe(buf()).await.is_err());
    }
}
