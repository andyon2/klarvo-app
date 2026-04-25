use tokio::sync::broadcast;

use crate::i18n;

/// Application events emitted by core and consumed by shells / pipeline stages.
///
/// All string payloads are i18n keys (FR8 / NFR-G3). Shell-side tauri-specta
/// mirror types live in `shells/windows/src-tauri/src/events.rs` (Epic 3 scope)
/// with `#[tauri_specta(event_name = "...")]` wire-name annotations following
/// the `<domain>.<event>` convention (e.g. `recording.started`).
///
/// `ts_ms` is session-relative monotone milliseconds per ADR-0001 and
/// `memory/project_event_ts_ms_convention`.
#[derive(Debug, Clone)]
pub enum Event {
    RecordingStarted { ts_ms: u64 },
    RecordingStopped { ts_ms: u64 },
    PipelineStageStarted { stage_type: String, ts_ms: u64 },
    PipelineStageCompleted { stage_type: String, ts_ms: u64 },
    /// `error_key` MUST be a valid i18n key (validated via `Event::error_emitted`
    /// constructor in debug builds). Use `Event::error_emitted(key, ts_ms)` at
    /// emission sites to enforce this invariant.
    ErrorEmitted { error_key: String, ts_ms: u64 },
}

impl Event {
    /// Construct `ErrorEmitted`, asserting key format in debug builds.
    ///
    /// Prefer this over the struct-literal form so that invalid keys surface
    /// immediately during development without runtime cost in release.
    pub fn error_emitted(error_key: impl Into<String>, ts_ms: u64) -> Self {
        let error_key = error_key.into();
        debug_assert!(i18n::is_key(&error_key), "invalid i18n key: {error_key:?}");
        Event::ErrorEmitted { error_key, ts_ms }
    }
}

/// Default broadcast capacity for the [`EventBus`].
///
/// Sized for typical Phase-1 burst: hotkey-press + RecordingStarted +
/// per-stage start/complete events + RecordingStopped within a single
/// PTT cycle. Subscribers that lag past this watermark observe
/// `RecvError::Lagged`; tray and EventMirror tasks are expected to drain
/// well below it.
pub const DEFAULT_EVENT_BUS_CAPACITY: usize = 64;

/// Non-blocking broadcast bus for [`Event`] values.
///
/// Backed by `tokio::sync::broadcast`; `emit` ignores `SendError::Receivers`
/// per ADR-0007 (no receivers is not an error, events are advisory). Cloning
/// a `broadcast::Sender` before subscriptions are created is intentional —
/// the sender is kept alive on the bus so that subscribers added later see
/// future events, not a closed channel.
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Emit `event` to all current subscribers. Non-blocking; ignored if no
    /// receivers are registered (ADR-0007: `NoReceivers` is not an error).
    pub fn emit(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}
