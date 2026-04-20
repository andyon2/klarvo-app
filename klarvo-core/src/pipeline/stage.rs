use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Base composition shape for all pipeline stages (STT, Cleanup, future Phase-2+ extensions).
///
/// # Contract
///
/// 1. **Idempotency-friendly**: Implementations SHOULD be safe to call multiple times
///    with the same input. The Executor does not guarantee call deduplication.
/// 2. **Error propagation**: Plugin-specific errors MUST wrap via `AppError::from(PluginError)`.
///    Impls MUST NOT panic on expected failure conditions — return `Err(AppError)` instead.
/// 3. **Ordering**: The Executor guarantees sequential dispatch per pipeline; impls MAY
///    parallelize internally (e.g., batched STT requests) as long as they return a single
///    `Output` value to the next stage.
///
/// # Minimal Implementation Example
///
/// ```rust,ignore
/// use klarvo_core::pipeline::PipelineStage;
/// use klarvo_core::error::AppError;
/// use async_trait::async_trait;
///
/// struct IdentityPassthroughStage;
///
/// #[async_trait]
/// impl PipelineStage for IdentityPassthroughStage {
///     type Input = String;
///     type Output = String;
///
///     async fn process(&self, input: String) -> Result<String, AppError> {
///         Ok(input)
///     }
///
///     fn stage_type(&self) -> &'static str {
///         "passthrough"
///     }
/// }
/// ```
///
/// # Cross-References
///
/// - `SttProvider` / `CleanupStyle`: Super-trait relationship introduced in Story 1A.6.
/// - [`PipelineStageType`]: Compile-time registry enum in this module.
/// - Executor dispatch: Epic 1B Story 1B.5 (sequential dispatch + plugin-registry lookup).
/// - FR6: Compile-time Stage-Registry-Set via Cargo features + `PipelineStageType` variants.
/// - `architecture.md:234–238`: Trait-design table (Phase-0 shape + object-safety mandate).
///
/// # Phase-1 Stability
///
/// This signature is **Phase-1-locked**. Any change to associated types or method signatures
/// constitutes a breaking change and MUST trigger a Breaking-Change-Review before merging.
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Input type consumed by this stage. Must be `Send` for cross-thread pipeline dispatch.
    type Input: Send;
    /// Output type produced by this stage. Must be `Send` for cross-thread pipeline dispatch.
    type Output: Send;

    /// Execute this stage, consuming `input` and producing `Output` or an `AppError`.
    async fn process(&self, input: Self::Input) -> Result<Self::Output, AppError>;

    /// Discriminator string matching the `#[serde(tag = "type")]` wire-name in
    /// [`PipelineStageType`]. Used by the Executor (1B.5) for runtime dispatch validation.
    fn stage_type(&self) -> &'static str;
}

/// Compile-time registry of allowed pipeline stage types (FR6).
///
/// # Innovation-A Two-Layer Model
///
/// **Compile-Time layer** (this enum): The set of *allowed* stage types is fixed at
/// compile-time by Cargo feature flags. Each `stage-*` feature enables one or more
/// enum variants. A production binary built without `stage-stt` cannot load an STT stage
/// at runtime — the serde deserializer rejects the unknown tag before the Executor
/// ever runs.
///
/// **Boot-Time layer** (Epic 1B Story 1B.2 + 1B.5): The pipeline manifest TOML is parsed
/// at application start via serde's `#[serde(tag = "type")]` dispatch against this enum.
/// An unknown `"type"` value fails immediately with `AppError::kind::PipelineValidation`.
/// `warn!+skip` is forbidden (see `feedback_manifest_compile_contract`). Story 1B.5
/// then resolves each parsed variant against the plugin registry at runtime.
///
/// # Variants
///
/// Additional variants (`stt`, `cleanup`) are added additively in Story 1B.1.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PipelineStageType {
    /// No-op identity stage for testing and pipeline scaffolding. Wire-name: `"passthrough"`.
    #[cfg(feature = "stage-passthrough")]
    Passthrough,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct IdentityPassthroughStage;

    #[async_trait]
    impl PipelineStage for IdentityPassthroughStage {
        type Input = String;
        type Output = String;

        async fn process(&self, input: String) -> Result<String, AppError> {
            Ok(input)
        }

        fn stage_type(&self) -> &'static str {
            "passthrough"
        }
    }

    #[tokio::test]
    async fn identity_passthrough_stage_works() {
        let stage = IdentityPassthroughStage;
        assert_eq!(stage.process("hello".to_string()).await.unwrap(), "hello");
    }

    #[test]
    fn pipeline_stage_type_passthrough_serde_roundtrip() {
        let t = PipelineStageType::Passthrough;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"type":"passthrough"}"#);
        let back: PipelineStageType = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, PipelineStageType::Passthrough));
    }
}
