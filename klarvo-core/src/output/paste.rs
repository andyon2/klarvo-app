//! Paste-trigger abstraction for the dictation pipeline output step.
//!
//! # Second-Consumer Rationale
//!
//! Primary consumer: `shells/windows/src-tauri/src/paste.rs::WinSendInputPasteBackend`
//! (Story 3.5) — Win32 `SendInput` key-injection.
//!
//! Phase-3 consumer: `shells/android/.../AccessibilityPasteBackend` —
//! Android AccessibilityService paste-action.
//!
//! Two concrete platform implementations justify introducing this abstraction per
//! `feedback_premature_abstraction_guard` (second-consumer requirement).

use async_trait::async_trait;

use crate::error::AppError;

/// Triggers a paste action (Ctrl+V / platform paste-injection) into the focused window.
///
/// Step 6 (OutputTarget::deliver): sets clipboard content.
/// Step 7 (PasteBackend::paste): triggers Ctrl+V injection into focused window.
#[async_trait]
pub trait PasteBackend: Send + Sync {
    /// Trigger the platform paste action.
    ///
    /// # Error Variants
    ///
    /// Implementations are expected to return:
    /// - `AppErrorKind::Io` — Win32 SendInput failure or OS clipboard-access failure.
    /// - `AppErrorKind::PermissionDenied` — Android AccessibilityService denied or
    ///   not enabled (Phase-3).
    /// - `AppErrorKind::Configuration` — no target window focused or paste-target
    ///   not resolvable.
    ///
    /// i18n-key prefix: `error.paste.*` (per `docs/shell-error-mapping.md` Evolution-Policy).
    /// Concrete keys are registered by implementation stories (Story 3.5 for Windows,
    /// Phase-3 for Android) — not in this trait-definition story.
    async fn paste(&self) -> Result<(), AppError>;
}
