use klarvo_core::manifest::embedded_default;
use klarvo_core::pipeline;
use klarvo_core::registry::PluginRegistry;

#[tokio::test]
async fn embedded_manifest_with_verbatim_plugin_is_identity() {
    let mut registry = PluginRegistry::new();
    klarvo_plugin_verbatim::register(&mut registry);

    let manifest = embedded_default();
    let input = "äh also das ist, ähm, der Verbatim-Test mit Umlauten.";

    let output = pipeline::run(&manifest, &registry, input)
        .await
        .expect("pipeline runs");

    assert_eq!(output, input);
}

#[tokio::test]
async fn missing_plugin_returns_validation_error() {
    use klarvo_core::manifest::{Manifest, Pipeline, Stage};

    let registry = PluginRegistry::new();
    let manifest = Manifest {
        manifest_version: "1".to_string(),
        pipeline: Pipeline {
            stages: vec![Stage::Cleanup {
                plugin: "not-registered".to_string(),
            }],
        },
    };

    let err = pipeline::run(&manifest, &registry, "anything")
        .await
        .expect_err("must fail when plugin not registered");
    assert!(err.message.contains("not-registered"));
}
