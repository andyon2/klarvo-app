//! E2E integration-test for Groq-Plugin consumed via `run_capture_session`. Layers:
//! MockAudioSource + MockVadProvider + InMemoryKeyStore (from `klarvo-test-fixtures`) +
//! wiremock mock-HTTP-server. First test-layer where the full dictation-path
//! (Audio-Capture → VAD-Gate → STT → Cleanup → StageData::Text) runs end-to-end in a
//! single async test. Bootstrap-Wire-Up (real WindowsKeystore + real AudioSource) is
//! Epic 3 scope. OutputTarget-Delivery via InMemoryOutputTarget: Story 2.4.

use std::sync::Arc;

use klarvo_core::audio::{AudioSource, CaptureConfig, DEFAULT_AUDIOEVENT_CAPACITY};
use klarvo_core::audio::vad::VadDecision;
use klarvo_core::error::AppErrorKind;
use klarvo_core::keystore::KeyStore;
use klarvo_core::manifest::parse_from_str;
use klarvo_core::pipeline::{run_capture_session, StageData};
use klarvo_core::registry::PluginRegistry;
use klarvo_plugin_groq::{Groq, GROQ_API_KEY_ID, keys};
use klarvo_core::output::OutputTarget;
use klarvo_test_fixtures::{GroqMockServer, InMemoryKeyStore, InMemoryOutputTarget, MockAudioSource, MockVadProvider};
use secrecy::SecretString;
use tokio::sync::broadcast;

const MANIFEST_GROQ_VERBATIM: &str = r#"schema_version = 1

[[pipeline.stages]]
type = "stt"
plugin_id = "groq"

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
"#;

fn vad_one_utterance() -> MockVadProvider {
    MockVadProvider::with_decisions(vec![
        VadDecision::SpeechStart { ts_ms: 0 },
        VadDecision::Speech,
        VadDecision::SpeechEnd { ts_ms: 1000, duration_ms: 1000 },
    ])
}

#[tokio::test]
async fn e2e_groq_happy_path() {
    let server = GroqMockServer::start().await;
    server.with_success_response("hello world").await;

    let ks: Arc<dyn KeyStore> = Arc::new(InMemoryKeyStore::with_pairs([
        (GROQ_API_KEY_ID, SecretString::new("test-key".into())),
    ]));

    let mut registry = PluginRegistry::new();
    registry.register_stt(
        "groq",
        Arc::new(Groq::new_with_client(Arc::clone(&ks), server.uri(), reqwest::Client::new())),
    );
    registry.register_cleanup("verbatim", Arc::new(klarvo_plugin_verbatim::Verbatim::new()));

    let manifest = parse_from_str(MANIFEST_GROQ_VERBATIM).expect("manifest must parse");

    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
    let mut source = MockAudioSource::with_synthetic_chunks(3, 1024, 0);
    let _handle = source.start(config).await.unwrap();

    let mut vad = vad_one_utterance();

    let result = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect("run_capture_session must not error");

    let StageData::Text(text) = result.expect("must return Some(StageData)") else {
        panic!("expected StageData::Text");
    };
    assert_eq!(text, "hello world");
}

#[tokio::test]
async fn e2e_groq_upstream_5xx_propagates() {
    let server = GroqMockServer::start().await;
    server.with_status(503, "Service Unavailable").await;

    let ks: Arc<dyn KeyStore> = Arc::new(InMemoryKeyStore::with_pairs([
        (GROQ_API_KEY_ID, SecretString::new("test-key".into())),
    ]));

    let mut registry = PluginRegistry::new();
    registry.register_stt(
        "groq",
        Arc::new(Groq::new_with_client(Arc::clone(&ks), server.uri(), reqwest::Client::new())),
    );
    registry.register_cleanup("verbatim", Arc::new(klarvo_plugin_verbatim::Verbatim::new()));

    let manifest = parse_from_str(MANIFEST_GROQ_VERBATIM).expect("manifest must parse");

    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
    let mut source = MockAudioSource::with_synthetic_chunks(3, 1024, 0);
    let _handle = source.start(config).await.unwrap();

    let mut vad = vad_one_utterance();

    let err = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect_err("must propagate Err from STT stage");

    assert!(matches!(err.kind, AppErrorKind::UpstreamUnavailable));
    assert_eq!(err.user_message, Some(keys::UPSTREAM_5XX.to_string()));
}

#[tokio::test]
async fn e2e_groq_key_not_configured_propagates() {
    let server = GroqMockServer::start().await;
    // No mock expectations — transcribe() should fail before any HTTP call.

    let ks: Arc<dyn KeyStore> = Arc::new(InMemoryKeyStore::empty());

    let mut registry = PluginRegistry::new();
    registry.register_stt(
        "groq",
        Arc::new(Groq::new_with_client(Arc::clone(&ks), server.uri(), reqwest::Client::new())),
    );
    registry.register_cleanup("verbatim", Arc::new(klarvo_plugin_verbatim::Verbatim::new()));

    let manifest = parse_from_str(MANIFEST_GROQ_VERBATIM).expect("manifest must parse");

    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
    let mut source = MockAudioSource::with_synthetic_chunks(3, 1024, 0);
    let _handle = source.start(config).await.unwrap();

    let mut vad = vad_one_utterance();

    let err = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect_err("must propagate Err when API key is absent");

    assert!(matches!(err.kind, AppErrorKind::UpstreamUnavailable));
    assert_eq!(err.user_message, Some(keys::KEY_NOT_CONFIGURED.to_string()));
}

#[tokio::test]
async fn e2e_dictation_with_output_target() {
    let server = GroqMockServer::start().await;
    server.with_success_response("hello world").await;

    let ks: Arc<dyn klarvo_core::keystore::KeyStore> = Arc::new(InMemoryKeyStore::with_pairs([
        (GROQ_API_KEY_ID, SecretString::new("test-key".into())),
    ]));

    let mut registry = PluginRegistry::new();
    registry.register_stt(
        "groq",
        Arc::new(Groq::new_with_client(Arc::clone(&ks), server.uri(), reqwest::Client::new())),
    );
    registry.register_cleanup("verbatim", Arc::new(klarvo_plugin_verbatim::Verbatim::new()));

    // Arc-Split: retain typed handle for assertion access after delivery.
    let sink = Arc::new(InMemoryOutputTarget::new());
    let sink_as_trait: Arc<dyn OutputTarget> = Arc::clone(&sink) as Arc<dyn OutputTarget>;
    registry.register_output("clipboard", sink_as_trait);

    let manifest = parse_from_str(MANIFEST_GROQ_VERBATIM).expect("manifest must parse");

    let (tx, rx) = broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY);
    let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
    let mut source = MockAudioSource::with_synthetic_chunks(3, 1024, 0);
    let _handle = source.start(config).await.unwrap();

    let mut vad = vad_one_utterance();

    let result = run_capture_session(rx, &mut vad, &manifest, &registry)
        .await
        .expect("run_capture_session must not error");

    let StageData::Text(text) = result.expect("must return Some(StageData)") else {
        panic!("expected StageData::Text");
    };

    let out = registry.output("clipboard").expect("clipboard output must be registered");
    out.deliver(&text).await.expect("InMemoryOutputTarget::deliver() is always Ok");

    assert_eq!(sink.last_delivered(), Some("hello world".to_owned()));
}
