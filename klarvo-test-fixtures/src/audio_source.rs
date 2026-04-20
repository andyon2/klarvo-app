use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::oneshot;

use klarvo_core::audio::{AudioError, AudioEvent, AudioSource, CaptureConfig, CaptureHandle};

/// Test fixture implementing `AudioSource`. Emits synthetic zero-filled chunks
/// at the specified rate. `chunk_interval_ms = 0` for fastest-possible emission
/// (unit-tests); nonzero for backpressure-simulation (ADR-0007 lag-tests).
/// WAV-file-playback variant is Story 2.4 scope. Factor-out deferred until
/// Story 2.4 proves the need (ref `memory/feedback_premature_abstraction_guard`).
pub struct MockAudioSource {
    count: usize,
    samples_per_chunk: usize,
    chunk_interval_ms: u64,
}

impl MockAudioSource {
    pub fn with_synthetic_chunks(
        count: usize,
        samples_per_chunk: usize,
        chunk_interval_ms: u64,
    ) -> Self {
        Self { count, samples_per_chunk, chunk_interval_ms }
    }
}

#[async_trait]
impl AudioSource for MockAudioSource {
    async fn start(
        &mut self,
        config: CaptureConfig,
    ) -> Result<CaptureHandle, AudioError> {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let count = self.count;
        let samples_per_chunk = self.samples_per_chunk;
        let chunk_interval_ms = self.chunk_interval_ms;
        let sender = config.events;

        tokio::spawn(async move {
            for i in 0..count {
                let data: Arc<[f32]> = Arc::from(vec![0.0_f32; samples_per_chunk]);
                let ts_ms = i as u64 * chunk_interval_ms;
                let _ = sender.send(AudioEvent::Samples { data, ts_ms });

                // After each send, yield or sleep — checking shutdown in parallel.
                // This guarantees a task-switch between chunk emissions, so the
                // consumer can drop CaptureHandle and stop further emission.
                if chunk_interval_ms > 0 {
                    tokio::select! {
                        biased;
                        _ = &mut shutdown_rx => return,
                        _ = tokio::time::sleep(
                            std::time::Duration::from_millis(chunk_interval_ms)
                        ) => {}
                    }
                } else {
                    tokio::select! {
                        biased;
                        _ = &mut shutdown_rx => return,
                        _ = tokio::task::yield_now() => {}
                    }
                }
            }
            // sender drops here → RecvError::Closed for downstream receivers
        });

        Ok(CaptureHandle::new(shutdown_tx))
    }
}
