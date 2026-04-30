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
        /// Mode active when the session was started. `on_release` and the second
        /// Toggle press route on this snapshot — not on the current
        /// `Orchestrator::mode` value — so a settings-driven mode change while a
        /// session is active does not split press-time and release-time semantics.
        press_mode: RecordingMode,
    },
}

/// AutoStop hard-cap: the longest a single AutoStop session may run before the
/// pipeline is aborted with `error.recording.timeout`. Default 60s; user-
/// configurable threshold tracked in deferred-work.md (A1-D5 follow-up).
const MAX_RECORDING_DURATION_SECS: u64 = 60;

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

    /// `true` if no session is active. Useful for test assertions and for shell
    /// state-pull UIs (tray, status bar). Cheap — single Mutex read.
    pub async fn is_idle(&self) -> bool {
        matches!(*self.session_state.lock().await, SessionState::Idle)
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
        // Single critical section for the Recording-state branch: dispatching on
        // press_mode (snapshotted at session start) under the same lock guard
        // that does mem::replace, so a concurrent on_press cannot observe a
        // transient Idle window between check and replace.
        {
            let mut state = self.session_state.lock().await;
            if let SessionState::Recording { press_mode, .. } = *state {
                if press_mode == RecordingMode::Toggle {
                    let prev = std::mem::replace(&mut *state, SessionState::Idle);
                    drop(state); // release before drop(capture_handle)
                    if let SessionState::Recording { capture_handle, pipeline_task, .. } = prev {
                        self.event_bus.emit(Event::RecordingStopped { ts_ms: self.clock.now_ms() });
                        drop(capture_handle);
                        drop(pipeline_task);
                    }
                } else {
                    tracing::debug!(
                        "on_press called while recording in {:?}; discarding (key-repeat-guard)",
                        press_mode
                    );
                }
                return;
            }
        } // lock released

        // Idle: snapshot current mode at press-time and start a new session.
        let press_mode = self.mode.read().await.clone();

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

        // AutoStop needs session_state to clean up after pipeline (ADR-0012 Amendment 1
        // + AC-5 amendment: cleanup runs unconditionally on every pipeline-task exit
        // path, before any deliver, so error/empty results do not leak the
        // capture_handle and pin the audio source open.
        let session_state_for_autostop = if press_mode == RecordingMode::AutoStop {
            Some(Arc::clone(&self.session_state))
        } else {
            None
        };

        let pipeline_task = tokio::spawn(async move {
            let pipeline = async {
                let mut vad_guard = vad.lock().await;
                let r = run_capture_session(rx, &mut **vad_guard, &manifest, &registry).await;
                drop(vad_guard);
                r
            };

            // AutoStop hard-cap: prevent runaway recording when VAD never reports
            // SpeechEnd (continuous noise above threshold). Other modes have
            // explicit user-driven termination, so no timeout is wrapped.
            let result = if press_mode == RecordingMode::AutoStop {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(MAX_RECORDING_DURATION_SECS),
                    pipeline,
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => {
                        error_emitter
                            .emit_error("error.recording.timeout", clock.now_ms())
                            .await;
                        Ok(None)
                    }
                }
            } else {
                pipeline.await
            };

            // Extract delivery text, if any. Non-Text variants and empty results
            // skip delivery; errors emit a toast.
            let text_to_deliver = match result {
                Ok(Some(StageData::Text(t))) => Some(t),
                Ok(Some(_)) => None, // Audio variant not expected at pipeline output
                Ok(None) => None,    // channel closed before any speech (accidental press)
                Err(e) => {
                    error_emitter
                        .emit_error(
                            e.user_message.as_deref().unwrap_or("error.internal"),
                            clock.now_ms(),
                        )
                        .await;
                    None
                }
            };

            // AutoStop cleanup runs BEFORE any deliver (ADR-0012 Amendment 1) and
            // unconditionally across success/empty/error paths so the capture
            // handle is always released. Race-safety: if on_release fires
            // concurrently (mode change mid-session), mem::replace on a
            // non-Recording state is a no-op under the same Mutex.
            if let Some(state_arc) = session_state_for_autostop {
                let mut st = state_arc.lock().await;
                if let SessionState::Recording { capture_handle, .. } =
                    std::mem::replace(&mut *st, SessionState::Idle)
                {
                    drop(capture_handle);
                }
            }

            if let Some(text) = text_to_deliver {
                match registry.output(&output_target_id) {
                    Some(target) => {
                        if let Err(e) = target.deliver(&text).await {
                            error_emitter
                                .emit_error(
                                    e.user_message.as_deref().unwrap_or("error.internal"),
                                    clock.now_ms(),
                                )
                                .await;
                        } else if press_mode == RecordingMode::WaitAndType {
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

            // Pipeline processing finished — emit RecordingCompleted regardless of
            // outcome so "system idle" subscribers (tray, Phase-2 progress UI) can
            // return to idle state. Distinct from RecordingStopped (audio-capture
            // termination) per the recording-lifecycle contract on `Event`.
            event_bus.emit(Event::RecordingCompleted { ts_ms: clock.now_ms() });
        });

        // Re-acquire state lock to transition to Recording.
        let mut state = self.session_state.lock().await;
        *state = SessionState::Recording { capture_handle, pipeline_task, press_mode };
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
        // Single critical section: dispatch on press_mode (snapshotted at session
        // start) under one lock guard. Avoids the drop+re-acquire race where a
        // concurrent on_press could observe transient Idle and start a new
        // session that this release would then destroy.
        let mut state = self.session_state.lock().await;

        match &*state {
            SessionState::Recording { press_mode: RecordingMode::Toggle, .. }
            | SessionState::Recording { press_mode: RecordingMode::AutoStop, .. } => {
                tracing::debug!(
                    "on_release while recording in non-release-driven mode; no-op"
                );
                return;
            }
            SessionState::Idle => {
                tracing::debug!("on_release called while idle; discarding (stray-release)");
                return;
            }
            SessionState::Recording { .. } => {
                // Hold or WaitAndType: fall through to the cleanup below.
            }
        }

        let prev = std::mem::replace(&mut *state, SessionState::Idle);
        drop(state); // release lock before drop(capture_handle)

        if let SessionState::Recording { capture_handle, pipeline_task, .. } = prev {
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
