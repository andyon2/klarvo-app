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
///
/// # Recording lifecycle
///
/// A push-to-talk cycle emits a fixed three-event sequence:
///
/// 1. [`Event::RecordingStarted`] — audio capture began (`AudioSource::start`
///    returned `Ok`). Subscribers reflect "user is recording" state.
/// 2. [`Event::RecordingStopped`] — audio capture ended on hotkey-release
///    (`CaptureHandle` dropped → broadcast sender closed). Pipeline processing
///    may still be running; subscribers should NOT treat this as "system idle".
/// 3. [`Event::RecordingCompleted`] — pipeline task finished, regardless of
///    outcome (delivered, no-speech, or error). Subscribers reflecting end-to-end
///    completion (tray idle, "Klarvo ready" UI) consume this event.
#[derive(Debug, Clone)]
pub enum Event {
    /// Audio capture has started after a successful `AudioSource::start`.
    /// Emitted synchronously inside `on_press` before the pipeline task spawns.
    RecordingStarted { ts_ms: u64 },
    /// Audio capture has stopped on hotkey-release. The `CaptureHandle` has
    /// been dropped, signalling the audio task to exit. Pipeline processing
    /// may still be running asynchronously; for end-to-end completion listen
    /// for [`Event::RecordingCompleted`] instead.
    RecordingStopped { ts_ms: u64 },
    /// Pipeline processing has finished. Emitted from the detached pipeline
    /// task after delivery / no-speech-discard / error-emit, regardless of
    /// outcome. Subscribers reflecting "system idle" UI consume this event.
    /// `ts_ms` reflects pipeline-completion time, not hotkey-release time.
    RecordingCompleted { ts_ms: u64 },
    /// User aborted the recording session via the Pill-Bar abort button.
    /// Audio buffer is discarded; pipeline task is hard-cancelled (no STT call,
    /// no paste). Pill-Bar fades out on this event identically to RecordingCompleted.
    RecordingAborted { ts_ms: u64 },
    PipelineStageStarted { stage_type: String, ts_ms: u64 },
    PipelineStageCompleted { stage_type: String, ts_ms: u64 },
    /// `error_key` MUST be a valid i18n key (validated via `Event::error_emitted`
    /// constructor in debug builds). Use `Event::error_emitted(key, ts_ms)` at
    /// emission sites to enforce this invariant.
    ErrorEmitted { error_key: String, ts_ms: u64 },
    /// Text delivered to OutputTarget but `PasteBackend::paste()` was intentionally
    /// skipped (WaitAndType mode). `text` is the transcribed payload — not an i18n key.
    /// Subscribers (Pill-Bar, Story A3) use this to display the transcription for
    /// manual confirmation before pasting.
    RecordingDelivered { ts_ms: u64, text: String },
    /// RMS audio level tap for Pill-Bar waveform (Shell-subscriber accumulates
    /// into 64-bin ring buffer; not forwarded to main WebView by EventMirror).
    /// `rms` is 0.0..=1.0 (same range as `AudioEvent::Level`).
    AudioLevel { rms: f32, ts_ms: u64 },
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
/// Sized for the Phase-1 baseline (hotkey/recording lifecycle, ~5 events per
/// PTT cycle) plus the Story-9.6 `Event::AudioLevel` stream (~15.6 Hz during
/// active recording). 256 slots tolerate ~16 s of subscriber back-pressure
/// before the slowest consumer sees `RecvError::Lagged`. Phase-2 backlog item
/// `eventbus-topology-split` will move the high-frequency stream to its own
/// channel; until then, this capacity is the load-bearing buffer.
pub const DEFAULT_EVENT_BUS_CAPACITY: usize = 256;

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
