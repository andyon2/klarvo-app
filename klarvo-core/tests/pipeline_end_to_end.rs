#![allow(clippy::disallowed_methods)]

use klarvo_core::error::AppErrorKind;
use klarvo_core::manifest::parse_from_str;
use klarvo_core::pipeline::{self, StageData};
use klarvo_core::pipeline::executor::keys;
use klarvo_core::registry::PluginRegistry;

/// Passthrough behavior test — uses an explicit passthrough manifest via `parse_from_str`
/// so this test is independent of the embedded production manifest.
///
/// The embedded manifest no longer contains a passthrough stage (it has stt + cleanup
/// since Story 12.1). This test preserves coverage of the passthrough executor-arm.
#[tokio::test]
#[cfg(feature = "stage-passthrough")]
async fn passthrough_manifest_preserves_input() {
    let registry = PluginRegistry::new();
    let manifest = parse_from_str(
        r#"
schema_version = 1

[[pipeline.stages]]
type = "passthrough"
"#,
    )
    .expect("passthrough manifest parses");

    let input = "äh also das ist, ähm, der Verbatim-Test mit Umlauten.";
    let result = pipeline::run_pipeline(&manifest, &registry, StageData::Text(input.to_string()))
        .await
        .expect("passthrough pipeline runs");

    let StageData::Text(output) = result else {
        panic!("expected StageData::Text output");
    };
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
    let result = pipeline::run_pipeline(&manifest, &registry, StageData::Text(input.to_string()))
        .await
        .expect("cleanup pipeline runs");

    let StageData::Text(output) = result else {
        panic!("expected StageData::Text output");
    };
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

    let err = pipeline::run_pipeline(&manifest, &registry, StageData::Text("anything".to_string()))
        .await
        .expect_err("must fail when plugin not registered");

    assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
    assert_eq!(err.user_message.as_deref(), Some(keys::PLUGIN_NOT_FOUND));
    assert!(err.message.contains("not-registered"));
}
