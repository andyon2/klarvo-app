use klarvo_core::audio::{AudioEvent, CaptureConfig, DEFAULT_AUDIOEVENT_CAPACITY};
use klarvo_core::audio::{AudioSource};
use klarvo_test_fixtures::MockAudioSource;
use tokio::sync::broadcast;

#[tokio::test]
async fn mock_emits_exact_chunk_count() {
    let (tx, mut rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, device: None, events: tx };
    let mut mock = MockAudioSource::with_synthetic_chunks(3, 1024, 64);
    let _handle = mock.start(config).await.unwrap();

    let mut chunk_count: usize = 0;
    loop {
        match rx.recv().await {
            Ok(AudioEvent::Samples { data, ts_ms }) => {
                assert_eq!(data.len(), 1024);
                assert_eq!(ts_ms, chunk_count as u64 * 64);
                chunk_count += 1;
            }
            Ok(AudioEvent::Level { .. }) => {}
            Err(broadcast::error::RecvError::Closed) => break,
            Err(e) => panic!("unexpected recv error: {e:?}"),
        }
    }
    assert_eq!(chunk_count, 3);
}

#[tokio::test]
async fn mock_early_drop_stops_emission() {
    let (tx, mut rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, device: None, events: tx };
    let mut mock = MockAudioSource::with_synthetic_chunks(3, 1024, 0);
    let handle = mock.start(config).await.unwrap();

    // Receive first chunk then drop the handle
    let first = rx.recv().await.unwrap();
    assert!(matches!(first, AudioEvent::Samples { .. }));
    drop(handle);

    // Drain remaining — should reach RecvError::Closed before all 3 chunks
    let mut extra_chunks: usize = 0;
    loop {
        match rx.recv().await {
            Ok(AudioEvent::Samples { .. }) => extra_chunks += 1,
            Ok(_) => {}
            Err(broadcast::error::RecvError::Closed) => break,
            Err(e) => panic!("unexpected recv error: {e:?}"),
        }
    }
    assert!(
        extra_chunks < 2,
        "expected early stop (< 2 extra chunks), got {extra_chunks}"
    );
}
