use klarvo_core::audio::{AudioEvent, CaptureHandle, DEFAULT_AUDIOEVENT_CAPACITY};
use tokio::sync::broadcast;

/// Verifies that dropping a `CaptureHandle` whose guard is a broadcast `Sender`
/// closes the channel and causes downstream receivers to observe
/// `RecvError::Closed` — without requiring a real cpal audio device.
///
/// This validates the opaque-guard contract introduced by the CaptureHandle
/// Option-A refactor (Story 2.5 Divergenz 1): any `G: Send + 'static` whose
/// `Drop` closes the channel satisfies the session-lifetime contract.
#[tokio::test]
async fn broadcast_sender_drop_closes_receivers() {
    let (tx, mut rx) = broadcast::channel::<AudioEvent>(DEFAULT_AUDIOEVENT_CAPACITY);
    // tx is the guard — dropping CaptureHandle drops tx, closing the channel.
    let handle = CaptureHandle::new(tx);
    drop(handle);

    match rx.recv().await {
        Err(broadcast::error::RecvError::Closed) => {}
        other => panic!("expected RecvError::Closed, got {other:?}"),
    }
}
