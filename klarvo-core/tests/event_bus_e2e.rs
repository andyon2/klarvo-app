#![allow(clippy::disallowed_methods)]

use klarvo_core::event::{Event, EventBus};

const CAPACITY: usize = 16;

#[tokio::test]
async fn subscribe_before_emit_receives() {
    let bus = EventBus::new(CAPACITY);
    let mut rx = bus.subscribe();
    bus.emit(Event::RecordingStarted { ts_ms: 0 });
    let received = rx.recv().await.unwrap();
    assert!(matches!(received, Event::RecordingStarted { ts_ms: 0 }));
}

#[tokio::test]
async fn multiple_subscribers_all_receive() {
    let bus = EventBus::new(CAPACITY);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();
    bus.emit(Event::RecordingStopped { ts_ms: 42 });
    let e1 = rx1.recv().await.unwrap();
    let e2 = rx2.recv().await.unwrap();
    assert!(matches!(e1, Event::RecordingStopped { ts_ms: 42 }));
    assert!(matches!(e2, Event::RecordingStopped { ts_ms: 42 }));
}

#[tokio::test]
async fn emit_without_subscribers_is_noop() {
    let bus = EventBus::new(CAPACITY);
    // No subscribers registered — must not panic (ADR-0007: no receivers is not an error).
    bus.emit(Event::RecordingStarted { ts_ms: 0 });
}

#[tokio::test]
async fn error_emitted_constructor_validates_key_in_debug() {
    let bus = EventBus::new(CAPACITY);
    let mut rx = bus.subscribe();
    bus.emit(Event::error_emitted("error.pipeline.unknown_stage", 10));
    let received = rx.recv().await.unwrap();
    assert!(matches!(
        received,
        Event::ErrorEmitted { error_key: ref k, ts_ms: 10 } if k == "error.pipeline.unknown_stage"
    ));
}
