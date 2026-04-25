use std::sync::Arc;

use klarvo_core::audio::{AudioEvent, AudioSource, CaptureConfig, CaptureHandle,
    DEFAULT_AUDIOEVENT_CAPACITY};
use klarvo_core::audio::vad::VadProvider;
use klarvo_core::event::{Event, EventBus};
use klarvo_core::event::emitter::ErrorEmitter;
use klarvo_core::manifest::PipelineManifest;
use klarvo_core::output::PasteBackend;
use klarvo_core::pipeline::{run_capture_session, StageData};
use klarvo_core::registry::PluginRegistry;
use klarvo_core::time::Clock;

/// State of the push-to-talk session.
///
/// Invariants:
/// - `Idle`: no active audio capture, no pipeline task running.
/// - `Recording`: exactly one `CaptureHandle` + one spawned pipeline task are live.
///   Dropping `CaptureHandle` signals the audio task to stop, which closes the
///   broadcast channel, which unblocks `run_capture_session`.
enum SessionState {
    Idle,
    Recording {
        capture_handle: CaptureHandle,
        /// Phase-2 Toggle-Mode revisit: graceful await/abort on App-Exit
        /// (ADR-0012 Open-Questions §Orchestrator-Shutdown-bei-App-Exit).
        pipeline_task: tokio::task::JoinHandle<()>,
    },
}

/// Coordinates the 7-Step Push-to-Talk cycle.
///
/// All dependencies are injected via `Arc`-wrapped trait objects for DI-mockability
/// in unit tests (ADR-0012 §SD-5).
///
/// `SessionOrchestrator` is `Send + Sync` — all fields are `Arc<…>`.
pub struct SessionOrchestrator {
    registry: Arc<PluginRegistry>,
    manifest: Arc<PipelineManifest>,
    /// Mutex-wrapped so `start()` (which takes `&mut self`) can be called from `&self`.
    audio_source: Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>,
    /// Registry ID of the configured output target (e.g. `"clipboard"`).
    output_target_id: String,
    paste_backend: Arc<dyn PasteBackend>,
    error_emitter: Arc<dyn ErrorEmitter>,
    clock: Arc<dyn Clock>,
    /// Mutex-wrapped so `run_capture_session` (which takes `&mut dyn VadProvider`) can be
    /// called from a shared reference.
    vad: Arc<tokio::sync::Mutex<Box<dyn VadProvider>>>,
    event_bus: Arc<EventBus>,
    session_state: Arc<tokio::sync::Mutex<SessionState>>,
}

impl SessionOrchestrator {
    /// Construct a new `SessionOrchestrator` with fully injected dependencies.
    pub fn new(
        registry: Arc<PluginRegistry>,
        manifest: Arc<PipelineManifest>,
        audio_source: Arc<tokio::sync::Mutex<Box<dyn AudioSource>>>,
        output_target_id: String,
        paste_backend: Arc<dyn PasteBackend>,
        error_emitter: Arc<dyn ErrorEmitter>,
        clock: Arc<dyn Clock>,
        vad: Arc<tokio::sync::Mutex<Box<dyn VadProvider>>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            registry,
            manifest,
            audio_source,
            output_target_id,
            paste_backend,
            error_emitter,
            clock,
            vad,
            event_bus,
            session_state: Arc::new(tokio::sync::Mutex::new(SessionState::Idle)),
        }
    }

    /// Begin a push-to-talk recording session (Step 1).
    ///
    /// If already recording (key-repeat OS event), the call is silently discarded.
    /// Errors from `AudioSource::start` are emitted via `ErrorEmitter`; the state
    /// remains `Idle` on failure.
    ///
    /// Error i18n keys emitted: `error.audio.start_failed`.
    pub async fn on_press(&self) {
        // Check for key-repeat without holding the lock across async boundaries.
        {
            let state = self.session_state.lock().await;
            if matches!(*state, SessionState::Recording { .. }) {
                tracing::debug!("on_press called while recording; discarding (key-repeat-guard)");
                return;
            }
        } // lock released here

        let (tx, rx) = tokio::sync::broadcast::channel::<AudioEvent>(DEFAULT_AUDIOEVENT_CAPACITY);
        let config = CaptureConfig {
            sample_rate: 16_000,
            channels: 1,
            events: tx,
        };

        let capture_handle = match self.audio_source.lock().await.start(config).await {
            Ok(h) => h,
            Err(_) => {
                self.error_emitter
                    .emit_error("error.audio.start_failed", self.clock.now_ms())
                    .await;
                return;
            }
        };

        self.event_bus.emit(Event::RecordingStarted { ts_ms: self.clock.now_ms() });

        // Clone Arcs for the pipeline task.
        let registry = Arc::clone(&self.registry);
        let manifest = Arc::clone(&self.manifest);
        let vad = Arc::clone(&self.vad);
        let output_target_id = self.output_target_id.clone();
        let paste_backend = Arc::clone(&self.paste_backend);
        let error_emitter = Arc::clone(&self.error_emitter);
        let clock = Arc::clone(&self.clock);
        let event_bus = Arc::clone(&self.event_bus);

        let pipeline_task = tokio::spawn(async move {
            let mut vad_guard = vad.lock().await;
            let result =
                run_capture_session(rx, &mut **vad_guard, &manifest, &registry).await;
            drop(vad_guard);

            match result {
                Ok(Some(stage_data)) => {
                    let text = match stage_data {
                        StageData::Text(t) => t,
                        // Audio variant not expected at pipeline output — swallow silently.
                        _ => {
                            event_bus.emit(Event::RecordingCompleted { ts_ms: clock.now_ms() });
                            return;
                        }
                    };
                    match registry.output(&output_target_id) {
                        Some(target) => {
                            if let Err(e) = target.deliver(&text).await {
                                error_emitter
                                    .emit_error(
                                        e.user_message.as_deref().unwrap_or("error.internal"),
                                        clock.now_ms(),
                                    )
                                    .await;
                            } else if let Err(e) = paste_backend.paste().await {
                                error_emitter
                                    .emit_error(
                                        e.user_message.as_deref().unwrap_or("error.internal"),
                                        clock.now_ms(),
                                    )
                                    .await;
                            }
                        }
                        None => {
                            error_emitter
                                .emit_error(
                                    "error.config.output_target_not_found",
                                    clock.now_ms(),
                                )
                                .await;
                        }
                    }
                }
                // Channel closed before any speech — accidental hotkey trigger, swallow silently.
                Ok(None) => {}
                Err(e) => {
                    error_emitter
                        .emit_error(
                            e.user_message.as_deref().unwrap_or("error.internal"),
                            clock.now_ms(),
                        )
                        .await;
                }
            }

            // Pipeline processing finished — emit RecordingCompleted regardless of
            // outcome so "system idle" subscribers (tray, Phase-2 progress UI) can
            // return to idle state. Distinct from RecordingStopped (audio-capture
            // termination) per the recording-lifecycle contract on `Event`.
            event_bus.emit(Event::RecordingCompleted { ts_ms: clock.now_ms() });
        });

        // Re-acquire state lock to transition to Recording.
        let mut state = self.session_state.lock().await;
        *state = SessionState::Recording { capture_handle, pipeline_task };
    }

    /// End the push-to-talk recording session (Step 4).
    ///
    /// Drops the `CaptureHandle`, closing the broadcast channel and signalling
    /// the audio task to stop. The pipeline task continues asynchronously;
    /// `on_release` returns immediately (non-blocking).
    ///
    /// If called without a prior `on_press` (stray release), the call is a no-op.
    pub async fn on_release(&self) {
        let mut state = self.session_state.lock().await;
        let prev = std::mem::replace(&mut *state, SessionState::Idle);
        drop(state); // release lock before drop(capture_handle)

        match prev {
            SessionState::Idle => {
                tracing::debug!("on_release called while idle; discarding (stray-release)");
            }
            SessionState::Recording { capture_handle, pipeline_task } => {
                self.event_bus.emit(Event::RecordingStopped { ts_ms: self.clock.now_ms() });
                // Step 4: drop CaptureHandle → broadcast sender closes →
                // run_capture_session's receiver gets RecvError::Closed.
                drop(capture_handle);
                // Dropping JoinHandle detaches the task (does NOT abort it) —
                // pipeline_task runs to completion to deliver results or emit errors.
                drop(pipeline_task);
            }
        }
    }
}
