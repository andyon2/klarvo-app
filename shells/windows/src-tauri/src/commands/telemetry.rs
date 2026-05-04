use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::State;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::telemetry::export::export_debug_zip;

pub struct ExportState {
    pub log_dir: PathBuf,
    pub in_progress: Arc<AtomicBool>,
}

/// RAII-Drop-Guard für das Single-Flight-Flag. Garantiert reset auch bei Panic
/// im Export-Body (vgl. Story-Spec L358-362 + Memory `feedback_test_raii_cleanup_pattern`).
struct InProgressGuard<'a>(&'a AtomicBool);
impl<'a> Drop for InProgressGuard<'a> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn export_debug_zip_cmd(
    state: State<'_, ExportState>,
) -> Result<String, AppError> {
    if state
        .in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(AppError {
            kind: AppErrorKind::ExportFailed,
            message: "export already in progress".into(),
            user_message: Some("error.telemetry.export.in_progress".into()),
            retryable: false,
        });
    }
    let _guard = InProgressGuard(&state.in_progress);

    let log_dir = state.log_dir.clone();
    let out_path = resolve_export_path();

    // Sync-Export läuft auf Blocking-Pool; vermeidet Stalls auf der Tauri-IPC-Runtime.
    let zip_path = out_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        export_debug_zip(&log_dir, &zip_path, env!("CARGO_PKG_VERSION"))
    })
    .await
    .map_err(|e| AppError {
        kind: AppErrorKind::ExportFailed,
        message: format!("join error: {e}"),
        user_message: Some("error.telemetry.export.failed".into()),
        retryable: false,
    })?;

    result?;
    Ok(out_path.to_string_lossy().into_owned())
}

fn resolve_export_path() -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!("klarvo-debug-{ts}.zip");
    std::env::var("USERPROFILE")
        .ok()
        .map(|h| PathBuf::from(h).join("Downloads").join(&name))
        .filter(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .unwrap_or_else(|| std::env::temp_dir().join(name))
}
