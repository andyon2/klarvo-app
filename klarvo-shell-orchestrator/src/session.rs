use std::sync::Arc;

use klarvo_core::audio::{AudioEvent, AudioSource, CaptureConfig, CaptureHandle,
    DEFAULT_AUDIOEVENT_CAPACITY};
use klarvo_core::audio::vad::VadProvider;
use klarvo_core::event::{Event, EventBus};
use klarvo_core::event::emitter::ErrorEmitter;
use klarvo_core::manifest::PipelineManifest;
use klarvo_core::output::PasteBackend;
use klarvo_core::pipeline::{run_capture_session, StageData};
use klarvo_core::recording::RecordingMode;
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
/// `SessionOrchestrator` is `Send + Sync + Clone` — all fields are `Arc<…>`.
/// `Clone` produces a shallow copy sharing all internal state (Mutex-guarded session_state
/// is intentionally shared so hotkey callbacks and Tauri State access the same session).
#[derive(Clone)]
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
    /// Active recording mode; updated by the shell on settings change (ADR-0012 Amendment 1).
    mode: Arc<tokio::sync::RwLock<RecordingMode>>,
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
        mode: Arc<tokio::sync::RwLock<RecordingMode>>,
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
            mode,
        }
    }

    /// Begin a push-to-talk recording session (Step 1).
    ///
    /// Behaviour varies by `RecordingMode` (ADR-0012 Amendment 1):
    /// - **Hold/WaitAndType**: starts recording; `on_release` triggers stop.
    /// - **Toggle**: first press starts; second press stops (inline stop logic).
    ///   Key-repeat OS events (state=Recording, second press discarded for non-Toggle)
    ///   are routed to the Toggle-stop branch instead.
    /// - **AutoStop**: starts recording; VAD SpeechEnd auto-stops via pipeline cleanup.
    ///
    /// Errors from `AudioSource::start` are emitted via `ErrorEmitter`; state remains
    /// `Idle` on failure. i18n key emitted: `error.audio.start_failed`.
    pub async fn on_press(&self) {
        let mode = self.mode.read().await.clone();

        {
            let state = self.session_state.lock().await;
            if matches!(*state, SessionState::Recording { .. }) {
                if mode == RecordingMode::Toggle {
                    // Second Toggle-press: inline stop, same as on_release for Hold.
                    // Lock MUST be released before re-acquiring below (deadlock guard).
                    drop(state);
                    let mut st = self.session_state.lock().await;
                    let prev = std::mem::replace(&mut *st, SessionState::Idle);
                    drop(st);
                    if let SessionState::Recording { capture_handle, pipeline_task } = prev {
                        self.event_bus.emit(Event::RecordingStopped { ts_ms: self.clock.now_ms() });
                        drop(capture_handle);
                        drop(pipeline_task);
                    }
                    return;
                }
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

        // AutoStop needs session_state to clean up after pipeline delivery (ADR-0012 Amendment 1).
        let session_state_for_autostop = if mode == RecordingMode::AutoStop {
            Some(Arc::clone(&self.session_state))
        } else {
            None
        };

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

                    // AutoStop cleanup: capture_handle has not been dropped yet (on_release is
                    // a no-op in AutoStop mode). Drop it now to stop audio capture.
                    // Race-safety: if on_release fires concurrently, std::mem::replace finds
                    // no Recording state and is a no-op (Mutex guards exclusive access).
                    if let Some(state_arc) = session_state_for_autostop {
                        let mut st = state_arc.lock().await;
                        if let SessionState::Recording { capture_handle, .. } =
                            std::mem::replace(&mut *st, SessionState::Idle)
                        {
                            drop(capture_handle);
                        }
                    }

                    match registry.output(&output_target_id) {
                        Some(target) => {
                            if let Err(e) = target.deliver(&text).await {
                                error_emitter
                                    .emit_error(
                                        e.user_message.as_deref().unwrap_or("error.internal"),
                                        clock.now_ms(),
                                    )
                                    .await;
                            } else if mode == RecordingMode::WaitAndType {
                                // WaitAndType: skip paste, signal Pill-Bar (Story A3) instead.
                                event_bus.emit(Event::RecordingDelivered {
                                    ts_ms: clock.now_ms(),
                                    text: text.clone(),
                                });
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
    /// Behaviour varies by `RecordingMode` (ADR-0012 Amendment 1):
    /// - **Hold/WaitAndType**: drops `CaptureHandle`, closing the broadcast channel and
    ///   signalling the audio task to stop. Non-blocking.
    /// - **Toggle**: no-op when Recording (stop triggered by second `on_press`).
    /// - **AutoStop**: no-op when Recording (stop triggered by VAD SpeechEnd in pipeline).
    ///
    /// Stray release calls (state=Idle for any mode) are a silent no-op.
    pub async fn on_release(&self) {
        let mode = self.mode.read().await.clone();

        // Toggle and AutoStop: on_release is a no-op while recording
        // (Toggle stops via second on_press; AutoStop stops via VAD).
        if matches!(mode, RecordingMode::Toggle | RecordingMode::AutoStop) {
            let state = self.session_state.lock().await;
            if matches!(*state, SessionState::Recording { .. }) {
                tracing::debug!(
                    "on_release in {:?} mode while recording; no-op",
                    mode
                );
                return;
            }
        }

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
