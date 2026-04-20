use klarvo_core::manifest::{parse_embedded, parse_from_str};
use klarvo_core::pipeline;
use klarvo_core::registry::PluginRegistry;

#[tokio::test]
async fn embedded_passthrough_manifest_preserves_input() {
    let registry = PluginRegistry::new();
    let manifest = parse_embedded().expect("embedded manifest parses");

    let input = "äh also das ist, ähm, der Verbatim-Test mit Umlauten.";
    let output = pipeline::run(&manifest, &registry, input)
        .await
        .expect("passthrough pipeline runs");

    assert_eq!(output, input);
}

#[tokio::test]
#[cfg(feature = "stage-cleanup")]
async fn cleanup_verbatim_via_parse_from_str_is_identity() {
    let mut registry = PluginRegistry::new();
    klarvo_plugin_verbatim::register(&mut registry);

    let manifest = parse_from_str(
        r#"
schema_version = 1

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
"#,
    )
    .expect("cleanup manifest parses");

    let input = "äh also das ist, ähm, der Verbatim-Test.";
    let output = pipeline::run(&manifest, &registry, input)
        .await
        .expect("cleanup pipeline runs");

    assert_eq!(output, input);
}

#[tokio::test]
#[cfg(feature = "stage-cleanup")]
async fn missing_plugin_returns_validation_error() {
    let registry = PluginRegistry::new();

    let manifest = parse_from_str(
        r#"
schema_version = 1

[[pipeline.stages]]
type = "cleanup"
plugin_id = "not-registered"
"#,
    )
    .expect("manifest parses");

    let err = pipeline::run(&manifest, &registry, "anything")
        .await
        .expect_err("must fail when plugin not registered");

    assert!(err.message.contains("not-registered"));
}
