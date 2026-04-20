//! Manifest TOML helpers for `klarvo_core::manifest::parse_from_str` tests.
//!
//! Used by 1B.5 Executor integration-tests and Epic 5 FR32 xtask-harness to reproduce
//! bad-input scenarios without modifying `EMBEDDED_MANIFEST`.

/// Returns a minimal valid manifest TOML: `schema_version = 1` + single passthrough stage.
pub fn valid_minimal_manifest_toml() -> String {
    r#"schema_version = 1

[[pipeline.stages]]
type = "passthrough"
"#
    .to_string()
}

/// Returns a manifest TOML with `schema_version = 1` and a single stage with the given
/// unknown `type` tag — triggers `keys::UNKNOWN_STAGE_TYPE` on parse.
pub fn manifest_with_unknown_stage_toml(unknown_tag: &str) -> String {
    format!(
        r#"schema_version = 1

[[pipeline.stages]]
type = "{unknown_tag}"
"#
    )
}

/// Returns a manifest TOML with the given `schema_version` and a passthrough stage —
/// triggers `keys::SCHEMA_VERSION_UNSUPPORTED` when `version != 1`.
pub fn manifest_with_wrong_schema_version_toml(version: u32) -> String {
    format!(
        r#"schema_version = {version}

[[pipeline.stages]]
type = "passthrough"
"#
    )
}

/// Returns a manifest TOML with `schema_version = 1`, a passthrough stage, and a
/// `cleanup{plugin_id="verbatim"}` stage — used by 1B.5 Executor E2E happy-path test.
///
/// Requires `stage-passthrough` and `stage-cleanup` features (both enabled by default).
pub fn valid_passthrough_verbatim_manifest_toml() -> String {
    r#"schema_version = 1

[[pipeline.stages]]
type = "passthrough"

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
"#
    .to_string()
}

/// Assert that `parse_from_str` fails with the given `AppErrorKind` pattern.
///
/// Fails the test with a diagnostic if the parse unexpectedly succeeds or returns a
/// different error kind.
///
/// # Example
///
/// ```rust,ignore
/// assert_parse_fails_with_kind!(
///     &manifest_with_wrong_schema_version_toml(2),
///     klarvo_core::error::AppErrorKind::PipelineValidation
/// );
/// ```
#[macro_export]
macro_rules! assert_parse_fails_with_kind {
    ($manifest_toml:expr, $kind:pat) => {{
        let result = klarvo_core::manifest::parse_from_str($manifest_toml);
        let err = result.expect_err("expected parse_from_str to fail but it succeeded");
        assert!(
            matches!(err.kind, $kind),
            "unexpected error kind {:?}: {}",
            err.kind,
            err.message
        );
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use klarvo_core::error::AppErrorKind;
    use klarvo_core::manifest::parse_from_str;

    #[test]
    fn valid_minimal_parses_successfully() {
        let result = parse_from_str(&valid_minimal_manifest_toml());
        assert!(result.is_ok(), "valid minimal manifest must parse: {:?}", result);
    }

    #[test]
    fn unknown_stage_toml_fails_parse() {
        let toml = manifest_with_unknown_stage_toml("neural-turbo");
        let err = parse_from_str(&toml).expect_err("must reject unknown stage");
        assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
    }

    #[test]
    fn wrong_schema_version_toml_fails_parse() {
        let toml = manifest_with_wrong_schema_version_toml(99);
        let err = parse_from_str(&toml).expect_err("must reject schema version 99");
        assert!(matches!(err.kind, AppErrorKind::PipelineValidation));
        assert!(err.message.contains("99"), "error must carry received version");
    }

    #[test]
    fn assert_parse_fails_with_kind_macro_works() {
        assert_parse_fails_with_kind!(
            &manifest_with_wrong_schema_version_toml(2),
            klarvo_core::error::AppErrorKind::PipelineValidation
        );
    }

    #[test]
    fn valid_passthrough_verbatim_parses_successfully() {
        let result = parse_from_str(&valid_passthrough_verbatim_manifest_toml());
        assert!(result.is_ok(), "valid passthrough+verbatim manifest must parse: {:?}", result);
        let m = result.unwrap();
        assert_eq!(m.pipeline.stages.len(), 2, "must have exactly 2 stages");
    }
}
