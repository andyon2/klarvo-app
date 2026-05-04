//! Shell-side event bridge between `klarvo-core` and the Tauri frontend.
//!
//! Two types live here:
//!
//! - [`TauriErrorEmitter`] — implements the `klarvo_core::event::ErrorEmitter` trait,
//!   forwarding errors to the frontend via `app_handle.emit("app.error", ...)`.
//!   Emits [`"app.error"`] — ADR-0009 §SD-1.
//!   Key-Forwarding only — does not translate or validate i18n keys.
//!   Frontend resolves keys via its i18n-stack (ADR-0009 §SD-2).
//!
//! - [`EventMirror`] — subscribes on the `klarvo_core::event::EventBus` broadcast
//!   receiver and re-emits each `Event` variant as a Tauri frontend event using the
//!   `<domain>.<event>` wire-name convention (ADR-0002, `reference_tauri_specta_rc24_event_name`).
//!   Payload-forwarding only — no key-translation. Frontend-listener resolves all
//!   i18n keys via JS i18n-stack (memory/project_i18n_core_contract).

use async_trait::async_trait;
use serde::Serialize;
use tauri::Emitter as _;
use tokio::sync::broadcast;

use klarvo_core::event::{ErrorEmitter, Event};

// ---------------------------------------------------------------------------
// Shared payload
// ---------------------------------------------------------------------------

/// Payload for the `"app.error"` Tauri frontend event.
///
/// Used by both [`TauriErrorEmitter::emit_error`] and the `ErrorEmitted`
/// variant arm of [`EventMirror::mirror_event`] so the frontend always sees
/// the same shape on `"app.error"` regardless of emission path.
#[derive(Debug, Clone, Serialize)]
pub struct AppErrorEventPayload {
    pub key: String,
    pub ts_ms: u64,
}

// ---------------------------------------------------------------------------
// Payload structs for EventMirror
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct RecordingStartedPayload {
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct RecordingStoppedPayload {
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct RecordingCompletedPayload {
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineStageStartedPayload {
    stage_type: String,
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineStageCompletedPayload {
    stage_type: String,
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingDeliveredPayload {
    pub ts_ms: u64,
    pub text: String,
}

// ---------------------------------------------------------------------------
// TauriErrorEmitter
// ---------------------------------------------------------------------------

/// Shell implementation of [`ErrorEmitter`] that pushes errors to the Tauri
/// frontend via `app_handle.emit("app.error", ...)`.
///
/// Generic over `R: tauri::Runtime` so the same type works with `Wry` in
/// production and `MockRuntime` in tests (idiomatic Tauri v2 extension pattern).
///
/// Emits [`"app.error"`] — ADR-0009 §SD-1.
/// Key-Forwarding only — does not translate or validate i18n keys.
/// Frontend resolves keys via its i18n-stack (ADR-0009 §SD-2).
pub struct TauriErrorEmitter<R: tauri::Runtime> {
    app_handle: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> TauriErrorEmitter<R> {
    pub fn new(handle: tauri::AppHandle<R>) -> Self {
        Self { app_handle: handle }
    }
}

#[async_trait]
impl<R: tauri::Runtime> ErrorEmitter for TauriErrorEmitter<R> {
    async fn emit_error(&self, key: &str, ts_ms: u64) {
        let payload = AppErrorEventPayload { key: key.to_string(), ts_ms };
        if let Err(e) = self.app_handle.emit("app.error", &payload) {
            tracing::error!(error = %e, key = key, "failed to emit app.error event to frontend");
        }
    }
}

// ---------------------------------------------------------------------------
// EventMirror
// ---------------------------------------------------------------------------

/// Subscribes on the [`EventBus`] broadcast channel and re-emits each
/// [`Event`] variant as a Tauri frontend event.
///
/// Wire-names follow `<domain>.<event>` dot-notation per
/// `reference_tauri_specta_rc24_event_name` and ADR-0002.
///
/// Generic over `R: tauri::Runtime` so the same type works with `Wry` in
/// production and `MockRuntime` in tests.
///
/// Payload-forwarding only — no key-translation. Frontend-listener resolves all
/// i18n keys via JS i18n-stack (memory/project_i18n_core_contract).
pub struct EventMirror<R: tauri::Runtime> {
    app_handle: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> EventMirror<R> {
    pub fn new(handle: tauri::AppHandle<R>) -> Self {
        Self { app_handle: handle }
    }

    /// Spawn a background task that drains `receiver` and mirrors each event to
    /// the Tauri frontend. Returns immediately; the task runs until the channel
    /// is closed.
    ///
    /// Uses `tauri::async_runtime::spawn` (managed Tauri runtime) per
    /// `memory/project_shell_runtime_model` (single tokio-Runtime in shell scope).
    pub fn start(self, mut receiver: broadcast::Receiver<Event>) {
        tauri::async_runtime::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => self.mirror_event(event),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "EventMirror lagged; skipped core events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    fn mirror_event(&self, event: Event) {
        let result = match event {
            Event::RecordingStarted { ts_ms } => self
                .app_handle
                .emit("recording.started", &RecordingStartedPayload { ts_ms }),
            Event::RecordingStopped { ts_ms } => self
                .app_handle
                .emit("recording.stopped", &RecordingStoppedPayload { ts_ms }),
            Event::RecordingCompleted { ts_ms } => self
                .app_handle
                .emit("recording.completed", &RecordingCompletedPayload { ts_ms }),
            Event::PipelineStageStarted { stage_type, ts_ms } => self.app_handle.emit(
                "pipeline.stage_started",
                &PipelineStageStartedPayload { stage_type, ts_ms },
            ),
            Event::PipelineStageCompleted { stage_type, ts_ms } => self.app_handle.emit(
                "pipeline.stage_completed",
                &PipelineStageCompletedPayload { stage_type, ts_ms },
            ),
            // ErrorEmitted maps to "app.error" — same wire-name as TauriErrorEmitter.
            // Frontend uses a single listen("app.error", ...) for both paths (ADR-0009 §SD-1).
            Event::ErrorEmitted { error_key, ts_ms } => self
                .app_handle
                .emit("app.error", &AppErrorEventPayload { key: error_key, ts_ms }),
            Event::RecordingDelivered { ts_ms, text } => self
                .app_handle
                .emit("recording.delivered", &RecordingDeliveredPayload { ts_ms, text }),
            // AudioLevel is consumed by PillBar overlay only — not mirrored to main WebView.
            // High-frequency (~15.6Hz) and the main WebView has no consumer.
            Event::AudioLevel { .. } => return,
        };
        if let Err(e) = result {
            tracing::warn!(error = %e, "EventMirror failed to emit event to frontend");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use klarvo_core::event::ErrorEmitter;

    /// Compile-time check: TauriErrorEmitter<Wry> satisfies Send + Sync + 'static
    /// bounds required by Arc<dyn ErrorEmitter>. No runtime execution needed.
    ///
    /// Generic-over-Runtime means we verify bounds for the concrete production type.
    #[allow(dead_code)]
    fn _assert_error_emitter_send_sync<T: ErrorEmitter + Send + Sync + 'static>() {}

    #[allow(dead_code)]
    fn _check() {
        _assert_error_emitter_send_sync::<TauriErrorEmitter<tauri::Wry>>();
    }

    /// MockRuntime headless test: verifies `emit_error` completes without panic
    /// using `tauri::test::mock_app()` (Tauri v2 test helpers, `test` feature).
    ///
    /// Marked `#[ignore]` as a CI safety net: `mock_app()` may require a display
    /// context on some runners. Run explicitly via `cargo test -- --ignored` or
    /// `cargo xtask test-bridge-manual`.
    ///
    /// Delegate choice: MockRuntime compiles and links on this RC (tauri::test
    /// module gated behind `features = ["test"]` in [dev-dependencies]). The
    /// test body is non-empty; the `#[ignore]` is retained for CI safety only.
    #[tokio::test]
    #[ignore = "tauri::test::mock_app() may require display context — run manually or via `cargo xtask test-bridge-manual`"]
    async fn emit_error_does_not_panic_with_mock_runtime() {
        let app = tauri::test::mock_app();
        let handle = app.handle().clone();
        let emitter = TauriErrorEmitter::new(handle);
        // Should complete without panic; emit result is logged, not propagated.
        emitter.emit_error("error.test", 0).await;
    }
}
