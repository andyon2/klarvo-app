/// Forcing-Sentinel: every `plugin_id` in the embedded `pipeline-manifest.toml` must be
/// registered in a fresh `PluginRegistry` by the corresponding plugin's `register()` call.
///
/// This test fails at `cargo test` if:
/// - The manifest is updated to reference a new `plugin_id` without wiring it up in the shell
/// - A `register()` implementation stops inserting the plugin under the expected ID
///
/// Story 12.1 context: guards against manifest/registry drift that caused the original
/// no-op pipeline (passthrough-only manifest while Groq was committed but not registered).
///
/// Note: `PipelineStageType::Stt` and `PipelineStageType::Cleanup` are always available here
/// because `klarvo-plugin-groq` depends on `klarvo-core` with its default features, which
/// include `stage-stt` and `stage-cleanup`. No `#[cfg]` gating needed in this crate's tests.
use std::sync::Arc;

use klarvo_core::manifest::parse_embedded;
use klarvo_core::pipeline::stage::PipelineStageType;
use klarvo_core::registry::PluginRegistry;
use klarvo_test_fixtures::InMemoryKeyStore;

#[test]
fn embedded_manifest_stt_plugin_ids_are_registered() {
    let manifest = parse_embedded().expect("embedded manifest must parse cleanly");
    let stt_count = manifest
        .pipeline
        .stages
        .iter()
        .filter(|s| matches!(s, PipelineStageType::Stt { .. }))
        .count();
    assert!(
        stt_count > 0,
        "embedded manifest has no STT stage — manifest regressed away from production wire-up \
         (the original drift class Story 12.1 was designed to prevent)"
    );
    for stage in &manifest.pipeline.stages {
        if let PipelineStageType::Stt { plugin_id } = stage {
            let mut registry = PluginRegistry::new();
            klarvo_plugin_groq::register(&mut registry, Arc::new(InMemoryKeyStore::empty()));
            assert!(
                registry.stt(plugin_id).is_some(),
                "STT plugin '{plugin_id}' referenced in embedded manifest is NOT registered \
                 by klarvo_plugin_groq::register() — manifest/registry drift detected"
            );
        }
    }
}

#[test]
fn embedded_manifest_cleanup_plugin_ids_are_registered() {
    let manifest = parse_embedded().expect("embedded manifest must parse cleanly");
    let cleanup_count = manifest
        .pipeline
        .stages
        .iter()
        .filter(|s| matches!(s, PipelineStageType::Cleanup { .. }))
        .count();
    assert!(
        cleanup_count > 0,
        "embedded manifest has no Cleanup stage — manifest regressed away from production wire-up \
         (the original drift class Story 12.1 was designed to prevent)"
    );
    for stage in &manifest.pipeline.stages {
        if let PipelineStageType::Cleanup { plugin_id } = stage {
            let mut registry = PluginRegistry::new();
            klarvo_plugin_verbatim::register(&mut registry);
            assert!(
                registry.cleanup(plugin_id).is_some(),
                "Cleanup plugin '{plugin_id}' referenced in embedded manifest is NOT registered \
                 by klarvo_plugin_verbatim::register() — manifest/registry drift detected"
            );
        }
    }
}
