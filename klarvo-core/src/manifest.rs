use serde::Deserialize;
use thiserror::Error;

const EMBEDDED_DEFAULT_TOML: &str = include_str!("../../pipeline-manifest.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub manifest_version: String,
    pub pipeline: Pipeline,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Stage {
    Cleanup { plugin: String },
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest parse: {0}")]
    Parse(#[from] toml::de::Error),
}

pub fn parse(input: &str) -> Result<Manifest, ManifestError> {
    Ok(toml::from_str(input)?)
}

pub fn embedded_default() -> Manifest {
    parse(EMBEDDED_DEFAULT_TOML).expect("embedded-default pipeline-manifest.toml must parse")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_cleanup_manifest() {
        let input = r#"
manifest_version = "1"

[pipeline]
stages = [
    { type = "cleanup", plugin = "verbatim" },
]
"#;
        let m = parse(input).expect("parses");
        assert_eq!(m.manifest_version, "1");
        assert_eq!(m.pipeline.stages.len(), 1);
        match &m.pipeline.stages[0] {
            Stage::Cleanup { plugin } => assert_eq!(plugin, "verbatim"),
        }
    }

    #[test]
    fn rejects_unknown_stage_type() {
        let input = r#"
manifest_version = "1"

[pipeline]
stages = [
    { type = "stt", plugin = "groq" },
]
"#;
        let err = parse(input).expect_err("must reject unknown type");
        let msg = err.to_string();
        assert!(msg.contains("stt") || msg.contains("variant"), "unexpected error: {msg}");
    }

    #[test]
    fn rejects_missing_plugin_field() {
        let input = r#"
manifest_version = "1"

[pipeline]
stages = [
    { type = "cleanup" },
]
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn embedded_default_parses() {
        let m = embedded_default();
        assert_eq!(m.manifest_version, "1");
        assert!(!m.pipeline.stages.is_empty());
    }
}
