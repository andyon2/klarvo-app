use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use klarvo_core::audio::{AudioEvent, AudioSource, CaptureConfig, CaptureHandle,
    DEFAULT_AUDIOEVENT_CAPACITY};
use klarvo_core::audio::vad::VadProvider;
use klarvo_core::event::{Event, EventBus};
use klarvo_core::event::emitter::ErrorEmitter;
use klarvo_core::history::{HistoryBackend, NewHistoryEntry};
use klarvo_core::manifest::PipelineManifest;
use klarvo_core::output::{FocusCapture, PasteBackend};
use klarvo_core::pipeline::{run_capture_session, StageData};
use klarvo_core::recording::{HotkeySlot, RecordingMode};
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
        /// Story 9.6 AC-2 + Code-Review-D3=A: level-tap task forwards
        /// `AudioEvent::Level` onto the EventBus as `Event::AudioLevel`.
        /// JoinHandle stored so `shutdown` can abort it deterministically;
        /// natural-drop paths (`on_press` Toggle-stop, `on_release`) rely
        /// on the broadcast channel closing once `capture_handle` drops.
        level_tap_task: tokio::task::JoinHandle<()>,
        /// Mode active when the session was started. `on_release` and the second
        /// Toggle press route on this snapshot — not on the current
        /// `Orchestrator::mode` value — so a settings-driven mode change while a
        /// session is active does not split press-time and release-time semantics.
        press_mode: RecordingMode,
        /// Slot that owns this session. `on_press` / `on_release` from a different
        /// slot are discarded (D-1 mutual-exclusion across slots — ADR-0012
        /// Amendment 3 / Story 8.1 Code-Review-Closure 2026-05-05). Without this,
        /// a Slot-Two release during a Slot-One Hold-recording would terminate
        /// Slot-One, and a Slot-Two press during a Slot-One Toggle-recording
        /// would toggle Slot-One off.
        owner_slot: HotkeySlot,
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
    /// Active recording mode for slot 1; updated by the shell on settings change (ADR-0012 Amendment 1).
    mode: Arc<tokio::sync::RwLock<RecordingMode>>,
    /// Active recording mode for slot 2 (Story 8.1 / ADR-0012 Amendment 2).
    mode_slot2: Arc<tokio::sync::RwLock<RecordingMode>>,
    focus_capture: Arc<dyn FocusCapture>,
    history_backend: Arc<dyn HistoryBackend>,
    /// Generation counter incremented on every new session start. The
    /// pipeline task captures its session_id at spawn-time and self-filters
    /// `Event::LivePreviewChunk` emit when a newer session has superseded it
    /// (Toggle stop-then-start race: without this guard, stale LP-text from
    /// the old session would overwrite the freshly-shown pill-bar of the new
    /// session). Delivery itself is unaffected — the user still gets their
    /// transcript pasted; only the visual feedback is suppressed for the
    /// detached old task.
    session_counter: Arc<AtomicU64>,
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
        mode_slot2: Arc<tokio::sync::RwLock<RecordingMode>>,
        focus_capture: Arc<dyn FocusCapture>,
        history_backend: Arc<dyn HistoryBackend>,
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
            mode_slot2,
            focus_capture,
            history_backend,
            session_counter: Arc::new(AtomicU64::new(0)),
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
    /// - **AutoStop**: starts recording; VAD SpeechEnd auto-stops via pipeline cleanup.
    ///
    /// `slot` identifies which hotkey fired; mode is looked up per slot (Amendment 3).
    ///
    /// Mutual-Exclusion (D-1, ADR-0012 Amendment 3): if a Recording session is
    /// active and `slot != owner_slot`, the press is discarded — modus-unabhängig.
    /// Same-slot Toggle press stops the session; same-slot Hold/AutoStop/WaitAndType
    /// press during recording is treated as key-repeat and discarded.
    pub async fn on_press(&self, slot: HotkeySlot) {
        // Capture focus before any recording-state check or audio start (AC-2 1a).
        // Captured even on key-repeat (early-return discards it without side effect).
        let captured_focus = self.focus_capture.capture();

        // Single critical section for the Recording-state branch: dispatching on
        // press_mode (snapshotted at session start) under the same lock guard
        // that does mem::replace, so a concurrent on_press cannot observe a
        // transient Idle window between check and replace.
        {
            let mut state = self.session_state.lock().await;
            if let SessionState::Recording { press_mode, owner_slot, .. } = *state {
                if slot != owner_slot {
                    tracing::debug!(
                        "on_press({:?}) while recording owned by {:?}; discarding (D-1 cross-slot mutual-exclusion)",
                        slot, owner_slot
                    );
                    return;
                }
                if press_mode == RecordingMode::Toggle {
                    let prev = std::mem::replace(&mut *state, SessionState::Idle);
                    drop(state); // release before drop(capture_handle)
                    if let SessionState::Recording {
                        capture_handle,
                        pipeline_task,
                        level_tap_task,
                        ..
                    } = prev {
                        self.event_bus.emit(Event::RecordingStopped { ts_ms: self.clock.now_ms() });
                        drop(capture_handle);
                        drop(pipeline_task);
                        drop(level_tap_task);
                    }
                } else {
                    tracing::debug!(
                        "on_press({:?}) called while recording in {:?}; discarding (key-repeat-guard)",
                        slot, press_mode
                    );
                }
                return;
            }
        } // lock released

        // Idle: snapshot mode for the active slot at press-time (ADR-0012 Amendment 2).
        let press_mode = match slot {
            HotkeySlot::One => self.mode.read().await.clone(),
            HotkeySlot::Two => self.mode_slot2.read().await.clone(),
        };

        let (tx, rx) = tokio::sync::broadcast::channel::<AudioEvent>(DEFAULT_AUDIOEVENT_CAPACITY);
        // Story 9.6 AC-2: second subscriber for Pill-Bar level tap; created
        // before tx moves into CaptureConfig so no Level event is missed.
        let level_rx = tx.subscribe();
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

        // Story 9.6 AC-2: spawn level-tap task that forwards `AudioEvent::Level`
        // values onto the EventBus as `Event::AudioLevel` for the Pill-Bar overlay.
        // Code-Review-D3=A: JoinHandle stored on `SessionState::Recording`; aborted
        // explicitly in `shutdown` for deterministic teardown. Other exit paths
        // close the broadcast channel via `drop(capture_handle)`, which the loop
        // observes as `RecvError::Closed`.
        let level_tap_task = {
            let event_bus_level = Arc::clone(&self.event_bus);
            tokio::spawn(async move {
                let mut rx = level_rx;
                loop {
                    match rx.recv().await {
                        Ok(AudioEvent::Level { rms, ts_ms }) => {
                            // Code-Review-P2: NaN/Inf at source must never reach the
                            // Pill-Bar payload (would serialize as JSON `null` and
                            // break canvas math downstream). Replace with 0.0.
                            let safe_rms = if rms.is_finite() { rms } else { 0.0 };
                            event_bus_level.emit(Event::AudioLevel { rms: safe_rms, ts_ms });
                        }
                        Ok(AudioEvent::Samples { .. }) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "level-tap lagged; skipped AudioLevel events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            })
        };

        // Clone Arcs for the pipeline task.
        let registry = Arc::clone(&self.registry);
        let manifest = Arc::clone(&self.manifest);
        let vad = Arc::clone(&self.vad);
        let output_target_id = self.output_target_id.clone();
        let paste_backend = Arc::clone(&self.paste_backend);
        let error_emitter = Arc::clone(&self.error_emitter);
        let clock = Arc::clone(&self.clock);
        let event_bus = Arc::clone(&self.event_bus);
        let focus_capture = Arc::clone(&self.focus_capture);
        let history_backend = Arc::clone(&self.history_backend);

        // AutoStop needs session_state to clean up after pipeline (ADR-0012 Amendment 1
        // + AC-5 amendment: cleanup runs unconditionally on every pipeline-task exit
        // path, before any deliver, so error/empty results do not leak the
        // capture_handle and pin the audio source open.
        let session_state_for_autostop = if press_mode == RecordingMode::AutoStop {
            Some(Arc::clone(&self.session_state))
        } else {
            None
        };

        // Snapshot a session-generation id for the LivePreviewChunk stale-emit
        // guard (Story 11.4 Pass-2 EC-2): if a NEWER session starts before this
        // pipeline_task finishes, the global counter advances past our snapshot
        // and the LP-emit self-suppresses so the new session's pill-bar isn't
        // overwritten by stale text. fetch_add returns the previous value, so
        // the freshly-captured id is `prev + 1`.
        let session_id = self.session_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let session_counter_for_task = Arc::clone(&self.session_counter);

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

            // Re-D1 (Re-Review-Closure 4f0e0f7): Hold/Toggle/WaitAndType emit
            // `RecordingStopped` from `on_release` / Toggle-inline-stop — the
            // user-driven termination. AutoStop's audio capture ends here at
            // `pipeline.await` resolution (whether VAD SpeechEnd or hard-cap
            // timeout), so emit `Stopped` here to keep the 3-state lifecycle
            // (Started → Stopped → Completed) uniform across modes. Subscribers
            // (tray state-pull, A3 Pill-Bar) can rely on the contract.
            if press_mode == RecordingMode::AutoStop {
                event_bus.emit(Event::RecordingStopped { ts_ms: clock.now_ms() });
            }

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

            // Always-restore policy (Review-Closure 2026-05-03 Decision D1=A): focus is
            // restored on every post-capture exit path so the user is never stranded on
            // Klarvo's overlay after a failure (paste error, output-target-not-found,
            // empty pipeline, deliver error). WaitAndType is the one branch that restores
            // *early* — before emitting RecordingDelivered — so the target window is
            // focused by the time Pill-Bar handles the event.
            let mut focus_restored = false;

            // LivePreview: emit before delivery so the Pill Bar shows the text
            // before focus shifts to the target window (AC-2 ordering invariant).
            //
            // Pass-2 guards (code-review 2026-05-08):
            //   EC-1: skip empty text — empty STT result would otherwise resize
            //   the pill-bar to LP-mode and display nothing, visually misleading.
            //   EC-2: skip if session has been superseded — Toggle stop-then-start
            //   race spawns a new session before this detached pipeline_task
            //   finishes; without the guard, stale LP-text would overwrite the
            //   new session's pill-bar.
            let session_active =
                session_counter_for_task.load(Ordering::SeqCst) == session_id;
            if let Some(ref text) = text_to_deliver {
                if session_active && !text.is_empty() {
                    event_bus.emit(Event::LivePreviewChunk {
                        text: text.clone(),
                        ts_ms: clock.now_ms(),
                    });
                }
            }

            if let Some(ref text) = text_to_deliver {
                if session_active && !text.is_empty() {
                    event_bus.emit(Event::CleanupDone {
                        text: text.clone(),
                        ts_ms: clock.now_ms(),
                    });
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
                        } else {
                            // History-Persistence: fail-soft — delivery must not be blocked
                            // by a storage write failure.
                            let history_entry = NewHistoryEntry {
                                text: text.clone(),
                                raw_text: None,
                                style: manifest_stt_style(&manifest),
                                language: String::new(),
                                app_name: None,
                                created_at: klarvo_core::history::wall_clock_iso8601(),
                                uuid: None,
                                device_id: None,
                                plugin_id: manifest_stt_plugin(&manifest),
                                manifest_version: None,
                                output_language: None,
                            };
                            if let Err(e) = history_backend.append(&history_entry).await {
                                tracing::warn!(
                                    error = %e.message,
                                    "history write failed; continuing without persistence"
                                );
                            }

                            if press_mode == RecordingMode::WaitAndType {
                                focus_capture.restore(captured_focus);
                                focus_restored = true;
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
                            // Hold/Toggle/AutoStop success path falls through to end-restore.
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

            if !focus_restored {
                focus_capture.restore(captured_focus);
            }

            // Pipeline processing finished — emit RecordingCompleted regardless of
            // outcome so "system idle" subscribers (tray, Phase-2 progress UI) can
            // return to idle state. Distinct from RecordingStopped (audio-capture
            // termination) per the recording-lifecycle contract on `Event`.
            event_bus.emit(Event::RecordingCompleted { ts_ms: clock.now_ms() });
        });

        // Re-acquire state lock to transition to Recording.
        let mut state = self.session_state.lock().await;
        *state = SessionState::Recording {
            capture_handle,
            pipeline_task,
            level_tap_task,
            press_mode,
            owner_slot: slot,
        };
    }

    /// Abort the current recording session from the UI (Pill-Bar abort button).
    ///
    /// Differs from `shutdown`: emits `Event::RecordingAborted` after teardown
    /// (so the Pill-Bar overlay fades). Does NOT emit `RecordingStopped` (no
    /// meaningful audio boundary crossed). Idempotent: second call observes Idle
    /// and no-ops.
    ///
    /// Pipeline task is hard-cancelled (abort, not graceful drop) — audio buffer
    /// is discarded without STT call or paste (per UX-Spec §2.5 Abort Affordance).
    pub async fn cancel_recording(&self) {
        let mut state = self.session_state.lock().await;
        let prev = std::mem::replace(&mut *state, SessionState::Idle);
        drop(state); // release lock before async work
        if let SessionState::Recording {
            capture_handle,
            pipeline_task,
            level_tap_task,
            ..
        } = prev {
            pipeline_task.abort();
            level_tap_task.abort();
            let _ = pipeline_task.await; // JoinError::is_cancelled() expected
            let _ = level_tap_task.await;
            drop(capture_handle);
            self.event_bus.emit(Event::RecordingAborted { ts_ms: self.clock.now_ms() });
        }
    }

    /// Abort any active pipeline and return to `Idle`. Called on App-Exit.
    ///
    /// Differs from `on_release`: calls `pipeline_task.abort()` (hard-cancel) instead
    /// of dropping the handle (which only detaches). Awaits the JoinHandle after abort
    /// so the teardown is deterministic (matches the Outcome contract from Story 2.A.D3).
    /// Does NOT emit `RecordingStopped` — App-Exit is a forced-teardown path, not a
    /// user-driven stop.
    ///
    /// Idempotent and concurrent-safe: callers serialize via the `session_state` mutex;
    /// the second caller observes `Idle` and no-ops.
    pub async fn shutdown(&self) {
        let mut state = self.session_state.lock().await;
        let prev = std::mem::replace(&mut *state, SessionState::Idle);
        drop(state);
        if let SessionState::Recording {
            capture_handle,
            pipeline_task,
            level_tap_task,
            ..
        } = prev {
            pipeline_task.abort();
            // Code-Review-D3=A: abort the level-tap task explicitly so shutdown
            // does not depend on the implicit channel-close cascade. The task
            // is short-lived per-session, but abort makes teardown ordering
            // deterministic and matches the discipline applied to pipeline_task.
            level_tap_task.abort();
            // Await JoinHandle so shutdown returns only after the task actually observed
            // the cancel. JoinError::is_cancelled() is the expected outcome; Ok(()) means
            // the task completed before the abort signal landed — both are acceptable.
            let _ = pipeline_task.await;
            let _ = level_tap_task.await;
            drop(capture_handle);
        }
    }

    /// End the push-to-talk recording session (Step 4).
    ///
    /// Behaviour varies by `RecordingMode` (ADR-0012 Amendment 1):
    /// - **Hold/WaitAndType**: drops `CaptureHandle`, closing the broadcast channel.
    /// - **Toggle**: no-op (stop triggered by second `on_press`).
    /// - **AutoStop**: no-op (stop triggered by VAD SpeechEnd in pipeline).
    ///
    /// `slot` identifies which hotkey released. If `slot != owner_slot`, the
    /// release is discarded (D-1 cross-slot mutual-exclusion — ADR-0012 Amendment 3).
    /// Otherwise the press_mode snapshotted in `SessionState::Recording` governs
    /// stop-semantics. Stray release calls (state=Idle for any mode) are a silent no-op.
    pub async fn on_release(&self, slot: HotkeySlot) {
        // Single critical section: dispatch on press_mode (snapshotted at session
        // start) under one lock guard. Avoids the drop+re-acquire race where a
        // concurrent on_press could observe transient Idle and start a new
        // session that this release would then destroy.
        let mut state = self.session_state.lock().await;

        match &*state {
            SessionState::Recording { owner_slot, .. } if *owner_slot != slot => {
                tracing::debug!(
                    "on_release({:?}) while recording owned by {:?}; discarding (D-1 cross-slot mutual-exclusion)",
                    slot, owner_slot
                );
                return;
            }
            SessionState::Recording { press_mode: RecordingMode::Toggle, .. }
            | SessionState::Recording { press_mode: RecordingMode::AutoStop, .. } => {
                tracing::debug!(
                    "on_release({:?}) while recording in non-release-driven mode; no-op",
                    slot
                );
                return;
            }
            SessionState::Idle => {
                tracing::debug!("on_release({:?}) called while idle; discarding (stray-release)", slot);
                return;
            }
            SessionState::Recording { .. } => {
                // Hold or WaitAndType + same slot: fall through to the cleanup below.
            }
        }

        let prev = std::mem::replace(&mut *state, SessionState::Idle);
        drop(state); // release lock before drop(capture_handle)

        if let SessionState::Recording {
            capture_handle,
            pipeline_task,
            level_tap_task,
            ..
        } = prev {
            self.event_bus.emit(Event::RecordingStopped { ts_ms: self.clock.now_ms() });
            // Level-tap-task self-terminates once the broadcast channel closes
            // (drop(capture_handle) below). Detach the JoinHandle so the closure
            // happens asynchronously in the background.
            drop(level_tap_task);
            // Step 4: drop CaptureHandle → broadcast sender closes →
            // run_capture_session's receiver gets RecvError::Closed.
            drop(capture_handle);
            // Dropping JoinHandle detaches the task (does NOT abort it) —
            // pipeline_task runs to completion to deliver results or emit errors.
            drop(pipeline_task);
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers for History-Persistence (AC-5)
// ---------------------------------------------------------------------------

/// Extract the STT plugin_id from the first Stt stage in the manifest.
fn manifest_stt_plugin(manifest: &PipelineManifest) -> Option<String> {
    use klarvo_core::pipeline::PipelineStageType;
    manifest.pipeline.stages.iter().find_map(|s| match s {
        PipelineStageType::Stt { plugin_id } => Some(plugin_id.clone()),
        _ => None,
    })
}

/// Extract the cleanup style name from the first Cleanup stage; falls back to "verbatim".
fn manifest_stt_style(manifest: &PipelineManifest) -> String {
    use klarvo_core::pipeline::PipelineStageType;
    manifest
        .pipeline
        .stages
        .iter()
        .find_map(|s| match s {
            PipelineStageType::Cleanup { plugin_id } => Some(plugin_id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "verbatim".to_string())
}
