use std::sync::Arc;
use std::time::Duration;

use klarvo_core::audio::vad::VadDecision;
use klarvo_core::event::{Event, EventBus, DEFAULT_EVENT_BUS_CAPACITY};
use klarvo_core::manifest::parse_from_str as parse_manifest;
use klarvo_core::recording::RecordingMode;
use klarvo_test_fixtures::{
    FakeClock, InMemoryOutputTarget, MockAudioSource, MockErrorEmitter, MockPasteBackend,
    MockSttProvider, MockVadProvider, MockCleanupStyle, QueuedMockSttProvider,
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
    );

    (orch, output_target, paste_backend, error_emitter, event_bus)
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
) {
    make_orchestrator_with_mode(stt, RecordingMode::Hold)
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

/// Poll until a RecordingCompleted event is received, or timeout.
async fn wait_for_completed(rx: &mut tokio::sync::broadcast::Receiver<Event>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match rx.try_recv() {
                Ok(Event::RecordingCompleted { .. }) => break,
                Ok(_) | Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        }
    })
    .await
    .expect("RecordingCompleted must arrive within 5 seconds");
}

#[tokio::test]
async fn test1_happy_path_press_release_delivers_and_pastes() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, output_target, paste_backend, error_emitter, event_bus) = make_orchestrator(stt);
    let mut rx = event_bus.subscribe();

    orch.on_press().await;
    wait_for_delivery(&output_target).await;
    orch.on_release().await;

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
    let (orch, output_target, _paste_backend, error_emitter, _event_bus) = make_orchestrator(stt);

    orch.on_press().await;
    orch.on_press().await; // second press — must be discarded by key-repeat guard

    // Wait for the single pipeline to deliver, then clean up.
    wait_for_delivery(&output_target).await;
    orch.on_release().await;

    // Only one delivery, no error emitted.
    assert_eq!(output_target.all_delivered().len(), 1, "exactly one delivery expected");
    assert!(error_emitter.recorded().is_empty(), "key-repeat must not emit errors");
}

#[tokio::test]
async fn test3_stray_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("hello"));
    let (orch, _output, paste_backend, error_emitter, _event_bus) = make_orchestrator(stt);

    // Release without prior press — must be a silent no-op.
    orch.on_release().await;

    assert!(!paste_backend.was_called(), "stray release must not paste");
    assert!(error_emitter.recorded().is_empty(), "stray release must not emit errors");
}

#[tokio::test]
async fn test4_pipeline_failure_emits_error_state_recovers() {
    // Empty queue → QueuedMockSttProvider returns Internal error on first call.
    let stt = Arc::new(QueuedMockSttProvider::with_transcriptions(vec![]));
    let (orch, _, paste_backend, error_emitter, _event_bus) = make_orchestrator(stt);

    orch.on_press().await;
    wait_for_error(&error_emitter).await;
    orch.on_release().await;

    assert!(!error_emitter.recorded().is_empty(), "pipeline failure must emit an error");
    assert!(!paste_backend.was_called(), "paste must not happen after STT failure");
}

// ---------------------------------------------------------------------------
// AC-4: Toggle-Modus
// ---------------------------------------------------------------------------

#[tokio::test]
async fn toggle_press_starts_recording() {
    let stt = Arc::new(MockSttProvider::returning("toggle text"));
    let (orch, _output, _paste, _err, event_bus) = make_orchestrator_with_mode(stt, RecordingMode::Toggle);
    let mut rx = event_bus.subscribe();

    orch.on_press().await;

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
    orch.on_press().await; // second press to stop
}

#[tokio::test]
async fn toggle_second_press_stops_recording() {
    let stt = Arc::new(MockSttProvider::returning("toggle delivery"));
    let (orch, output_target, paste_backend, _err, _event_bus) =
        make_orchestrator_with_mode(stt, RecordingMode::Toggle);

    orch.on_press().await; // start
    orch.on_press().await; // stop → channel closes → pipeline runs

    wait_for_delivery(&output_target).await;
    assert_eq!(output_target.last_delivered().as_deref(), Some("toggle delivery"));
    // Paste should be called (Toggle behavior same as Hold)
    assert!(paste_backend.was_called(), "paste must be called in Toggle mode");
}

#[tokio::test]
async fn toggle_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, paste_backend, error_emitter, _event_bus) =
        make_orchestrator_with_mode(stt, RecordingMode::Toggle);

    orch.on_press().await; // start recording

    // on_release in Toggle mode while recording — must be a no-op
    orch.on_release().await;

    // No paste yet, no errors from stray release
    assert!(!paste_backend.was_called());
    assert!(error_emitter.recorded().is_empty());

    // Cleanup: second press to stop
    orch.on_press().await;
}

// ---------------------------------------------------------------------------
// AC-5: AutoStop-Modus
// ---------------------------------------------------------------------------

#[tokio::test]
async fn autostop_transitions_to_idle_after_vad() {
    let stt = Arc::new(MockSttProvider::returning("autostop text"));
    let (orch, output_target, paste_backend, _err, _event_bus) =
        make_orchestrator_with_mode(stt, RecordingMode::AutoStop);

    orch.on_press().await;
    // VAD fires SpeechEnd automatically via MockVadProvider → pipeline runs → cleanup → Idle
    wait_for_delivery(&output_target).await;

    assert_eq!(output_target.last_delivered().as_deref(), Some("autostop text"));
    assert!(paste_backend.was_called(), "paste must be called in AutoStop mode");
}

#[tokio::test]
async fn autostop_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("ignored"));
    let (orch, _output, paste_backend, error_emitter, _event_bus) =
        make_orchestrator_with_mode(stt, RecordingMode::AutoStop);

    orch.on_press().await;

    // on_release in AutoStop mode while recording — must be a no-op
    orch.on_release().await;

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
    let (orch, output_target, paste_backend, _err, event_bus) =
        make_orchestrator_with_mode(stt, RecordingMode::WaitAndType);
    let mut rx = event_bus.subscribe();

    orch.on_press().await;
    wait_for_delivery(&output_target).await;
    orch.on_release().await;

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
