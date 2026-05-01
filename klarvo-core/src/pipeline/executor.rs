//! Pipeline executor — Boot+Runtime Dispatch layer of Innovation-A's FR6 three-layer contract.
//!
//! # Pipeline-Executor Scope
//!
//! Pipeline-Executor is the Runtime-Layer of Innovation-A's three-layer FR6 contract
//! (Compile-Time Registry-Set in 1B.1, Boot-Time Parse in 1B.2, Boot+Runtime Dispatch here).
//! All three layers share the hard-fail-no-warn-skip invariant per
//! `memory/feedback_manifest_compile_contract`. The Executor closes Epic 1B
//! (Pipeline-Composition Runtime).
//!
//! # Boot-Check Ordering
//!
//! Boot-time checks run in order: (1) Type-Chaining-Compat-Check, (2)
//! Plugin-Registry-Lookup-Check. Rationale: Type-mismatch is a manifest-authoring-error
//! independent of registry-state; checking it first surfaces manifest-bugs even when plugins
//! are missing from the registry.
//!
//! # Per-Pipeline Correlation (Epic 6 scope)
//!
//! Per-Pipeline-Correlation (correlation-IDs, tracing-spans, structured log-events) is Epic 6
//! Observability scope — not 1B.5. The Executor emits no `tracing` events in Phase 1;
//! instrumentation points are reserved for Epic 6's NFR5/NFR6-respecting Observability layer.
//!
//! # E2E Tests and Groq Wire-Up
//!
//! E2E-Pipeline-Tests intentionally use Verbatim-based pipelines only (passthrough + verbatim).
//! Groq-based pipelines require KeyStore-backed API-key-delivery (Epic 1C) and are first
//! exercised end-to-end in Epic 2 (End-to-End Dictation Pipeline headless integration tests).
//!
//! # CleanupInput Construction (1A.6 Amendment)
//!
//! `CleanupInput::from_raw(raw: String) -> Self` with `CleanupContext::default()` was added as
//! Phase-1-amendment to 1A.6 in 1B.5 to enable dispatch-arm-internal Text→CleanupInput wrapping
//! without forcing CleanupInput to be a distinct StageData-variant.
//! Pipeline-Config-passed-CleanupContext (i18n-Output-Language-Axis per
//! `memory/project_i18n_three_axes`) is Epic 4 / Phase-2+ scope.
//!
//! # Dispatch-Arm Invariance
//!
//! Variant-extract operations in dispatch-arms
//! (`let StageData::Audio(audio) = data else { unreachable!(...); };`) are guaranteed by the
//! boot-time Type-Chaining-Check. Removing or weakening that check breaks this invariant —
//! any future refactor that touches the boot-check must preserve the dispatch-arm-type-safety
//! contract.

use crate::error::{AppError, AppErrorKind};
use crate::manifest::PipelineManifest;
use crate::pipeline::stage::PipelineStageType;
use crate::pipeline::stage_data::StageData;
use crate::registry::PluginRegistry;
use crate::traits::CleanupInput;

/// i18n keys emitted by the pipeline executor.
///
/// All keys validated via `debug_assert!(klarvo_core::i18n::is_key(KEY))` at emission sites.
/// Keys are statically referenzierbar from Shell-Translation-Tables (Epic 4 forward-reference).
pub mod keys {
    /// A required plugin was not found in the [`crate::registry::PluginRegistry`] at boot-time.
    pub const PLUGIN_NOT_FOUND: &str = "error.pipeline.plugin_not_found";
    /// Two adjacent pipeline stages have incompatible input/output types (Type-Chaining-Mismatch).
    pub const STAGE_TYPE_MISMATCH: &str = "error.pipeline.stage_type_mismatch";
}

/// Execute a pipeline defined by `manifest` against `registry`, threading `input` through all stages.
///
/// # Boot-Time Checks (before first stage dispatch)
///
/// 1. **Type-Chaining-Compat-Check** (first): verifies that each stage's required input type
///    matches the output type of the preceding stage (or `input.type_name()` for stage 0).
///    Fails with `AppError::kind::PipelineValidation` +
///    [`keys::STAGE_TYPE_MISMATCH`] on mismatch.
/// 2. **Plugin-Registry-Lookup-Check** (second): verifies every `plugin_id` in the manifest is
///    registered. Fails with `AppError::kind::PipelineValidation` +
///    [`keys::PLUGIN_NOT_FOUND`] on missing entry.
///
/// # Runtime Dispatch
///
/// Stages are dispatched sequentially via exhaustive match over [`PipelineStageType`] (no `_`
/// wildcard). Variant-extract operations are guaranteed safe by the boot-time Type-Chaining-Check.
pub async fn run_pipeline(
    manifest: &PipelineManifest,
    registry: &PluginRegistry,
    input: StageData,
) -> Result<StageData, AppError> {
    // Boot-Time-Check-1: Type-Chaining-Compat-Check.
    // Runs independently of registry-state so manifest-authoring-errors surface even when
    // plugins are missing.
    let mut current_type = input.type_name();
    for (idx, stage) in manifest.pipeline.stages.iter().enumerate() {
        match stage {
            #[cfg(feature = "stage-passthrough")]
            PipelineStageType::Passthrough => {
                // identity — current_type propagates unchanged over both variants
            }
            #[cfg(feature = "stage-stt")]
            PipelineStageType::Stt { .. } => {
                if current_type != "audio" {
                    debug_assert!(crate::i18n::is_key(keys::STAGE_TYPE_MISMATCH));
                    return Err(AppError {
                        kind: AppErrorKind::PipelineValidation,
                        message: format!(
                            "stage[{idx}] (stt): expected input type 'audio', got '{current_type}'"
                        ),
                        user_message: Some(keys::STAGE_TYPE_MISMATCH.into()),
                        retryable: false,
                    });
                }
                current_type = "text";
            }
            #[cfg(feature = "stage-cleanup")]
            PipelineStageType::Cleanup { .. } => {
                if current_type != "text" {
                    debug_assert!(crate::i18n::is_key(keys::STAGE_TYPE_MISMATCH));
                    return Err(AppError {
                        kind: AppErrorKind::PipelineValidation,
                        message: format!(
                            "stage[{idx}] (cleanup): expected input type 'text', got '{current_type}'"
                        ),
                        user_message: Some(keys::STAGE_TYPE_MISMATCH.into()),
                        retryable: false,
                    });
                }
                // current_type stays "text"
            }
        }
    }

    // Boot-Time-Check-2: Plugin-Registry-Lookup-Hard-Fail.
    // Runs after Type-Chaining-Check per boot-check ordering invariant.
    for (idx, stage) in manifest.pipeline.stages.iter().enumerate() {
        match stage {
            #[cfg(feature = "stage-passthrough")]
            PipelineStageType::Passthrough => {
                // Passthrough is Executor-built-in — no registry slot required
            }
            #[cfg(feature = "stage-stt")]
            PipelineStageType::Stt { plugin_id } => {
                if registry.stt(plugin_id).is_none() {
                    debug_assert!(crate::i18n::is_key(keys::PLUGIN_NOT_FOUND));
                    return Err(AppError {
                        kind: AppErrorKind::PipelineValidation,
                        message: format!(
                            "stage[{idx}] (stt): plugin '{plugin_id}' not registered"
                        ),
                        user_message: Some(keys::PLUGIN_NOT_FOUND.into()),
                        retryable: false,
                    });
                }
            }
            #[cfg(feature = "stage-cleanup")]
            PipelineStageType::Cleanup { plugin_id } => {
                if registry.cleanup(plugin_id).is_none() {
                    debug_assert!(crate::i18n::is_key(keys::PLUGIN_NOT_FOUND));
                    return Err(AppError {
                        kind: AppErrorKind::PipelineValidation,
                        message: format!(
                            "stage[{idx}] (cleanup): plugin '{plugin_id}' not registered"
                        ),
                        user_message: Some(keys::PLUGIN_NOT_FOUND.into()),
                        retryable: false,
                    });
                }
            }
        }
    }

    // Runtime-Dispatch: both boot-checks passed — all type-chains and plugin lookups are valid.
    let mut data = input;
    for stage in &manifest.pipeline.stages {
        data = match stage {
            #[cfg(feature = "stage-passthrough")]
            PipelineStageType::Passthrough => data,

            #[cfg(feature = "stage-stt")]
            PipelineStageType::Stt { plugin_id } => {
                // Boot-time Type-Chaining-Check guarantees Audio-variant here.
                let StageData::Audio(audio) = data else {
                    unreachable!("boot-time Type-Chaining-Check guarantees Audio-variant here");
                };
                let plugin = registry.stt(plugin_id)
                    .unwrap_or_else(|| unreachable!("boot-check guaranteed registered"));
                let text = plugin.transcribe(audio).await?;
                StageData::Text(text)
            }

            #[cfg(feature = "stage-cleanup")]
            PipelineStageType::Cleanup { plugin_id } => {
                // Boot-time Type-Chaining-Check guarantees Text-variant here.
                let StageData::Text(raw) = data else {
                    unreachable!("boot-time Type-Chaining-Check guarantees Text-variant here");
                };
                let plugin = registry.cleanup(plugin_id)
                    .unwrap_or_else(|| unreachable!("boot-check guaranteed registered"));
                let cleanup_input = CleanupInput::from_raw(raw);
                let text = plugin.apply(cleanup_input).await?;
                StageData::Text(text)
            }
        };
    }
    Ok(data)
}
