use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use klarvo_core::audio::vad::RmsVad;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::event::{EventBus, DEFAULT_EVENT_BUS_CAPACITY};
use klarvo_core::manifest::parse_from_str as parse_manifest;
use klarvo_core::recording::RecordingMode;
use klarvo_core::time::MonotonicClock;
use klarvo_test_fixtures::{
    InMemoryOutputTarget, MockAudioSource, MockErrorEmitter, MockPasteBackend,
    MockSttProvider,
};

use klarvo_shell_orchestrator::SessionOrchestrator;

// Manifest: STT (mock-stt) → Cleanup (verbatim real plugin).
fn e2e_manifest_toml() -> &'static str {
    r#"
schema_version = 1

[[pipeline.stages]]
type = "stt"
plugin_id = "mock-stt"

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
"#
}

/// Constructs `SessionOrchestrator` with real pipeline (verbatim plugin) and real
/// `RmsVad` driven by synthetic loud audio. Mocks remain for the pure OS boundaries
/// (STT, Paste, ErrorEmitter) and for `OutputTarget` inspection.
///
/// Per Story-3.11 AC-B + Technical-Notes-§"Warum RmsVad": exercising the real
/// VAD signal-path is the whole point of the headless E2E suite — substituting
/// `MockVadProvider` would re-test mock logic, not the integration.
fn make_test_orchestrator_real_pipeline(
    stt: Arc<dyn klarvo_core::traits::SttProvider>,
) -> (
    SessionOrchestrator,
    Arc<InMemoryOutputTarget>,
    Arc<MockPasteBackend>,
    Arc<MockErrorEmitter>,
    Arc<EventBus>,
) {
    let manifest =
        Arc::new(parse_manifest(e2e_manifest_toml()).expect("e2e test manifest must parse"));

    let mut registry = klarvo_core::registry::bootstrap();
    klarvo_plugin_verbatim::register(&mut registry);
    registry.register_stt("mock-stt", stt);
    let output_target = Arc::new(InMemoryOutputTarget::new());
    registry.register_output(
        "test-output",
        Arc::clone(&output_target) as Arc<dyn klarvo_core::output::OutputTarget>,
    );
    let registry = Arc::new(registry);

    // Real `RmsVad` (Phase-1 default). `run_capture_session` calls `vad.reset()`
    // on entry, so cycle-2 starts with a fresh `is_speaking=false` regardless of
    // how cycle-1 ended (SpeechEnd vs. Closed-mid-Speech).
    let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(RmsVad::new())));

    // 10 chunks at amplitude 0.5 — well above the RMS threshold (0.01).
    // First chunk fires `VadDecision::SpeechStart`, subsequent chunks emit
    // `Speech`; on hotkey-release the broadcast channel closes and
    // `run_capture_session` triggers the pipeline via the Closed-mid-Speech path.
    let audio_source: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::AudioSource>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MockAudioSource::with_loud_chunks(
            10, 160, 0, 0.5,
        ))));

    let paste_backend = Arc::new(MockPasteBackend::new());
    let error_emitter = Arc::new(MockErrorEmitter::new());
    let clock: Arc<dyn klarvo_core::time::Clock> = Arc::new(MonotonicClock::new());
    let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));

    let mode_arc = Arc::new(tokio::sync::RwLock::new(RecordingMode::Hold));
    let orch = SessionOrchestrator::new(
        registry,
        manifest,
        audio_source,
        "test-output".to_string(),
        Arc::clone(&paste_backend) as Arc<dyn klarvo_core::output::PasteBackend>,
        Arc::clone(&error_emitter) as Arc<dyn klarvo_core::event::ErrorEmitter>,
        clock,
        vad,
        Arc::clone(&event_bus),
        mode_arc,
    );

    (orch, output_target, paste_backend, error_emitter, event_bus)
}

async fn wait_for_delivery(target: &InMemoryOutputTarget) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if !target.all_delivered().is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("delivery must occur within 5 seconds");
}

async fn wait_for_delivery_count(target: &InMemoryOutputTarget, count: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if target.all_delivered().len() >= count {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("deliveries must occur within 5 seconds");
}

// Does NOT panic on timeout — lets the caller assert on recorded().
async fn wait_for_error_or_timeout(emitter: &MockErrorEmitter, timeout: Duration) {
    let _ = tokio::time::timeout(timeout, async {
        loop {
            if !emitter.recorded().is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
}

// --- Scenario 1: Happy-Path End-to-End ---

#[tokio::test]
async fn e2e_happy_path_delivers_and_pastes() {
    let stt = Arc::new(MockSttProvider::returning("hello world"));
    let (orch, output_target, paste_backend, error_emitter, _event_bus) =
        make_test_orchestrator_real_pipeline(stt);

    orch.on_press().await;
    wait_for_delivery(&output_target).await;
    orch.on_release().await;

    assert_eq!(output_target.last_delivered().as_deref(), Some("hello world"));
    assert_eq!(paste_backend.call_count(), 1);
    assert!(error_emitter.recorded().is_empty(), "no errors expected on happy path");
}

// --- Scenario 2: Multi-Cycle Independence ---

#[tokio::test]
async fn e2e_two_cycles_are_independent() {
    let stt = Arc::new(MockSttProvider::returning("hello world"));
    let (orch, output_target, paste_backend, error_emitter, _event_bus) =
        make_test_orchestrator_real_pipeline(stt);

    // Cycle 1
    orch.on_press().await;
    orch.on_release().await;
    wait_for_delivery_count(&output_target, 1).await;

    // Cycle 2
    orch.on_press().await;
    orch.on_release().await;
    wait_for_delivery_count(&output_target, 2).await;

    assert_eq!(output_target.all_delivered().len(), 2, "exactly two deliveries expected");
    assert_eq!(paste_backend.call_count(), 2, "paste called once per cycle");
    assert!(error_emitter.recorded().is_empty(), "no errors across two cycles");
}

// --- Scenario 3: Key-Repeat Guard ---

#[tokio::test]
async fn e2e_key_repeat_guard_prevents_double_start() {
    let stt = Arc::new(MockSttProvider::returning("hello world"));
    let (orch, output_target, paste_backend, error_emitter, _event_bus) =
        make_test_orchestrator_real_pipeline(stt);

    orch.on_press().await;
    orch.on_press().await; // second press while Recording — must be discarded
    wait_for_delivery(&output_target).await;
    orch.on_release().await;

    assert_eq!(output_target.all_delivered().len(), 1, "exactly one delivery expected");
    assert_eq!(paste_backend.call_count(), 1, "paste called exactly once");
    assert!(error_emitter.recorded().is_empty(), "key-repeat must not emit errors");
}

// --- Scenario 4: Stray Release ---

#[tokio::test]
async fn e2e_stray_release_is_noop() {
    let stt = Arc::new(MockSttProvider::returning("hello world"));
    let (orch, output_target, paste_backend, error_emitter, _event_bus) =
        make_test_orchestrator_real_pipeline(stt);

    orch.on_release().await; // release without prior press

    assert!(output_target.last_delivered().is_none(), "stray release must not deliver");
    assert_eq!(paste_backend.call_count(), 0, "stray release must not paste");
    assert!(error_emitter.recorded().is_empty(), "stray release must not emit errors");
}

// --- Scenario 5: Pipeline-Mid-Fail + Recovery ---

#[tokio::test]
async fn e2e_pipeline_fail_in_cycle1_does_not_prevent_cycle2() {
    // Local helper: returns a queue of Result<String, AppError> in order.
    // Not in klarvo-test-fixtures: single consumer here; premature-abstraction-guard applies.
    struct SequencedResultStt {
        queue: tokio::sync::Mutex<std::collections::VecDeque<Result<String, AppError>>>,
    }

    #[async_trait]
    impl klarvo_core::pipeline::PipelineStage for SequencedResultStt {
        type Input = klarvo_core::audio::AudioBuffer;
        type Output = String;

        async fn process(&self, _input: Self::Input) -> Result<String, AppError> {
            self.queue.lock().await.pop_front().unwrap_or_else(|| {
                Err(AppError {
                    kind: AppErrorKind::Internal,
                    message: "SequencedResultStt queue exhausted".to_string(),
                    user_message: None,
                    retryable: false,
                })
            })
        }

        fn stage_type(&self) -> &'static str {
            "stt"
        }
    }

    #[async_trait]
    impl klarvo_core::traits::SttProvider for SequencedResultStt {}

    let stt = Arc::new(SequencedResultStt {
        queue: tokio::sync::Mutex::new(std::collections::VecDeque::from([
            Err(AppError {
                kind: AppErrorKind::UpstreamUnavailable,
                message: "cycle-1 simulated failure".to_string(),
                user_message: Some("error.stt.upstream_unavailable".to_string()),
                retryable: true,
            }),
            Ok("hello".to_string()),
        ])),
    });

    let (orch, output_target, paste_backend, error_emitter, _event_bus) =
        make_test_orchestrator_real_pipeline(stt);

    // Cycle 1 — will fail at STT
    orch.on_press().await;
    orch.on_release().await;
    wait_for_error_or_timeout(&error_emitter, Duration::from_secs(3)).await;

    assert!(!error_emitter.recorded().is_empty(), "cycle-1 must emit at least one error");
    assert_eq!(paste_backend.call_count(), 0, "cycle-1 must not paste on STT failure");

    // Cycle 2 — must succeed (Orchestrator recovered to Idle after cycle-1)
    orch.on_press().await;
    orch.on_release().await;
    wait_for_delivery(&output_target).await;

    assert_eq!(output_target.all_delivered().len(), 1, "cycle-2 delivers");
    assert_eq!(paste_backend.call_count(), 1, "cycle-2 pastes");
}
