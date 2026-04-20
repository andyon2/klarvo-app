//! Pipeline stage type definitions and the [`PipelineStageType`] compile-time registry.
//!
//! [`PipelineStageType`] implements Innovation-A's compile-time stage-registry: allowed
//! stage types are closed at compile-time via Cargo feature flags. A binary built without
//! a feature cannot load that stage type at runtime — serde rejects the unknown tag.
//!
//! ## Exhaustive-Match Pattern (No Wildcard)
//!
//! Consumers matching on [`PipelineStageType`] MUST mirror the `#[cfg(feature = "stage-*")]`
//! gates on their match arms. No `_` wildcard arms — see [`PipelineStageType`] doc for details.
//!
//! ```
//! use klarvo_core::pipeline::PipelineStageType;
//!
//! fn describe(t: PipelineStageType) -> &'static str {
//!     match t {
//!         #[cfg(feature = "stage-passthrough")]
//!         PipelineStageType::Passthrough => "passthrough",
//!         #[cfg(feature = "stage-stt")]
//!         PipelineStageType::Stt { .. } => "stt",
//!         #[cfg(feature = "stage-cleanup")]
//!         PipelineStageType::Cleanup { .. } => "cleanup",
//!     }
//! }
//! ```

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

/// Compile-time registry of allowed pipeline stage types (FR6). Innovation-A, Compile-Time layer.
///
/// # Three Roles
///
/// 1. **Compile-Time-Registry**: The set of allowed stage types is closed at compile-time
///    via Cargo feature flags (`stage-passthrough`, `stage-stt`, `stage-cleanup`). Disabled
///    features remove variants — a binary without `stage-stt` cannot deserialize an STT stage.
/// 2. **Boot-Time-Parse**: Pipeline manifest TOML is parsed at startup via serde's
///    `#[serde(tag = "type")]` dispatch against this enum. Unknown types fail immediately with
///    `AppError::kind::PipelineValidation` — no `warn!+skip` (Story 1B.2).
/// 3. **Runtime-Dispatch**: The Executor (Story 1B.5) matches this enum to look up plugin impls
///    via `plugin_id` in the `PluginRegistry`.
///
/// # Innovation-A Failure Modes
///
/// Compile-time enforcement happens at the registry-set level: adding a new stage-type requires
/// an enum-variant addition plus feature declaration in `klarvo-core`. Unknown stage-types in a
/// user-authored manifest fail at boot-time deserialization (1B.2), not at compile-time —
/// these are distinct failure modes.
///
/// # No-Wildcard Invariant
///
/// Adding a variant here is a breaking change by design; downstream `match` sites MUST be
/// updated. No `_` wildcard arms in core match-sites. If you add a variant without updating
/// all exhaustive match sites, the compiler reports an error — that is intentional.
///
/// Consumers matching on `PipelineStageType` under non-default feature combinations must
/// mirror the `#[cfg(feature = …)]` gating in their match arms. See module-level doc for
/// an inline-runnable example.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PipelineStageType {
    /// No-op identity stage for testing and pipeline scaffolding. Wire-name: `"passthrough"`.
    ///
    /// Passthrough is the only plugin-free variant in Phase 1; all other stage-types carry
    /// `plugin_id` for plugin-registry resolution at boot-time (1B.5).
    ///
    /// Trait family: no corresponding provider trait — pure passthrough.
    /// Cross-references: Story 1A.5 (skeleton), Story 1B.5 (Executor dispatch).
    #[cfg(feature = "stage-passthrough")]
    Passthrough,

    /// STT stage: audio-to-text transcription via `plugin_id`. Wire-name: `"stt"`.
    ///
    /// `plugin_id` must match a registered [`crate::traits::SttProvider`] in the
    /// `PluginRegistry` at boot-time. An unregistered `plugin_id` causes a hard-fail
    /// `AppError::kind::PipelineValidation` (1B.5).
    ///
    /// Trait family: [`crate::traits::SttProvider`] (Story 1A.6).
    /// Cross-references: Story 1B.2 (manifest parse), Story 1B.5 (executor dispatch).
    #[cfg(feature = "stage-stt")]
    Stt { plugin_id: String },

    /// Cleanup stage: text transformation via `plugin_id`. Wire-name: `"cleanup"`.
    ///
    /// `plugin_id` must match a registered [`crate::traits::CleanupStyle`] in the
    /// `PluginRegistry` at boot-time. Phase-1 reference: `"verbatim"` (identity passthrough,
    /// `klarvo-plugin-verbatim`). An unregistered `plugin_id` causes a hard-fail
    /// `AppError::kind::PipelineValidation` (1B.5).
    ///
    /// Trait family: [`crate::traits::CleanupStyle`] (Story 1A.6).
    /// Cross-references: Story 1B.2 (manifest parse), Story 1B.3 (verbatim reference impl),
    /// Story 1B.5 (executor dispatch).
    #[cfg(feature = "stage-cleanup")]
    Cleanup { plugin_id: String },
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

    #[cfg(feature = "stage-passthrough")]
    #[test]
    fn pipeline_stage_type_passthrough_serde_roundtrip() {
        let t = PipelineStageType::Passthrough;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"type":"passthrough"}"#);
        let back: PipelineStageType = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, PipelineStageType::Passthrough));
    }

    #[cfg(feature = "stage-stt")]
    #[test]
    fn pipeline_stage_type_stt_json_roundtrip() {
        let t = PipelineStageType::Stt { plugin_id: "groq".into() };
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"type":"stt","plugin_id":"groq"}"#);
        let back: PipelineStageType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[cfg(feature = "stage-cleanup")]
    #[test]
    fn pipeline_stage_type_cleanup_json_roundtrip() {
        let t = PipelineStageType::Cleanup { plugin_id: "verbatim".into() };
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"type":"cleanup","plugin_id":"verbatim"}"#);
        let back: PipelineStageType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    // Exhaustive-match guard: adding a variant without updating this function is a compile error.
    fn assert_exhaustive_match(t: &PipelineStageType) -> &'static str {
        match t {
            #[cfg(feature = "stage-passthrough")]
            PipelineStageType::Passthrough => "passthrough",
            #[cfg(feature = "stage-stt")]
            PipelineStageType::Stt { .. } => "stt",
            #[cfg(feature = "stage-cleanup")]
            PipelineStageType::Cleanup { .. } => "cleanup",
        }
    }

    #[test]
    fn pipeline_stage_type_match_is_exhaustive_no_wildcard() {
        #[cfg(feature = "stage-passthrough")]
        assert_eq!(assert_exhaustive_match(&PipelineStageType::Passthrough), "passthrough");
        #[cfg(feature = "stage-stt")]
        assert_eq!(assert_exhaustive_match(&PipelineStageType::Stt { plugin_id: "groq".into() }), "stt");
        #[cfg(feature = "stage-cleanup")]
        assert_eq!(assert_exhaustive_match(&PipelineStageType::Cleanup { plugin_id: "verbatim".into() }), "cleanup");
    }
}
