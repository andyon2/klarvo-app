use std::sync::Arc;
use std::time::Duration;

use klarvo_core::audio::vad::VadDecision;
use klarvo_core::event::{Event, EventBus};
use klarvo_core::manifest::parse_from_str as parse_manifest;
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

/// Shared setup: builds a SessionOrchestrator with controllable mocks.
/// Returns (orchestrator, output_target, paste_backend, error_emitter, event_bus).
fn make_orchestrator(
    stt: Arc<dyn klarvo_core::traits::SttProvider>,
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
    let event_bus = Arc::new(EventBus::new(64));

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
    );

    (orch, output_target, paste_backend, error_emitter, event_bus)
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

    // Verify recording-state events are emitted on the EventBus.
    let first = rx.try_recv().expect("RecordingStarted must be in bus");
    assert!(matches!(first, Event::RecordingStarted { .. }), "first event must be RecordingStarted");
    let second = rx.try_recv().expect("RecordingStopped must be in bus");
    assert!(matches!(second, Event::RecordingStopped { .. }), "second event must be RecordingStopped");
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
