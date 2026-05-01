#![allow(clippy::disallowed_methods)]

use std::sync::Arc;

use klarvo_core::error::AppErrorKind;
use klarvo_core::manifest::parse_from_str;
use klarvo_core::pipeline::{self, StageData};
use klarvo_core::pipeline::executor::keys;
use klarvo_core::registry::PluginRegistry;
use klarvo_plugin_verbatim::Verbatim;
use klarvo_test_fixtures::manifest::valid_passthrough_verbatim_manifest_toml;

/// Happy-Path: passthrough → verbatim pipeline with Text input passes through unchanged.
#[tokio::test]
async fn happy_path_passthrough_verbatim_is_identity() {
    let manifest = parse_from_str(&valid_passthrough_verbatim_manifest_toml())
        .expect("valid_passthrough_verbatim manifest must parse");

    let mut registry = PluginRegistry::new();
    registry.register_cleanup("verbatim", Arc::new(Verbatim::new()));

    let result = pipeline::run_pipeline(&manifest, &registry, StageData::Text("hello world".into()))
        .await
        .expect("happy-path pipeline must succeed");

    let StageData::Text(text) = result else {
        panic!("expected StageData::Text output, got Audio");
    };
    assert_eq!(text, "hello world");
}

/// Plugin-Not-Found-Fail: identical manifest but empty registry → PipelineValidation at boot.
///
/// Verifies that the Plugin-Registry-Lookup-Check fires for the verbatim cleanup stage and
/// the cause-chain contains the plugin_id and stage index.
#[tokio::test]
async fn plugin_not_found_fails_with_pipeline_validation_error() {
    let manifest = parse_from_str(&valid_passthrough_verbatim_manifest_toml())
        .expect("valid_passthrough_verbatim manifest must parse");

    let registry = PluginRegistry::new(); // empty — no plugin registered

    let err = pipeline::run_pipeline(&manifest, &registry, StageData::Text("anything".into()))
        .await
        .expect_err("must fail when required plugin is not registered");

    assert!(
        matches!(err.kind, AppErrorKind::PipelineValidation),
        "expected PipelineValidation, got {:?}",
        err.kind
    );
    assert_eq!(
        err.user_message.as_deref(),
        Some(keys::PLUGIN_NOT_FOUND),
        "user_message must be PLUGIN_NOT_FOUND key"
    );
    assert!(
        err.message.contains("verbatim"),
        "cause-chain must reference plugin_id 'verbatim': {}",
        err.message
    );
    assert!(
        err.message.contains("stage[1]"),
        "cause-chain must reference stage index 1 (cleanup is stage[1]): {}",
        err.message
    );
}

/// Type-Chaining-Mismatch-Fail: cleanup → stt manifest with Text input fails at Type-Chaining-Check.
///
/// Type-Chaining-Check runs before Plugin-Registry-Lookup-Check — type-mismatch is a
/// manifest-authoring-error independent of registry-state. Reverse order would mask
/// manifest-bugs when the required plugin is absent.
///
/// Registry contains only Verbatim (no Groq) to confirm that STAGE_TYPE_MISMATCH fires
/// before PLUGIN_NOT_FOUND even when the stt plugin is also unregistered.
#[cfg(all(feature = "stage-cleanup", feature = "stage-stt"))]
#[tokio::test]
async fn type_chaining_mismatch_fails_before_plugin_lookup() {
    let manifest = parse_from_str(
        r#"schema_version = 1

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"

[[pipeline.stages]]
type = "stt"
plugin_id = "groq"
"#,
    )
    .expect("cleanup→stt manifest must parse");

    let mut registry = PluginRegistry::new();
    // Only Verbatim registered — Groq intentionally absent.
    // Proves Type-Chaining-Check fires before Plugin-Registry-Lookup.
    registry.register_cleanup("verbatim", Arc::new(Verbatim::new()));

    let err = pipeline::run_pipeline(&manifest, &registry, StageData::Text("...".into()))
        .await
        .expect_err("must fail on type-chaining mismatch");

    assert_eq!(
        err.user_message.as_deref(),
        Some(keys::STAGE_TYPE_MISMATCH),
        "must be STAGE_TYPE_MISMATCH (not PLUGIN_NOT_FOUND) — type-chaining check runs first"
    );
    assert!(
        matches!(err.kind, AppErrorKind::PipelineValidation),
        "expected PipelineValidation, got {:?}",
        err.kind
    );
}
