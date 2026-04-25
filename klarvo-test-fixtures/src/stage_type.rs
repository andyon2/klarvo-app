use klarvo_core::pipeline::PipelineStageType;

/// Construct a [`PipelineStageType::Passthrough`] value for use in serde-roundtrip tests
/// (1B.2 Manifest-Parser) without manual struct-construction boilerplate.
#[cfg(feature = "stage-passthrough")]
pub fn stage_type_passthrough() -> PipelineStageType {
    PipelineStageType::Passthrough
}

/// Construct a [`PipelineStageType::Stt`] value for use in serde-roundtrip tests
/// (1B.2 Manifest-Parser) without manual struct-construction boilerplate.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "stage-stt")]
/// # {
/// use klarvo_test_fixtures::stage_type_stt;
/// let t = stage_type_stt("groq");
/// # }
/// ```
#[cfg(feature = "stage-stt")]
pub fn stage_type_stt(plugin_id: &str) -> PipelineStageType {
    PipelineStageType::Stt { plugin_id: plugin_id.to_string() }
}

/// Construct a [`PipelineStageType::Cleanup`] value for use in serde-roundtrip tests
/// (1B.2 Manifest-Parser) without manual struct-construction boilerplate.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "stage-cleanup")]
/// # {
/// use klarvo_test_fixtures::stage_type_cleanup;
/// let t = stage_type_cleanup("verbatim");
/// # }
/// ```
#[cfg(feature = "stage-cleanup")]
pub fn stage_type_cleanup(plugin_id: &str) -> PipelineStageType {
    PipelineStageType::Cleanup { plugin_id: plugin_id.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "stage-passthrough")]
    #[test]
    fn stage_type_passthrough_returns_passthrough_variant() {
        let t = stage_type_passthrough();
        assert!(matches!(t, PipelineStageType::Passthrough));
    }

    #[cfg(feature = "stage-stt")]
    #[test]
    fn stage_type_stt_returns_stt_variant_with_plugin_id() {
        let t = stage_type_stt("groq");
        assert!(matches!(t, PipelineStageType::Stt { ref plugin_id } if plugin_id == "groq"));
    }

    #[cfg(feature = "stage-cleanup")]
    #[test]
    fn stage_type_cleanup_returns_cleanup_variant_with_plugin_id() {
        let t = stage_type_cleanup("verbatim");
        assert!(matches!(t, PipelineStageType::Cleanup { ref plugin_id } if plugin_id == "verbatim"));
    }
}
