//! TOML top-level must be a table; variant-isolated roundtrip uses transparent wrapper
//! to exercise serde-tag-parsing without depending on Pipeline-Shape —
//! Pipeline-Shape-Roundtrip is 1B.2-Scope.

#![allow(clippy::disallowed_methods)]

use klarvo_core::pipeline::PipelineStageType;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(transparent)]
struct SingleStageWrapper(PipelineStageType);

#[cfg(feature = "stage-stt")]
#[test]
fn toml_roundtrip_stt() {
    let t = PipelineStageType::Stt { plugin_id: "groq".into() };
    let s = toml::to_string_pretty(&SingleStageWrapper(t.clone())).unwrap();
    let parsed: SingleStageWrapper = toml::from_str(&s).unwrap();
    assert_eq!(parsed.0, t);
}

#[cfg(feature = "stage-cleanup")]
#[test]
fn toml_roundtrip_cleanup() {
    let t = PipelineStageType::Cleanup { plugin_id: "verbatim".into() };
    let s = toml::to_string_pretty(&SingleStageWrapper(t.clone())).unwrap();
    let parsed: SingleStageWrapper = toml::from_str(&s).unwrap();
    assert_eq!(parsed.0, t);
}

#[cfg(feature = "stage-passthrough")]
#[test]
fn toml_roundtrip_passthrough() {
    let t = PipelineStageType::Passthrough;
    let s = toml::to_string_pretty(&SingleStageWrapper(t.clone())).unwrap();
    let parsed: SingleStageWrapper = toml::from_str(&s).unwrap();
    assert_eq!(parsed.0, t);
}
