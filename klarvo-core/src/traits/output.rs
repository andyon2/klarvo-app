use async_trait::async_trait;

use crate::error::AppError;

/// `OutputTarget` is the Terminal-Sink-Trait for the dictation pipeline. It is architecturally
/// distinct from the 4 Phase-1 Data-Flow-Stability-Traits (`PipelineStage`, `SttProvider`,
/// `CleanupStyle`, `VadProvider`) — the Ring remains '4'.
/// `OutputTarget` is a Plugin-Contract (multiple concurrent implementations, Registry-dispatched)
/// not an Infrastructure-Trait (cf. `KeyStore`). Trait-Signature is locked in Phase 1; new
/// targets extend via new plugin-crates without Trait-change. Phase-1 Reference-Impl:
/// `klarvo-plugin-clipboard`. Phase-2 extension: `klarvo-plugin-keystroke` (architecture.md:1036).
#[async_trait]
pub trait OutputTarget: Send + Sync + 'static {
    /// Deliver the final cleanup-output text to the configured target (clipboard,
    /// keystroke-injection, file, network endpoint, etc.).
    ///
    /// Returns `Ok(())` on success. On failure returns `AppError` with `user_message` set to an
    /// i18n-key (Core emits keys, Shell resolves per i18n-contract,
    /// `memory/project_i18n_core_contract`). Production implementations MUST NOT log or persist
    /// `text` beyond the immediate delivery operation (PII-Log-Discipline per NFR5). Test fixtures
    /// (e.g., `InMemoryOutputTarget`) persist for assertion access and are a narrow, documented
    /// exception.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use klarvo_core::error::{AppError, AppErrorKind};
    /// use klarvo_core::output::keys;
    ///
    /// let text: String = /* extracted from run_capture_session result */;
    /// registry.output("clipboard")
    ///     .ok_or_else(|| AppError {
    ///         kind: AppErrorKind::Configuration,
    ///         message: "output target not found: clipboard".to_string(),
    ///         user_message: Some(keys::TARGET_NOT_FOUND.to_string()),
    ///         retryable: false,
    ///     })?
    ///     .deliver(&text)
    ///     .await?;
    /// ```
    async fn deliver(&self, text: &str) -> Result<(), AppError>;
}
