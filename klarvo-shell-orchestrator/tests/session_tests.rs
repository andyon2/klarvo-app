#![allow(clippy::disallowed_methods)]

use std::sync::Arc;
use std::time::Duration;

use klarvo_core::audio::vad::VadDecision;
use klarvo_core::event::{Event, EventBus, DEFAULT_EVENT_BUS_CAPACITY};
use klarvo_core::manifest::parse_from_str as parse_manifest;
use klarvo_core::recording::{HotkeySlot, RecordingMode};
use klarvo_test_fixtures::{
    FakeClock, InMemoryOutputTarget, MockAudioSource, MockErrorEmitter, MockFocusCapture,
    MockHistoryBackend, MockPasteBackend, MockSttProvider, MockVadProvider, MockCleanupStyle, QueuedMockSttProvider,
};

use klarvo_shell_orchestrator::SessionOrchestrator;

// Minimal manifest: STT stage + identity cleanup stage.
fn test_manifest_toml() -> &'static str {
    r#"
schema_version = 1

[[pipeline.stages]]
type = "stt"
plugin_id = "mock-stt"

[[pipeline.stages]]
type = "cleanup"
plugin_id = "mock-cleanup"
"#
}

/// Build a `SessionOrchestrator` with the given mode and controllable mocks.
fn make_orchestrator_with_mode(
    stt: Arc<dyn klarvo_core::traits::SttProvider>,
    mode: RecordingMode,
) -> (
    SessionOrchestrator,
    Arc<InMemoryOutputTarget>,
    Arc<MockPasteBackend>,
    Arc<MockErrorEmitter>,
    Arc<EventBus>,
    Arc<MockFocusCapture>,
    Arc<MockHistoryBackend>,
) {
    let manifest = Arc::new(parse_manifest(test_manifest_toml()).expect("test manifest must parse"));

    let mut registry = klarvo_core::registry::bootstrap();
    registry.register_stt("mock-stt", stt);
    registry.register_cleanup("mock-cleanup", Arc::new(MockCleanupStyle::identity()));
    let output_target = Arc::new(InMemoryOutputTarget::new());
    registry.register_output("test-output", Arc::clone(&output_target) as Arc<dyn klarvo_core::output::OutputTarget>);
    let registry = Arc::new(registry);

    // MockVadProvider: first chunk → SpeechStart, second → SpeechEnd (triggers pipeline).
    let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MockVadProvider::with_decisions(vec![
            VadDecision::SpeechStart { ts_ms: 0 },
            VadDecision::SpeechEnd { ts_ms: 10, duration_ms: 10 },
        ]))));

    // 10 chunks of zero-filled audio (enough for VAD to fire SpeechStart + SpeechEnd).
    let audio_source: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::AudioSource>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(
            MockAudioSource::with_synthetic_chunks(10, 160, 0),
        )));

    let paste_backend = Arc::new(MockPasteBackend::new());
    let error_emitter = Arc::new(MockErrorEmitter::new());
    let clock: Arc<FakeClock> = Arc::new(FakeClock::default());
    let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));
    let mode_arc = Arc::new(tokio::sync::RwLock::new(mode));
    let focus_capture = Arc::new(MockFocusCapture::new());
    let history_backend = Arc::new(MockHistoryBackend::new());

    let mode_arc_slot2 = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let orch = SessionOrchestrator::new(
        registry,
        manifest,
        audio_source,
        "test-output".to_string(),
        Arc::clone(&paste_backend) as Arc<dyn klarvo_core::output::PasteBackend>,
        Arc::clone(&error_emitter) as Arc<dyn klarvo_core::event::emitter::ErrorEmitter>,
        clock as Arc<dyn klarvo_core::time::Clock>,
        vad,
        Arc::clone(&event_bus),
        mode_arc,
        mode_arc_slot2,
        Arc::clone(&focus_capture) as Arc<dyn klarvo_core::output::FocusCapture>,
        Arc::clone(&history_backend) as Arc<dyn klarvo_core::history::HistoryBackend>,
        Arc::new(tokio::sync::RwLock::new(None::<String>)),
        Arc::new(|_name: &str| true),
    );

    (orch, output_target, paste_backend, error_emitter, event_bus, focus_capture, history_backend)
}

/// Shared setup: builds a SessionOrchestrator with Hold mode (Phase-1 default).
fn make_orchestrator(
    stt: Arc<dyn klarvo_core::traits::SttProvider>,
) -> (
    SessionOrchestrator,
    Arc<InMemoryOutputTarget>,
    Arc<MockPasteBackend>,
    Arc<MockErrorEmitter>,
    Arc<EventBus>,
    Arc<MockFocusCapture>,
    Arc<MockHistoryBackend>,
) {
    make_orchestrator_with_mode(stt, RecordingMode::Hold)
}

/// Build a `SessionOrchestrator` with separate modes for slot-1 and slot-2.
/// Used by Story 8.1 cross-slot mutual-exclusion tests where slot-1 and slot-2
/// can have different recording modes (Code-Review-Closure 2026-05-05 P4).
fn make_orchestrator_with_modes(
    stt: Arc<dyn klarvo_core::traits::SttProvider>,
    mode_slot1: RecordingMode,
    mode_slot2: RecordingMode,
) -> (
    SessionOrchestrator,
    Arc<InMemoryOutputTarget>,
    Arc<MockPasteBackend>,
    Arc<MockErrorEmitter>,
    Arc<EventBus>,
    Arc<MockFocusCapture>,
    Arc<MockHistoryBackend>,
) {
    let manifest = Arc::new(parse_manifest(test_manifest_toml()).expect("test manifest must parse"));

    let mut registry = klarvo_core::registry::bootstrap();
    registry.register_stt("mock-stt", stt);
    registry.register_cleanup("mock-cleanup", Arc::new(MockCleanupStyle::identity()));
    let output_target = Arc::new(InMemoryOutputTarget::new());
    registry.register_output("test-output", Arc::clone(&output_target) as Arc<dyn klarvo_core::output::OutputTarget>);
    let registry = Arc::new(registry);

    let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MockVadProvider::with_decisions(vec![
            VadDecision::SpeechStart { ts_ms: 0 },
            VadDecision::SpeechEnd { ts_ms: 10, duration_ms: 10 },
        ]))));

    let audio_source: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::AudioSource>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(
            MockAudioSource::with_synthetic_chunks(10, 160, 0),
        )));

    let paste_backend = Arc::new(MockPasteBackend::new());
    let error_emitter = Arc::new(MockErrorEmitter::new());
    let clock: Arc<FakeClock> = Arc::new(FakeClock::default());
    let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));
    let mode_arc = Arc::new(tokio::sync::RwLock::new(mode_slot1));
    let mode_arc_slot2 = Arc::new(tokio::sync::RwLock::new(mode_slot2));
    let focus_capture = Arc::new(MockFocusCapture::new());
    let history_backend = Arc::new(MockHistoryBackend::new());

    let orch = SessionOrchestrator::new(
        registry,
        manifest,
        audio_source,
        "test-output".to_string(),
        Arc::clone(&paste_backend) as Arc<dyn klarvo_core::output::PasteBackend>,
        Arc::clone(&error_emitter) as Arc<dyn klarvo_core::event::emitter::ErrorEmitter>,
        clock as Arc<dyn klarvo_core::time::Clock>,
        vad,
        Arc::clone(&event_bus),
        mode_arc,
        mode_arc_slot2,
        Arc::clone(&focus_capture) as Arc<dyn klarvo_core::output::FocusCapture>,
        Arc::clone(&history_backend) as Arc<dyn klarvo_core::history::HistoryBackend>,
        Arc::new(tokio::sync::RwLock::new(None::<String>)),
        Arc::new(|_name: &str| true),
    );

    (orch, output_target, paste_backend, error_emitter, event_bus, focus_capture, history_backend)
}

/// Poll until `InMemoryOutputTarget` has at least one delivery, or timeout.
async fn wait_for_delivery(target: &InMemoryOutputTarget) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if target.last_delivered().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("delivery must occur within 5 seconds");
}

/// Poll until `MockErrorEmitter` has at least one recorded error, or timeout.
async fn wait_for_error(emitter: &MockErrorEmitter) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if !emitter.recorded().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("error must be emitted within 5 seconds");
}

/// Poll until `MockFocusCapture` has at least `n` recorded restore() calls, or timeout.
/// Replaces sleep-based test sync to avoid CI flakes (see Review 2026-05-03 P4).
async fn wait_for_restore(focus_capture: &MockFocusCapture, n: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if focus_capture.restore_count() >= n {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("restore must be called within 5 seconds");
}

/// Poll until `MockHistoryBackend` has at least `expected_count` entries, or timeout.
async fn wait_for_history(backend: &MockHistoryBackend, expected_count: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if backend.entry_count() >= expected_count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("history entry must be recorded within 5 seconds");
}

/// Collect every event received until `RecordingCompleted` arrives, then return
/// the full sequence. Used to assert lifecycle ordering (Started→Stopped→Completed).
async fn collect_events_until_completed(
    rx: &mut tokio::sync::broadcast::Receiver<Event>,
) -> Vec<Event> {
    let mut events = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match rx.recv().await {
                Ok(evt) => {
                    let is_completed = matches!(evt, Event::RecordingCompleted { .. });
                    events.push(evt);
                    if is_completed {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
    .await
    .expect("RecordingCompleted must arrive within 5 seconds");
    events
}

#[tokio::test]
async fn test1_happy_path_press_release_delivers_and_pastes() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, output_target, paste_backend, error_emitter, event_bus, _, _) = make_orchestrator(stt);
    let mut rx = event_bus.subscribe();

    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    orch.on_release(HotkeySlot::One).await;

    assert_eq!(output_target.last_delivered().as_deref(), Some("hello"));
    assert!(paste_backend.was_called(), "paste must be called after delivery");
    assert!(error_emitter.recorded().is_empty(), "no errors expected");

    // Recording-lifecycle contract (klarvo-core/src/event/bus.rs `Event` doc):
    //   Started → fires synchronously inside on_press (deterministic-first).
    //   Stopped → fires synchronously inside on_release.
    //   Completed → fires from the detached pipeline task once it exits.
    // Stopped and Completed race each other (one is in the test task, the other
    // in the spawned pipeline task), so order between them is non-deterministic.
    // We assert Started-first + presence of both Stopped and Completed.
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        match rx.try_recv() {
            Ok(e) => events.push(e),
            Err(_) => {
                let saw_stopped = events.iter().any(|e| matches!(e, Event::RecordingStopped { .. }));
                let saw_completed = events.iter().any(|e| matches!(e, Event::RecordingCompleted { .. }));
                if events.first().is_some() && saw_stopped && saw_completed {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    panic!("expected Started/Stopped/Completed within 2s; got {events:?}");
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    }
    assert!(
        matches!(events.first(), Some(Event::RecordingStarted { .. })),
        "first event must be RecordingStarted; got {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::RecordingStopped { .. })),
        "RecordingStopped must be emitted on hotkey-release; got {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::RecordingCompleted { .. })),
        "RecordingCompleted must be emitted after pipeline task exits; got {events:?}"
    );
}

#[tokio::test]
async fn test2_idempotent_press_key_repeat_guard() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, output_target, _paste_backend, error_emitter, _event_bus, _, _) = make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    orch.on_press(HotkeySlot::One).await; // second press — must be discarded by key-repeat guard

    // Wait for the single pipeline to deliver, then clean up.
    wait_for_delivery(&output_target).await;
    orch.on_release(HotkeySlot::One).await;

    // Only one delivery, no error emitted.
    assert_eq!(output_target.all_delivered().len(), 1, "exactly one delivery expected");
    assert!(error_emitter.recorded().is_empty(), "key-repeat must not emit errors");
}

#[tokio::test]
async fn test3_stray_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, _output, paste_backend, error_emitter, _event_bus, _, _) = make_orchestrator(stt);

    // Release without prior press — must be a silent no-op.
    orch.on_release(HotkeySlot::One).await;

    assert!(!paste_backend.was_called(), "stray release must not paste");
    assert!(error_emitter.recorded().is_empty(), "stray release must not emit errors");
}

#[tokio::test]
async fn test4_pipeline_failure_emits_error_state_recovers() {
    // Empty queue → QueuedMockSttProvider returns Internal error on first call.
    let stt = Arc::new(QueuedMockSttProvider::with_transcriptions(vec![]));
    let (orch, _, paste_backend, error_emitter, _event_bus, _, _) = make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    wait_for_error(&error_emitter).await;
    orch.on_release(HotkeySlot::One).await;

    assert!(!error_emitter.recorded().is_empty(), "pipeline failure must emit an error");
    assert!(!paste_backend.was_called(), "paste must not happen after STT failure");
}

// ---------------------------------------------------------------------------
// AC-4: Toggle-Modus
// ---------------------------------------------------------------------------

#[tokio::test]
async fn toggle_press_starts_recording() {
    let stt = Arc::new(MockSttProvider::returning("toggle text"));
    let (orch, _output, _paste, _err, event_bus, _, _) = make_orchestrator_with_mode(stt, RecordingMode::Toggle);
    let mut rx = event_bus.subscribe();

    orch.on_press(HotkeySlot::One).await;

    // RecordingStarted must be emitted synchronously
    let evt = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Ok(e) = rx.try_recv() {
                return e;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("RecordingStarted within 1s");

    assert!(matches!(evt, Event::RecordingStarted { .. }), "first event must be RecordingStarted");

    // Cleanup
    orch.on_press(HotkeySlot::One).await; // second press to stop
}

#[tokio::test]
async fn toggle_second_press_stops_recording() {
    let stt = Arc::new(MockSttProvider::returning("toggle delivery"));
    let (orch, output_target, paste_backend, _err, _event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::Toggle);

    orch.on_press(HotkeySlot::One).await; // start
    orch.on_press(HotkeySlot::One).await; // stop → channel closes → pipeline runs

    wait_for_delivery(&output_target).await;
    assert_eq!(output_target.last_delivered().as_deref(), Some("toggle delivery"));
    // Paste should be called (Toggle behavior same as Hold)
    assert!(paste_backend.was_called(), "paste must be called in Toggle mode");
}

#[tokio::test]
async fn toggle_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, paste_backend, error_emitter, _event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::Toggle);

    orch.on_press(HotkeySlot::One).await; // start recording

    // on_release in Toggle mode while recording — must be a no-op
    orch.on_release(HotkeySlot::One).await;

    // No paste yet, no errors from stray release
    assert!(!paste_backend.was_called());
    assert!(error_emitter.recorded().is_empty());

    // Cleanup: second press to stop
    orch.on_press(HotkeySlot::One).await;
}

// ---------------------------------------------------------------------------
// AC-5: AutoStop-Modus
// ---------------------------------------------------------------------------

#[tokio::test]
async fn autostop_transitions_to_idle_after_vad() {
    let stt = Arc::new(MockSttProvider::returning("autostop text"));
    let (orch, output_target, paste_backend, _err, event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::AutoStop);
    let mut rx = event_bus.subscribe();

    orch.on_press(HotkeySlot::One).await;
    // VAD fires SpeechEnd automatically via MockVadProvider → pipeline runs → cleanup → Idle.
    wait_for_delivery(&output_target).await;
    let events = collect_events_until_completed(&mut rx).await;

    assert_eq!(output_target.last_delivered().as_deref(), Some("autostop text"));
    assert!(paste_backend.was_called(), "paste must be called in AutoStop mode");
    // AC-5: cleanup branch must transition state back to Idle so the next press
    // is not blocked by a stale Recording state.
    assert!(orch.is_idle().await, "orchestrator must be Idle after AutoStop cleanup");

    // Re-D1 (Re-Review-Closure): AutoStop must emit the full 3-state lifecycle
    // (Started → Stopped → Completed) like Hold/Toggle. A regression that drops
    // the Stopped emit would desync tray / Pill-Bar state-pull subscribers.
    assert!(
        matches!(events.first(), Some(Event::RecordingStarted { .. })),
        "first event must be RecordingStarted; got {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::RecordingStopped { .. })),
        "AutoStop must emit RecordingStopped after audio capture ends (Re-D1); got {events:?}"
    );
    let stopped_idx = events
        .iter()
        .position(|e| matches!(e, Event::RecordingStopped { .. }))
        .expect("RecordingStopped present (asserted above)");
    let completed_idx = events
        .iter()
        .position(|e| matches!(e, Event::RecordingCompleted { .. }))
        .expect("RecordingCompleted present (collect_events_until_completed terminator)");
    assert!(
        stopped_idx < completed_idx,
        "RecordingStopped must precede RecordingCompleted; got {events:?}"
    );
}

#[tokio::test]
async fn autostop_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, paste_backend, error_emitter, _event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::AutoStop);

    orch.on_press(HotkeySlot::One).await;

    // on_release in AutoStop mode while recording — must be a no-op
    orch.on_release(HotkeySlot::One).await;

    assert!(!paste_backend.was_called());
    assert!(error_emitter.recorded().is_empty());

    // Wait for VAD-triggered cleanup to complete
    tokio::time::sleep(Duration::from_millis(200)).await;
}

// ---------------------------------------------------------------------------
// AC-6: WaitAndType-Modus
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wait_and_type_skips_paste_emits_delivered() {
    let stt = Arc::new(MockSttProvider::returning("wait text"));
    let (orch, output_target, paste_backend, _err, event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::WaitAndType);
    let mut rx = event_bus.subscribe();

    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    orch.on_release(HotkeySlot::One).await;

    // Text must be in clipboard (OutputTarget delivery)
    assert_eq!(output_target.last_delivered().as_deref(), Some("wait text"));

    // Paste must NOT be called
    assert!(!paste_backend.was_called(), "paste must be skipped in WaitAndType mode");

    // RecordingDelivered must be emitted
    let delivered = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match rx.try_recv() {
                Ok(Event::RecordingDelivered { text, .. }) => return text,
                Ok(_) | Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        }
    })
    .await
    .expect("RecordingDelivered must be emitted within 2s");

    assert_eq!(delivered, "wait text");
}

// ---------------------------------------------------------------------------
// AC-1 / AC-3 / AC-4: shutdown()
// ---------------------------------------------------------------------------

/// Drain a broadcast Receiver until it is empty, with a bounded timeout per recv attempt.
/// More deterministic than `sleep + try_recv` — surfaces late events instead of racing them.
async fn drain_events(rx: &mut tokio::sync::broadcast::Receiver<Event>) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Ok(evt)) =
        tokio::time::timeout(Duration::from_millis(50), rx.recv()).await
    {
        events.push(evt);
    }
    events
}

/// Wrap a `shutdown().await` in a 1-second timeout — surfaces hangs (deadlocks, lock
/// contention, awaiting on never-completing futures) as a useful test failure instead of
/// blocking the test runner indefinitely.
async fn shutdown_with_timeout(orch: &SessionOrchestrator) {
    tokio::time::timeout(Duration::from_secs(1), orch.shutdown())
        .await
        .expect("shutdown must complete within 1s");
}

/// AC-1: shutdown() while Idle is a no-op and does not panic.
#[tokio::test]
async fn shutdown_while_idle_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, paste_backend, error_emitter, _event_bus, _, _) = make_orchestrator(stt);

    // Idle state — shutdown must be a no-op.
    shutdown_with_timeout(&orch).await;

    assert!(!paste_backend.was_called());
    assert!(error_emitter.recorded().is_empty());
    assert!(orch.is_idle().await);
}

/// AC-1: shutdown() while Recording aborts the pipeline and transitions to Idle.
/// AC-1: RecordingStopped + RecordingCompleted must NOT be emitted on the forced-teardown path.
#[tokio::test]
async fn shutdown_while_recording_aborts_and_no_stopped_event() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, _paste, _err, event_bus, _, _) = make_orchestrator(stt);
    let mut rx = event_bus.subscribe();

    orch.on_press(HotkeySlot::One).await;
    // Verify we are recording.
    assert!(!orch.is_idle().await);

    shutdown_with_timeout(&orch).await;

    // After shutdown, state must be Idle.
    assert!(orch.is_idle().await, "orchestrator must be Idle after shutdown");

    // Drain via a bounded poll loop instead of sleep+try_recv — see drain_events doc.
    let events = drain_events(&mut rx).await;

    // RecordingStarted must have been emitted by on_press.
    assert!(
        events.iter().any(|e| matches!(e, Event::RecordingStarted { .. })),
        "RecordingStarted must be emitted by on_press; got {events:?}"
    );
    // RecordingStopped must NOT be emitted by shutdown().
    assert!(
        !events.iter().any(|e| matches!(e, Event::RecordingStopped { .. })),
        "RecordingStopped must NOT be emitted on forced shutdown; got {events:?}"
    );
    // RecordingCompleted must NOT be emitted on the forced-teardown path either.
    assert!(
        !events.iter().any(|e| matches!(e, Event::RecordingCompleted { .. })),
        "RecordingCompleted must NOT be emitted on forced shutdown; got {events:?}"
    );
}

/// AC-1: shutdown() is idempotent — calling it twice must not panic.
#[tokio::test]
async fn shutdown_is_idempotent() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, _paste, _err, _event_bus, _, _) = make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    shutdown_with_timeout(&orch).await;
    shutdown_with_timeout(&orch).await; // second call — must not panic, must stay Idle

    assert!(orch.is_idle().await);
}

/// AC-3: on_release semantics unchanged after shutdown is added — stray release still no-op.
/// Also covers the post-shutdown stray-release path: `shutdown() → on_release()` must be safe.
#[tokio::test]
async fn on_release_semantics_unchanged_after_shutdown_impl() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, output_target, paste_backend, error_emitter, _event_bus, _, _) = make_orchestrator(stt);

    // Phase 1: Normal Hold press/release cycle must still work.
    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    orch.on_release(HotkeySlot::One).await;

    assert_eq!(output_target.last_delivered().as_deref(), Some("hello"));
    assert!(paste_backend.was_called(), "paste must still be called in normal Hold cycle");
    assert!(error_emitter.recorded().is_empty());

    // Phase 2: After shutdown, a stray on_release must remain a safe no-op.
    shutdown_with_timeout(&orch).await;
    assert!(orch.is_idle().await);
    orch.on_release(HotkeySlot::One).await; // must not panic; orchestrator is Idle
    assert!(orch.is_idle().await, "stray on_release post-shutdown must not change state");
}

/// AC-1/coverage: `on_press → on_release → shutdown` — pipeline_task is detached
/// (Hold-mode `on_release` drops the handle without abort); shutdown afterwards must
/// remain a safe no-op (state already Idle) and not panic.
#[tokio::test]
async fn shutdown_after_on_release_is_safe_noop() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, _paste, _err, _event_bus, _, _) = make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    orch.on_release(HotkeySlot::One).await; // detaches pipeline_task — state transitions to Idle
    assert!(orch.is_idle().await);

    shutdown_with_timeout(&orch).await; // must be a clean no-op
    assert!(orch.is_idle().await);
}

/// AC-1/coverage: shutdown() in AutoStop mode while Recording. AutoStop's pipeline-internal
/// session_state lock-acquire (session.rs `RecordingMode::AutoStop` cleanup branch) is the
/// path most likely to deadlock if `shutdown()`'s lock-ordering is wrong. With the 1-second
/// timeout wrapper, a deadlock surfaces as a clean test failure.
#[tokio::test]
async fn shutdown_while_recording_autostop_does_not_deadlock() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, _paste, _err, _event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::AutoStop);

    orch.on_press(HotkeySlot::One).await;
    shutdown_with_timeout(&orch).await;
    assert!(orch.is_idle().await);
}

/// AC-1/coverage: shutdown() in Toggle mode while Recording. Toggle treats the second
/// on_press as the stop signal, so shutdown is the only forced-exit path mid-recording.
#[tokio::test]
async fn shutdown_while_recording_toggle_does_not_deadlock() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, _paste, _err, _event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::Toggle);

    orch.on_press(HotkeySlot::One).await; // Toggle: enters Recording
    shutdown_with_timeout(&orch).await;
    assert!(orch.is_idle().await);
}

// ---------------------------------------------------------------------------
// AC-8: Return-Focus-Dispatch (always-restore policy, Review-Closure 2026-05-03 D1=A)
// ---------------------------------------------------------------------------

/// After successful delivery in Hold mode, focus is restored exactly once with
/// the sentinel handle captured by MockFocusCapture.
#[tokio::test]
async fn focus_restored_after_successful_delivery() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, output_target, _paste, _err, _event_bus, focus_capture, _) =
        make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    orch.on_release(HotkeySlot::One).await;

    wait_for_restore(&focus_capture, 1).await;

    assert_eq!(focus_capture.restore_count(), 1, "restore must be called once after successful delivery");
    assert_eq!(
        focus_capture.last_restored(),
        Some(Some(42)),
        "restored handle must match sentinel captured by MockFocusCapture"
    );
}

/// Always-restore policy: after a failed delivery (OutputTarget returns error), focus
/// MUST still be restored so the user is not stranded on Klarvo's overlay.
#[tokio::test]
async fn focus_restored_after_deliver_error() {
    use klarvo_core::error::{AppError, AppErrorKind};

    // Build orchestrator manually with a failing output target
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let manifest =
        Arc::new(klarvo_core::manifest::parse_from_str(test_manifest_toml()).expect("test manifest"));

    let mut registry = klarvo_core::registry::bootstrap();
    registry.register_stt("mock-stt", stt);
    registry.register_cleanup("mock-cleanup", Arc::new(MockCleanupStyle::identity()));
    // Register a failing output target by wrapping InMemoryOutputTarget with a custom impl
    struct FailingOutputTarget;
    #[async_trait::async_trait]
    impl klarvo_core::output::OutputTarget for FailingOutputTarget {
        async fn deliver(&self, _text: &str) -> Result<(), AppError> {
            Err(AppError {
                kind: AppErrorKind::Internal,
                message: "simulated deliver failure".to_string(),
                user_message: Some("error.internal".to_string()),
                retryable: false,
            })
        }
    }
    registry.register_output("test-output", Arc::new(FailingOutputTarget) as Arc<dyn klarvo_core::output::OutputTarget>);
    let registry = Arc::new(registry);

    let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MockVadProvider::with_decisions(vec![
            klarvo_core::audio::vad::VadDecision::SpeechStart { ts_ms: 0 },
            klarvo_core::audio::vad::VadDecision::SpeechEnd { ts_ms: 10, duration_ms: 10 },
        ]))));
    let audio_source: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::AudioSource>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(
            MockAudioSource::with_synthetic_chunks(10, 160, 0),
        )));

    let paste_backend = Arc::new(MockPasteBackend::new());
    let error_emitter = Arc::new(MockErrorEmitter::new());
    let clock: Arc<FakeClock> = Arc::new(FakeClock::default());
    let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));
    let mode_arc = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let mode_arc_slot2 = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let focus_capture = Arc::new(MockFocusCapture::new());

    let orch = SessionOrchestrator::new(
        registry,
        manifest,
        audio_source,
        "test-output".to_string(),
        Arc::clone(&paste_backend) as Arc<dyn klarvo_core::output::PasteBackend>,
        Arc::clone(&error_emitter) as Arc<dyn klarvo_core::event::emitter::ErrorEmitter>,
        clock as Arc<dyn klarvo_core::time::Clock>,
        vad,
        Arc::clone(&event_bus),
        mode_arc,
        mode_arc_slot2,
        Arc::clone(&focus_capture) as Arc<dyn klarvo_core::output::FocusCapture>,
        Arc::new(klarvo_core::history::NullHistoryBackend) as Arc<dyn klarvo_core::history::HistoryBackend>,
        Arc::new(tokio::sync::RwLock::new(None::<String>)),
        Arc::new(|_name: &str| true),
    );

    orch.on_press(HotkeySlot::One).await;
    wait_for_error(&error_emitter).await;
    orch.on_release(HotkeySlot::One).await;

    wait_for_restore(&focus_capture, 1).await;

    assert_eq!(
        focus_capture.restore_count(),
        1,
        "restore MUST be called after failed delivery (always-restore policy)"
    );
    assert_eq!(
        focus_capture.last_restored(),
        Some(Some(42)),
        "restored handle must match sentinel captured by MockFocusCapture"
    );
}

/// Always-restore policy: when the paste backend fails, focus MUST still be restored.
/// Covers the Hold/Toggle/AutoStop paste-error branch of the always-restore decision.
#[tokio::test]
async fn focus_restored_after_paste_error() {
    use klarvo_core::error::{AppError, AppErrorKind};

    let stt = Arc::new(MockSttProvider::returning("hello"));
    let manifest =
        Arc::new(klarvo_core::manifest::parse_from_str(test_manifest_toml()).expect("test manifest"));

    let mut registry = klarvo_core::registry::bootstrap();
    registry.register_stt("mock-stt", stt);
    registry.register_cleanup("mock-cleanup", Arc::new(MockCleanupStyle::identity()));
    let output_target = Arc::new(InMemoryOutputTarget::new());
    registry.register_output(
        "test-output",
        Arc::clone(&output_target) as Arc<dyn klarvo_core::output::OutputTarget>,
    );
    let registry = Arc::new(registry);

    let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MockVadProvider::with_decisions(vec![
            klarvo_core::audio::vad::VadDecision::SpeechStart { ts_ms: 0 },
            klarvo_core::audio::vad::VadDecision::SpeechEnd { ts_ms: 10, duration_ms: 10 },
        ]))));
    let audio_source: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::AudioSource>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(
            MockAudioSource::with_synthetic_chunks(10, 160, 0),
        )));

    // Paste backend that always returns an error.
    struct FailingPasteBackend;
    #[async_trait::async_trait]
    impl klarvo_core::output::PasteBackend for FailingPasteBackend {
        async fn paste(&self) -> Result<(), AppError> {
            Err(AppError {
                kind: AppErrorKind::Internal,
                message: "simulated paste failure".to_string(),
                user_message: Some("error.internal".to_string()),
                retryable: false,
            })
        }
    }
    let paste_backend: Arc<dyn klarvo_core::output::PasteBackend> = Arc::new(FailingPasteBackend);
    let error_emitter = Arc::new(MockErrorEmitter::new());
    let clock: Arc<FakeClock> = Arc::new(FakeClock::default());
    let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));
    let mode_arc = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let mode_arc_slot2 = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let focus_capture = Arc::new(MockFocusCapture::new());

    let orch = SessionOrchestrator::new(
        registry,
        manifest,
        audio_source,
        "test-output".to_string(),
        paste_backend,
        Arc::clone(&error_emitter) as Arc<dyn klarvo_core::event::emitter::ErrorEmitter>,
        clock as Arc<dyn klarvo_core::time::Clock>,
        vad,
        Arc::clone(&event_bus),
        mode_arc,
        mode_arc_slot2,
        Arc::clone(&focus_capture) as Arc<dyn klarvo_core::output::FocusCapture>,
        Arc::new(klarvo_core::history::NullHistoryBackend) as Arc<dyn klarvo_core::history::HistoryBackend>,
        Arc::new(tokio::sync::RwLock::new(None::<String>)),
        Arc::new(|_name: &str| true),
    );

    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await; // deliver succeeded
    wait_for_error(&error_emitter).await;    // paste failed → error emitted
    orch.on_release(HotkeySlot::One).await;

    wait_for_restore(&focus_capture, 1).await;

    assert_eq!(
        focus_capture.restore_count(),
        1,
        "restore MUST be called after paste failure (always-restore policy)"
    );
}

// ---------------------------------------------------------------------------
// AC-11: History Backend Integration
// ---------------------------------------------------------------------------

/// History entry is appended when delivery succeeds.
#[tokio::test]
async fn history_saved_after_successful_delivery() {
    let stt = Arc::new(MockSttProvider::returning("saved text"));
    let (orch, output_target, _paste, _err, _event_bus, _, history_backend) =
        make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    orch.on_release(HotkeySlot::One).await;

    wait_for_history(&history_backend, 1).await;

    assert_eq!(history_backend.entry_count(), 1, "history must have exactly one entry");
    let entries = history_backend.all_entries();
    assert_eq!(entries[0].text, "saved text", "history entry text must match delivered text");
}

/// History is NOT written when delivery fails (error path).
#[tokio::test]
async fn history_not_saved_on_deliver_error() {
    use klarvo_core::error::{AppError, AppErrorKind};

    let stt = Arc::new(MockSttProvider::returning("not saved"));
    let manifest =
        Arc::new(klarvo_core::manifest::parse_from_str(test_manifest_toml()).expect("test manifest"));

    let mut registry = klarvo_core::registry::bootstrap();
    registry.register_stt("mock-stt", stt);
    registry.register_cleanup("mock-cleanup", Arc::new(MockCleanupStyle::identity()));
    struct FailingOutputTarget;
    #[async_trait::async_trait]
    impl klarvo_core::output::OutputTarget for FailingOutputTarget {
        async fn deliver(&self, _text: &str) -> Result<(), AppError> {
            Err(AppError {
                kind: AppErrorKind::Internal,
                message: "simulated deliver failure".to_string(),
                user_message: Some("error.internal".to_string()),
                retryable: false,
            })
        }
    }
    registry.register_output(
        "test-output",
        Arc::new(FailingOutputTarget) as Arc<dyn klarvo_core::output::OutputTarget>,
    );
    let registry = Arc::new(registry);

    let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MockVadProvider::with_decisions(vec![
            VadDecision::SpeechStart { ts_ms: 0 },
            VadDecision::SpeechEnd { ts_ms: 10, duration_ms: 10 },
        ]))));
    let audio_source: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::AudioSource>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(
            MockAudioSource::with_synthetic_chunks(10, 160, 0),
        )));

    let paste_backend = Arc::new(MockPasteBackend::new());
    let error_emitter = Arc::new(MockErrorEmitter::new());
    let clock: Arc<FakeClock> = Arc::new(FakeClock::default());
    let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));
    let mode_arc = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let focus_capture = Arc::new(MockFocusCapture::new());
    let history_backend = Arc::new(MockHistoryBackend::new());

    let mode_arc_slot2 = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let orch = SessionOrchestrator::new(
        registry,
        manifest,
        audio_source,
        "test-output".to_string(),
        Arc::clone(&paste_backend) as Arc<dyn klarvo_core::output::PasteBackend>,
        Arc::clone(&error_emitter) as Arc<dyn klarvo_core::event::emitter::ErrorEmitter>,
        clock as Arc<dyn klarvo_core::time::Clock>,
        vad,
        Arc::clone(&event_bus),
        mode_arc,
        mode_arc_slot2,
        Arc::clone(&focus_capture) as Arc<dyn klarvo_core::output::FocusCapture>,
        Arc::clone(&history_backend) as Arc<dyn klarvo_core::history::HistoryBackend>,
        Arc::new(tokio::sync::RwLock::new(None::<String>)),
        Arc::new(|_name: &str| true),
    );

    orch.on_press(HotkeySlot::One).await;
    wait_for_error(&error_emitter).await;
    orch.on_release(HotkeySlot::One).await;

    // Drain the pipeline-task deterministically: shutdown() aborts the in-flight task
    // and waits for it to settle. After this returns, no further history.append can
    // sneak in — so a zero-entry assertion is race-free (no fixed sleep needed).
    shutdown_with_timeout(&orch).await;

    assert_eq!(history_backend.entry_count(), 0, "history must NOT be written when delivery fails");
}

// ---------------------------------------------------------------------------
// AC-7 (Story 8.1): Mutual-Exclusion — Slot-2 press discarded during Slot-1 recording
// ---------------------------------------------------------------------------

/// D-1: A Slot-Two press while Slot-One is recording must be silently discarded.
/// Code-Review-Closure 2026-05-05 P1 made the guard slot-aware; it now relies on
/// `SessionState::Recording.owner_slot` rather than the press-mode dispatch.
#[tokio::test]
async fn slot2_press_discarded_when_slot1_recording() {
    let stt = Arc::new(MockSttProvider::returning("hello from slot1"));
    let (orch, output_target, _paste, _err, _event_bus, _, _history) = make_orchestrator(stt);

    // Start recording via Slot 1
    orch.on_press(HotkeySlot::One).await;
    assert!(!orch.is_idle().await, "must be Recording after Slot-1 press");

    // Slot-2 press while recording: must be silently discarded — state stays Recording
    orch.on_press(HotkeySlot::Two).await;
    assert!(!orch.is_idle().await, "must remain Recording after Slot-2 press (D-1 mutual-exclusion)");

    // Clean up: release Slot 1 and drain the pipeline
    orch.on_release(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    assert!(orch.is_idle().await, "must be Idle after Slot-1 release and pipeline drain");
}

/// P4 (Code-Review-Closure 2026-05-05): a stray `on_release(Two)` while Slot-1 is
/// recording in Hold mode must NOT terminate Slot-1. Pre-fix the `slot` parameter
/// was discarded (`let _ = slot;`) and the Hold-arm of `on_release` ran on every
/// release call regardless of which slot fired it — turning a Slot-2 tap into an
/// accidental Slot-1 stop mid-recording.
#[tokio::test]
async fn slot2_release_does_not_terminate_slot1_hold_recording() {
    let stt = Arc::new(MockSttProvider::returning("intact slot1 dictation"));
    let (orch, output_target, paste_backend, _err, _event_bus, _, _) = make_orchestrator(stt);

    orch.on_press(HotkeySlot::One).await;
    assert!(!orch.is_idle().await, "must be Recording after Slot-1 press");

    // Stray Slot-2 release while Slot-1 is in Hold-recording: must be silently discarded.
    orch.on_release(HotkeySlot::Two).await;
    assert!(!orch.is_idle().await, "Slot-2 release must NOT stop Slot-1 (D-1 cross-slot guard)");
    assert!(!paste_backend.was_called(), "paste must not fire — Slot-1 still recording");

    // Slot-1 release legitimately stops the session and triggers delivery.
    orch.on_release(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    assert_eq!(output_target.last_delivered().as_deref(), Some("intact slot1 dictation"));
}

/// P4: a Slot-2 press while Slot-One is recording in Toggle mode must NOT toggle
/// Slot-One off. Pre-fix the `if press_mode == Toggle` arm in `on_press` ran
/// without checking which slot fired the press, so any Slot-Two tap during a
/// Slot-One Toggle-session would stop Slot-One.
#[tokio::test]
async fn slot2_press_does_not_stop_slot1_toggle_recording() {
    let stt = Arc::new(MockSttProvider::returning("toggle stays alive"));
    let (orch, output_target, _paste, _err, _event_bus, _, _) =
        make_orchestrator_with_mode(stt, RecordingMode::Toggle);

    orch.on_press(HotkeySlot::One).await; // start Toggle-recording
    assert!(!orch.is_idle().await, "must be Recording after Slot-1 toggle-start");

    // Slot-2 press: must be discarded by cross-slot guard, NOT trigger toggle-stop.
    orch.on_press(HotkeySlot::Two).await;
    assert!(!orch.is_idle().await, "Slot-2 press must NOT stop Slot-1 Toggle-recording");

    // Second Slot-1 press legitimately stops via toggle-arm and triggers delivery.
    orch.on_press(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    assert_eq!(output_target.last_delivered().as_deref(), Some("toggle stays alive"));
    assert!(orch.is_idle().await, "must be Idle after Slot-1 toggle-stop and pipeline drain");
}

/// P4: symmetric check — when Slot-2 owns an active recording (Slot-2 in Hold),
/// a stray Slot-1 press must be discarded by the cross-slot guard.
#[tokio::test]
async fn slot1_press_discarded_when_slot2_recording_hold() {
    let stt = Arc::new(MockSttProvider::returning("intact slot2 dictation"));
    let (orch, output_target, paste_backend, _err, _event_bus, _, _) = make_orchestrator(stt);

    orch.on_press(HotkeySlot::Two).await;
    assert!(!orch.is_idle().await, "must be Recording after Slot-2 press");

    // Slot-1 press while Slot-2 owns the session: discard.
    orch.on_press(HotkeySlot::One).await;
    assert!(!orch.is_idle().await, "Slot-1 press must NOT affect Slot-2 ownership");
    assert!(!paste_backend.was_called(), "paste must not fire — Slot-2 still recording");

    // Slot-2 release legitimately stops; Slot-1 release between them was a no-op.
    orch.on_release(HotkeySlot::Two).await;
    wait_for_delivery(&output_target).await;
    assert_eq!(output_target.last_delivered().as_deref(), Some("intact slot2 dictation"));
}

/// P4: cross-mode coverage — Slot-1 Hold + Slot-2 Toggle. Each slot's session
/// must use its own mode arc; cross-slot presses/releases must be discarded.
#[tokio::test]
async fn cross_mode_slot1_hold_slot2_toggle_independence() {
    let stt = Arc::new(MockSttProvider::returning("slot1 hold session"));
    let (orch, output_target, _paste, _err, _event_bus, _, _) =
        make_orchestrator_with_modes(stt, RecordingMode::Hold, RecordingMode::Toggle);

    // Slot-1 press starts a Hold-mode session.
    orch.on_press(HotkeySlot::One).await;
    assert!(!orch.is_idle().await);

    // A Slot-2 press during Slot-1 Hold-recording: cross-slot guard discards.
    // Pre-fix this would have hit the Toggle-stop arm via Slot-1's mode lookup
    // (no slot check) — but with cross-slot guard it returns silently.
    orch.on_press(HotkeySlot::Two).await;
    assert!(!orch.is_idle().await, "Slot-2 press must not affect Slot-1 Hold-recording");

    // Slot-2 release: cross-slot guard discards (Slot-1 still owns).
    orch.on_release(HotkeySlot::Two).await;
    assert!(!orch.is_idle().await, "Slot-2 release must not affect Slot-1 Hold-recording");

    // Slot-1 release legitimately stops Hold-recording.
    orch.on_release(HotkeySlot::One).await;
    wait_for_delivery(&output_target).await;
    assert_eq!(output_target.last_delivered().as_deref(), Some("slot1 hold session"));
}
