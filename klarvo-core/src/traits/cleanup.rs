use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::pipeline::PipelineStage;

/// Language and dictionary context forwarded to a cleanup plugin.
///
/// Captures the **Output-Language Axis** (i18n Axis-3) independently of UI language
/// and dictionary language — three independent axes in Klarvo's i18n design.
///
/// Phase-1 default is empty string for `output_language` ("unset"). Full context-passing
/// from pipeline config is Epic 4 / Phase-2+ scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CleanupContext {
    /// BCP-47 output language tag (e.g., `"de"`, `"en-US"`). Empty string means
    /// "unset" — plugin uses its own default, NOT assumed English.
    pub output_language: String,
    /// Keys of custom dictionary plugins to consult. Empty in Phase-1 default.
    #[serde(default)]
    pub dictionary_refs: Vec<String>,
}

/// Input to a cleanup stage: raw transcription text plus language/dictionary context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupInput {
    /// Raw transcription string as produced by the STT stage.
    pub raw: String,
    /// Language and dictionary context for this cleanup call.
    pub context: CleanupContext,
}

impl CleanupInput {
    /// Construct with empty context (Phase-1 default). This is the dispatch-arm-internal
    /// constructor used by the Executor (Story 1B.5) when wrapping `StageData::Text`.
    pub fn from_raw(raw: String) -> Self {
        Self { raw, context: CleanupContext::default() }
    }
}

/// Cleanup provider: transforms raw transcription text into polished output.
///
/// # Intent
///
/// `CleanupStyle` is the text-to-text boundary in the Phase-1 pipeline. Implementations
/// receive a [`CleanupInput`] (raw transcription + language context) and produce a
/// formatted/cleaned string (punctuation, capitalisation, filler-word removal, etc.).
///
/// # Method Contract
///
/// - **Input pre-condition**: `CleanupInput.raw` may be empty (silence-transcribed-as-empty);
///   implementations MUST handle empty input without panicking.
/// - **Output**: Possibly shorter/transformed `input.raw`. Never `None`.
/// - **Error variants**: `AppError::kind::Network` / `::Auth` / `::RateLimit` for
///   LLM-backed cleanup; `Ok` for pure-local implementations like `klarvo-plugin-verbatim`.
///
/// # Implementing
///
/// Implement [`PipelineStage`] with `Input = CleanupInput, Output = String` and provide
/// an empty `impl CleanupStyle for YourType {}` to inherit the default [`Self::apply`].
/// Override `apply` only for custom pre/post-processing beyond the raw `process` call.
///
/// # Mock for Tests
///
/// Use `klarvo_test_fixtures::MockCleanupStyle` for unit-testing pipeline stages.
///
/// # Cross-References
///
/// - Reference implementation: `klarvo-plugin-verbatim` (identity/no-op cleanup).
/// - Executor dispatch: `klarvo-core::pipeline::run_pipeline` (Story 1B.5).
/// - [`CleanupContext`]: captures Output-Language-Axis (i18n Axis-3, Epic 4 scope).
/// - [`PipelineStageType::Cleanup`]: Compile-time registry variant (Story 1B.1).
/// - FR6: Compile-time Stage-Registry-Set via Cargo features + enum variants.
///
/// # Phase-1 Stability
///
/// Signature is **Phase-1-locked**. Post-close changes trigger Breaking-Change-Review.
#[async_trait]
pub trait CleanupStyle: PipelineStage<Input = CleanupInput, Output = String> {
    /// Apply cleanup to `input`, returning the transformed text or an `AppError`.
    /// Defaults to delegating to [`PipelineStage::process`].
    ///
    /// Override to add plugin-specific pre/post-processing while preserving the
    /// standard [`PipelineStage`] contract for Executor dispatch.
    async fn apply(&self, input: CleanupInput) -> Result<String, AppError> {
        self.process(input).await
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_context_default_is_unset() {
        let ctx = CleanupContext::default();
        assert_eq!(ctx.output_language, "");
        assert!(ctx.dictionary_refs.is_empty());
    }

    #[test]
    fn cleanup_input_serde_roundtrip() {
        let input = CleanupInput {
            raw: "hello world".to_string(),
            context: CleanupContext {
                output_language: "de".to_string(),
                dictionary_refs: vec!["medical".to_string()],
            },
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: CleanupInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, back);
    }

    #[test]
    fn cleanup_input_from_raw_has_empty_context() {
        let input = CleanupInput::from_raw("test".to_string());
        assert_eq!(input.raw, "test");
        assert_eq!(input.context, CleanupContext::default());
    }
}
