use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::{CleanupContext, CleanupInput, CleanupStyle};
use klarvo_plugin_verbatim::Verbatim;

#[tokio::test]
async fn external_crate_consumes_cleanup_style_contract() {
    let v = Verbatim::new();
    let input = CleanupInput { raw: "test".to_string(), context: CleanupContext::default() };
    let result = v.apply(input).await;
    assert_eq!(result.unwrap(), "test");
}

#[tokio::test]
async fn register_fn_wires_verbatim_into_registry() {
    let mut registry = klarvo_core::PluginRegistry::new();
    klarvo_plugin_verbatim::register(&mut registry);
    let plugin = registry
        .cleanup(klarvo_plugin_verbatim::ID)
        .expect("verbatim should be registered under its ID");
    let input = CleanupInput { raw: "via registry".to_string(), context: CleanupContext::default() };
    let result = plugin.apply(input).await;
    assert_eq!(result.unwrap(), "via registry");
}

// Verify PipelineStage::process is also accessible from external perspective.
#[tokio::test]
async fn pipeline_stage_process_accessible_externally() {
    let v = Verbatim::new();
    let input = CleanupInput::from_raw("raw access".to_string());
    let result = v.process(input).await;
    assert_eq!(result.unwrap(), "raw access");
}
