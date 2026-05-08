//! Recording control commands (Story 11.1).

use klarvo_shell_orchestrator::SessionOrchestrator;

/// Abort the current recording session (Pill-Bar abort button).
///
/// No-op if no recording is active. Returns immediately — the orchestrator's
/// async teardown (pipeline_task.abort, level_tap_task.abort) completes
/// concurrently; the Pill-Bar fades via `Event::RecordingAborted` on the
/// EventBus subscriber.
#[tauri::command]
#[specta::specta]
pub async fn cancel_recording(
    orch: tauri::State<'_, SessionOrchestrator>,
) -> Result<(), ()> {
    orch.cancel_recording().await;
    Ok(())
}
