use std::sync::{Arc, Mutex};

use klarvo_core::audio::{AudioBuffer, AudioEvent, AudioSource, CaptureConfig, DEFAULT_AUDIOEVENT_CAPACITY};
use klarvo_core::audio::vad::VadDecision;
use klarvo_core::manifest::parse_from_str;
use klarvo_core::pipeline::{run_capture_session, StageData};
use klarvo_core::registry::PluginRegistry;
use klarvo_test_fixtures::{MockAudioSource, MockSttProvider, MockVadProvider};
use tokio::sync::broadcast;

/// Manifest with stt{mock-stt} → cleanup{verbatim}: used by all capture_session tests
/// that exercise the full pipeline path.
const MANIFEST_STT_VERBATIM: &str = r#"schema_version = 1

[[pipeline.stages]]
type = "stt"
plugin_id = "mock-stt"

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
"#;

/// Happy-path: audio flows through VAD gate, speech segment is captured, and
/// run_pipeline returns Ok(Some(StageData::Text("hello"))).
/// Also verifies ts_ms_start/ts_ms_end on the AudioBuffer passed to STT (D1 Option A).
#[tokio::test]
async fn capture_session_happy_path() {
    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
    let mut source = MockAudioSource::with_synthetic_chunks(5, 1024, 64);
    let _handle = source.start(config).await.unwrap();

    let manifest = parse_from_str(MANIFEST_STT_VERBATIM).expect("MANIFEST_STT_VERBATIM must parse");

    let captured = Arc::new(Mutex::new(None::<AudioBuffer>));
    let mut registry = PluginRegistry::new();
    registry.register_stt(
        "mock-stt",
        Arc::new(MockSttProvider::with_capture("hello", captured.clone())),
    );
    registry.register_cleanup("verbatim", Arc::new(klarvo_plugin_verbatim::Verbatim::new()));

    let mut vad = MockVadProvider::with_decisions(vec![
        VadDecision::Silence,
        VadDecision::SpeechStart { ts_ms: 64 },
        VadDecision::Speech,
        VadDecision::SpeechEnd { ts_ms: 256, duration_ms: 192 },
        VadDecision::Silence,
    ]);

    let result = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect("run_capture_session must succeed");

    let StageData::Text(text) = result.expect("must return Some(StageData)") else {
        panic!("expected StageData::Text, got Audio variant");
    };
    assert_eq!(text, "hello");

    let buf = captured.lock().unwrap().take().expect("AudioBuffer must have been captured by MockSttProvider");
    assert_eq!(buf.ts_ms_start, 64, "ts_ms_start must equal SpeechStart.ts_ms");
    assert_eq!(buf.ts_ms_end, 256, "ts_ms_end must equal SpeechEnd.ts_ms");
}

/// Closed-before-speech: sender dropped immediately (no events sent).
/// run_capture_session must return Ok(None) — accidental hotkey-trigger path.
#[tokio::test]
async fn capture_session_closed_before_speech() {
    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    drop(tx); // immediate channel close — receiver sees RecvError::Closed immediately

    let manifest = parse_from_str(
        r#"schema_version = 1

[[pipeline.stages]]
type = "passthrough"
"#,
    )
    .expect("passthrough manifest must parse");

    let registry = PluginRegistry::new();
    let mut vad = MockVadProvider::with_decisions(vec![]);

    let result = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect("must not error on clean close");

    assert!(result.is_none(), "expected Ok(None) — no speech detected before close");
}

/// Closed-mid-speech: MockAudioSource emits 2 chunks then drops the sender.
/// VAD returns SpeechStart then Speech (no SpeechEnd). run_capture_session must
/// handle Closed-mid-Speech as SpeechEnd-equivalent and return Ok(Some(StageData::Text(_))).
#[tokio::test]
async fn capture_session_closed_mid_speech() {
    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
    // chunk_interval_ms=0: emit as fast as possible (yield_now between chunks)
    let mut source = MockAudioSource::with_synthetic_chunks(2, 1024, 0);
    let _handle = source.start(config).await.unwrap();

    let manifest = parse_from_str(MANIFEST_STT_VERBATIM).expect("MANIFEST_STT_VERBATIM must parse");

    let mut registry = PluginRegistry::new();
    registry.register_stt("mock-stt", Arc::new(MockSttProvider::returning("partial")));
    registry.register_cleanup("verbatim", Arc::new(klarvo_plugin_verbatim::Verbatim::new()));

    let mut vad = MockVadProvider::with_decisions(vec![
        VadDecision::SpeechStart { ts_ms: 0 },
        VadDecision::Speech,
    ]);

    let result = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect("must not error on closed-mid-speech");

    assert!(
        matches!(result, Some(StageData::Text(_))),
        "expected Ok(Some(StageData::Text(_))) for closed-mid-speech path"
    );
}
