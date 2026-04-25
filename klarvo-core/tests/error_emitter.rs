use klarvo_core::event::ErrorEmitter;
use klarvo_test_fixtures::MockErrorEmitter;

#[tokio::test]
async fn mock_emitter_records_key_and_ts() {
    let emitter = MockErrorEmitter::new();
    emitter.emit_error("error.pipeline.unknown_stage", 100).await;
    emitter.emit_error("error.audio.device_unavailable", 200).await;

    let recorded = emitter.recorded();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0], ("error.pipeline.unknown_stage".to_string(), 100));
    assert_eq!(recorded[1], ("error.audio.device_unavailable".to_string(), 200));
}
