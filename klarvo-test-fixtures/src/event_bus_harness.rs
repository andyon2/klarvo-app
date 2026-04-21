use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use klarvo_core::event::{ErrorEmitter, Event};
use tokio::sync::broadcast;

/// Collect up to `n` events from `rx` with per-event `timeout_ms` deadline.
/// Stops early if fewer events arrive before the timeout. Use in tests to drain
/// an `EventBus` subscriber without hanging indefinitely.
pub async fn collect_emitted(
    rx: &mut broadcast::Receiver<Event>,
    n: usize,
    timeout_ms: u64,
) -> Vec<Event> {
    let mut events = Vec::with_capacity(n);
    let timeout = Duration::from_millis(timeout_ms);
    for _ in 0..n {
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Ok(event)) => events.push(event),
            _ => break,
        }
    }
    events
}

/// Assert that at least one event in `events` satisfies `pred`.
///
/// `pred` is typically a `|ev| matches!(ev, Event::SomeVariant { .. })`
/// closure. Panics with the full event list on failure.
pub fn assert_contains_variant<F>(events: &[Event], pred: F)
where
    F: Fn(&Event) -> bool,
{
    assert!(
        events.iter().any(|ev| pred(ev)),
        "no event matched predicate; received events: {events:?}",
    );
}

/// Test-double for [`ErrorEmitter`] that records all `(key, ts_ms)` calls.
///
/// Shared via `Arc<MockErrorEmitter>` so multiple tasks can push errors and the
/// test inspects `recorded()` after awaiting.
pub struct MockErrorEmitter {
    collected: Arc<Mutex<Vec<(String, u64)>>>,
}

impl MockErrorEmitter {
    pub fn new() -> Self {
        Self { collected: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Return a snapshot of all recorded `(key, ts_ms)` pairs in call order.
    pub fn recorded(&self) -> Vec<(String, u64)> {
        self.collected.lock().unwrap().clone()
    }
}

impl Default for MockErrorEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorEmitter for MockErrorEmitter {
    async fn emit_error(&self, key: &str, ts_ms: u64) {
        self.collected.lock().unwrap().push((key.to_string(), ts_ms));
    }
}
