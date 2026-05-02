//! Debug-Export-Zip-Stub. Phase-2-Surface für Settings-UI (Epic 9).
//!
//! Spec: architecture.md §9 Observability + prd.md FR40.
//!
//! NFR5: ein zukünftiger Real-Impl darf KEINE Audio/Text-Daten exportieren.
//! Nur: Rolling-File-Log + redacted Config + Sys-Info.
//!
//! **Phase-2 Real-Impl Hinweise** (für Epic 9):
//! - Path-Traversal-Validation: `out_path` MUSS canonicalisiert + auf `..`-Segmente
//!   geprüft + Symlink-resolved werden, bevor geschrieben wird (Settings-UI-Input
//!   ist user-controlled).
//! - Single-Flight-Guard: Concurrent-Calls (UI-Doppelklick, Hotkey-Spam) müssen via
//!   `Mutex`/`AtomicBool` serialisiert werden, sonst korrupte Zip-Files.
//! - Caller MUSS `user_message` via i18n-Resolver auflösen, bevor er sie der UI gibt.

use std::path::Path;

use crate::error::{AppError, AppErrorKind};

/// Erzeugt ein Debug-Export-Zip am gegebenen Pfad.
///
/// Phase-1-Stub: returnt fail-soft `AppError` mit i18n-Key
/// `error.telemetry.export.unimplemented`. Real-Impl folgt in Epic 9
/// (Settings-UI-Trigger).
pub fn export_debug_zip(_out_path: &Path) -> Result<(), AppError> {
    Err(AppError {
        // TODO(phase-2): introduce dedicated `AppErrorKind::ExportFailed` /
        // `Unimplemented` variant; `Configuration` is semantically wrong and could
        // route this through config-error UI flows.
        kind: AppErrorKind::Configuration,
        message: "telemetry::export::export_debug_zip is a Phase-1 stub".into(),
        user_message: Some("error.telemetry.export.unimplemented".into()),
        retryable: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_debug_zip_returns_unimplemented_error() {
        let result = export_debug_zip(std::path::Path::new("/tmp/dummy.zip"));
        let err = result.expect_err("stub must return Err");
        assert_eq!(
            err.user_message.as_deref(),
            Some("error.telemetry.export.unimplemented")
        );
        assert!(!err.retryable);
    }
}
