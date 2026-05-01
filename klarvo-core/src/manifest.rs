//! Pipeline manifest: embedded TOML parse + schema-version-first two-pass validation.
//!
//! # Boot-Time Parse Flow
//!
//! 1. `include_str!` embeds `pipeline-manifest.toml` at compile time (`EMBEDDED_MANIFEST`).
//! 2. At app boot, `parse_embedded()` runs the two-pass parser:
//!    - **Pass 1**: TOML syntax parse → `VersionPeek` (captures `schema_version` + raw `pipeline`
//!      value). Syntax errors fail with `AppError::kind::PipelineValidation` +
//!      `keys::TOML_PARSE_FAILURE`.
//!    - **Pass 2**: `schema_version != 1` fails with `AppError::kind::PipelineValidation` +
//!      `keys::SCHEMA_VERSION_UNSUPPORTED`. Cause-chain carries received vs expected versions.
//!    - **Pass 3** (only if version OK): resolves `pipeline` value into `PipelineSpec` via
//!      serde + `PipelineStageType` compile-time registry (1B.1). Unknown stage-type tags fail
//!      with `AppError::kind::PipelineValidation` + `keys::UNKNOWN_STAGE_TYPE`.
//!
//! # Hard-Fail Invariant
//!
//! No `warn!+skip`, no `_ => …` fallback, no silent continuation on any parse error.
//! This is `feedback_manifest_compile_contract` enforced at the parser level.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppErrorKind};
use crate::pipeline::stage::PipelineStageType;

/// i18n keys emitted by the manifest parser.
///
/// All keys validated via `debug_assert!(klarvo_core::i18n::is_key(KEY))` at emission sites.
/// Keys are grep-safe and rename-safe static references for shell translation tables (Epic 4).
pub mod keys {
    /// TOML syntax or structural parse failure before schema-version check.
    pub const TOML_PARSE_FAILURE: &str = "error.pipeline.toml_parse_failure";
    /// `schema_version` field is present and a valid integer but not an accepted value.
    pub const SCHEMA_VERSION_UNSUPPORTED: &str = "error.pipeline.schema_version_unsupported";
    /// A stage entry's `type` tag does not match any variant in the compile-time registry.
    pub const UNKNOWN_STAGE_TYPE: &str = "error.pipeline.unknown_stage_type";
}

/// The `pipeline-manifest.toml` embedded at compile-time via `include_str!`.
///
/// Removing or renaming `pipeline-manifest.toml` at the workspace root causes a `cargo build`
/// failure (`include_str!` IO-error) — the intentional Compile-Time-Hard-Fail-Mechanism,
/// complementary to the Boot-Time-Parse-Hard-Fail in `parse_embedded`.
pub const EMBEDDED_MANIFEST: &str = include_str!("../../pipeline-manifest.toml");

/// The parsed pipeline manifest. Produced by `parse_embedded` or `parse_from_str`.
///
/// # Contract
///
/// `parse_embedded` never succeeds if `pipeline-manifest.toml` lacks a `schema_version` field
/// or references an unknown stage-type — both fail fast at boot with
/// `AppError::kind::PipelineValidation`.
///
/// Cross-references: 1B.1 `PipelineStageType` compile-time registry, 1B.5 Executor dispatch,
/// Epic 5 FR32 `cargo xtask manifest-strict` gate, Epic 4 error-surface (i18n key resolution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineManifest {
    /// Schema version: must be exactly `1` in Phase-1. Range-checks and wildcard accepts
    /// are intentionally absent — Phase-2+ manifests must bump the version explicitly.
    pub schema_version: u32,
    /// The ordered pipeline specification parsed against the 1B.1 stage-type registry.
    pub pipeline: PipelineSpec,
}

/// Ordered list of pipeline stage entries.
///
/// Each `PipelineStageType` variant corresponds to a Cargo feature-gated stage class.
/// Runtime-Dispatch and Plugin-Registry-Lookup-Hard-Fail happen in Story 1B.5 — this struct
/// is the parse-layer output only. Type-Chaining-Compat-Check is also 1B.5-scope
/// (architecture.md:234-238 mandates no parse-layer check).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Ordered pipeline stage entries. Each entry resolves to a plugin-registry lookup at
    /// boot-time (1B.5); an unregistered `plugin_id` is a 1B.5-Executor-Hard-Fail.
    pub stages: Vec<PipelineStageType>,
}

/// Internal first-pass struct: captures `schema_version` for version-gate before full parse.
///
/// Schema-version is validated before stage-type resolution so that a user authoring a
/// `schema_version=2` manifest (Phase-2) doesn't get a bogus `unknown_stage_type`-error for
/// a stage that is valid under v2 but not v1.
#[derive(Deserialize)]
struct VersionPeek {
    schema_version: u32,
    pipeline: toml::Value,
}

/// Parse the compile-time-embedded `pipeline-manifest.toml`.
///
/// Runs the two-pass parser against `EMBEDDED_MANIFEST`. Fails with
/// `AppError::kind::PipelineValidation` on TOML syntax errors, schema-version mismatch, or
/// unknown stage-types — no silent fallback, no warn+skip.
///
/// Cross-references: 1B.5 Executor (`run` consumes the returned `PipelineManifest`).
pub fn parse_embedded() -> Result<PipelineManifest, AppError> {
    parse_from_str(EMBEDDED_MANIFEST)
}

/// Parse a TOML manifest string into a `PipelineManifest`.
///
/// Used by `cargo xtask manifest-strict` (Epic 5 FR32) to exercise bad-input scenarios at
/// harness-compile-time — not a runtime user-facing API.
///
/// Runs the same two-pass validation as `parse_embedded`. Fails fast on any TOML syntax error,
/// schema-version mismatch, or unknown stage-type. No `_` wildcard fallback in stage resolution.
pub fn parse_from_str(toml_src: &str) -> Result<PipelineManifest, AppError> {
    // Pass 1: syntax parse + capture schema_version and raw pipeline value.
    // Missing/non-u32 schema_version → TOML_PARSE_FAILURE (structurally invalid manifest).
    let peek: VersionPeek = toml::from_str(toml_src).map_err(|e| {
        debug_assert!(crate::i18n::is_key(keys::TOML_PARSE_FAILURE));
        AppError {
            kind: AppErrorKind::PipelineValidation,
            message: format!("manifest TOML parse failure: {e}"),
            user_message: Some(keys::TOML_PARSE_FAILURE.into()),
            retryable: false,
        }
    })?;

    // Pass 2: schema_version gate. Only version 1 accepted in Phase-1.
    // Valid integer but wrong value → SCHEMA_VERSION_UNSUPPORTED.
    if peek.schema_version != 1 {
        debug_assert!(crate::i18n::is_key(keys::SCHEMA_VERSION_UNSUPPORTED));
        return Err(AppError {
            kind: AppErrorKind::PipelineValidation,
            message: format!(
                "unsupported schema_version: got {}, expected 1",
                peek.schema_version
            ),
            user_message: Some(keys::SCHEMA_VERSION_UNSUPPORTED.into()),
            retryable: false,
        });
    }

    // Pass 3: resolve pipeline value against PipelineStageType compile-time registry (1B.1).
    // Re-serialize captured pipeline Value → TOML string, then parse as PipelineSpec.
    // Unknown type-tags fail here with cause-chain from serde (tag + position context).
    let pipeline_str = toml::to_string_pretty(&peek.pipeline)
        .unwrap_or_else(|e| unreachable!("toml::Value parsed from valid TOML is always re-serializable: {e}"));
    let spec: PipelineSpec = toml::from_str(&pipeline_str).map_err(|e| {
        debug_assert!(crate::i18n::is_key(keys::UNKNOWN_STAGE_TYPE));
        AppError {
            kind: AppErrorKind::PipelineValidation,
            message: format!("unknown stage type in pipeline: {e}"),
            user_message: Some(keys::UNKNOWN_STAGE_TYPE.into()),
            retryable: false,
        }
    })?;

    Ok(PipelineManifest {
        schema_version: peek.schema_version,
        pipeline: spec,
    })
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::error::AppErrorKind;

    #[test]
    fn valid_passthrough_manifest_parses() {
        let toml = r#"
schema_version = 1

[[pipeline.stages]]
type = "passthrough"
"#;
        let m = parse_from_str(toml).expect("parses");
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.pipeline.stages.len(), 1);
        assert!(matches!(m.pipeline.stages[0], PipelineStageType::Passthrough));
    }

    #[cfg(feature = "stage-stt")]
    #[test]
    fn stt_stage_with_plugin_id_parses() {
        let toml = r#"
schema_version = 1

[[pipeline.stages]]
type = "stt"
plugin_id = "groq"
"#;
        let m = parse_from_str(toml).expect("parses");
        assert!(matches!(
            &m.pipeline.stages[0],
            PipelineStageType::Stt { plugin_id } if plugin_id == "groq"
        ));
    }

    #[cfg(feature = "stage-cleanup")]
    #[test]
    fn cleanup_stage_with_plugin_id_parses() {
        let toml = r#"
schema_version = 1

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
"#;
        let m = parse_from_str(toml).expect("parses");
        assert!(matches!(
            &m.pipeline.stages[0],
            PipelineStageType::Cleanup { plugin_id } if plugin_id == "verbatim"
        ));
    }

    #[test]
    fn schema_version_2_rejected_with_unsupported_key() {
        let toml = r#"
schema_version = 2

[[pipeline.stages]]
type = "passthrough"
"#;
        let err = parse_from_str(toml).expect_err("must reject v2");
        assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
        assert_eq!(
            err.user_message.as_deref(),
            Some(keys::SCHEMA_VERSION_UNSUPPORTED)
        );
        assert!(err.message.contains("2"), "cause-chain must carry received version");
        assert!(err.message.contains("1"), "cause-chain must carry expected version");
    }

    #[test]
    fn missing_schema_version_is_toml_parse_failure() {
        let toml = r#"
[[pipeline.stages]]
type = "passthrough"
"#;
        let err = parse_from_str(toml).expect_err("must reject missing schema_version");
        assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
        assert_eq!(
            err.user_message.as_deref(),
            Some(keys::TOML_PARSE_FAILURE)
        );
    }

    #[test]
    fn invalid_toml_syntax_is_toml_parse_failure() {
        let toml = "schema_version = [[[broken";
        let err = parse_from_str(toml).expect_err("must reject invalid TOML");
        assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
        assert_eq!(
            err.user_message.as_deref(),
            Some(keys::TOML_PARSE_FAILURE)
        );
    }

    #[test]
    fn unknown_stage_type_rejected_with_unknown_stage_key() {
        let toml = r#"
schema_version = 1

[[pipeline.stages]]
type = "quantum-fft"
"#;
        let err = parse_from_str(toml).expect_err("must reject unknown stage");
        assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
        assert_eq!(
            err.user_message.as_deref(),
            Some(keys::UNKNOWN_STAGE_TYPE)
        );
        assert!(
            err.message.contains("quantum-fft") || err.message.contains("unknown"),
            "cause-chain must reference offending type: {}",
            err.message
        );
    }

    #[test]
    fn parse_embedded_smoke_test() {
        let m = parse_embedded().expect("embedded manifest must parse");
        assert_eq!(m.schema_version, 1);
        assert!(!m.pipeline.stages.is_empty());
    }
}
