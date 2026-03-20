//! Tauri commands for Whisper model management (offline STT).
//!
//! These commands are Windows-only because local whisper.cpp is only built
//! for the Windows target. The frontend guards the UI accordingly.
//!
//! ## Commands
//!
//! - [`get_whisper_models`] -- list all catalogue models + download status.
//! - [`download_whisper_model`] -- start a background download, emits progress events.
//! - [`delete_whisper_model`] -- remove a downloaded model file.
//!
//! ## Events emitted during download
//!
//! | Event | Payload |
//! |-------|---------|
//! | `dikta://model-download-progress` | `{ modelId, bytesReceived, totalBytes }` |
//! | `dikta://model-download-complete` | `{ modelId }` |
//! | `dikta://model-download-error`    | `{ modelId, error }` |

#[cfg(target_os = "windows")]
pub mod windows {
    use serde::Serialize;
    use tauri::{AppHandle, Emitter, Manager, State};

    use crate::license::{is_feature_allowed, LicensedFeature};
    use crate::stt::model_manager::{
        self, ModelManagerError, WhisperModelWithStatus,
    };
    use crate::AppState;

    // -----------------------------------------------------------------------
    // Event names
    // -----------------------------------------------------------------------

    const EVENT_PROGRESS: &str = "dikta://model-download-progress";
    const EVENT_COMPLETE: &str = "dikta://model-download-complete";
    const EVENT_ERROR: &str = "dikta://model-download-error";

    // -----------------------------------------------------------------------
    // Event payloads
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ProgressPayload {
        model_id: String,
        bytes_received: u64,
        total_bytes: u64,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CompletePayload {
        model_id: String,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ErrorPayload {
        model_id: String,
        error: String,
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    /// Returns all Whisper models from the catalogue together with their
    /// current download status (file present on disk or not).
    ///
    /// The `app_data_dir` is resolved from the Tauri `AppHandle` so this
    /// command does not depend on `AppState` for the path.
    #[tauri::command]
    pub fn get_whisper_models(
        handle: AppHandle,
    ) -> Result<Vec<WhisperModelWithStatus>, String> {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

        Ok(model_manager::list_models_with_status(&app_data_dir))
    }

    /// Starts a background download for the given model.
    ///
    /// Returns immediately (non-blocking). Download progress is reported via
    /// `dikta://model-download-progress` events. On completion
    /// `dikta://model-download-complete` is emitted; on failure
    /// `dikta://model-download-error` is emitted.
    ///
    /// `app_data_dir` comes from the `AppHandle`, not `AppState`, to avoid
    /// holding the state lock across async boundaries.
    #[tauri::command]
    pub fn download_whisper_model(
        handle: AppHandle,
        model_id: String,
        _state: State<'_, AppState>,
    ) -> Result<(), String> {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

        // License gate: medium and large-v3 require a paid license.
        // Only small is free (no gate).
        if model_id == "medium" || model_id == "large-v3" {
            let license_status = handle
                .state::<AppState>()
                .license_status
                .lock()
                .map_err(|_| "License state lock poisoned".to_string())?
                .clone();
            if !is_feature_allowed(&license_status, LicensedFeature::OfflineMode) {
                return Err("feature_requires_license:OfflineMode".to_string());
            }
        }

        // Validate model_id eagerly to surface errors before spawning.
        model_manager::list_available_models()
            .iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| format!("Unknown model id: {model_id}"))?;

        let handle_clone = handle.clone();
        let model_id_clone = model_id.clone();

        tauri::async_runtime::spawn(async move {
            let mid = model_id_clone.clone();
            let h = handle_clone.clone();

            let result = model_manager::download_model(
                &model_id_clone,
                &app_data_dir,
                move |bytes_received, total_bytes| {
                    let _ = h.emit(
                        EVENT_PROGRESS,
                        ProgressPayload {
                            model_id: mid.clone(),
                            bytes_received,
                            total_bytes,
                        },
                    );
                },
            )
            .await;

            match result {
                Ok(()) => {
                    log::info!("[whisper_cmd] Download complete: {}", model_id_clone);
                    let _ = handle_clone.emit(
                        EVENT_COMPLETE,
                        CompletePayload {
                            model_id: model_id_clone,
                        },
                    );
                }
                Err(e) => {
                    log::warn!("[whisper_cmd] Download failed for {}: {e}", model_id_clone);
                    let _ = handle_clone.emit(
                        EVENT_ERROR,
                        ErrorPayload {
                            model_id: model_id_clone,
                            error: e.to_string(),
                        },
                    );
                }
            }
        });

        Ok(())
    }

    /// Deletes a previously downloaded model file.
    ///
    /// Idempotent: succeeds even when the file does not exist.
    #[tauri::command]
    pub fn delete_whisper_model(
        handle: AppHandle,
        model_id: String,
    ) -> Result<(), String> {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

        model_manager::delete_model(&model_id, &app_data_dir)
            .map_err(|e| match e {
                ModelManagerError::UnknownModel(id) => format!("Unknown model id: {id}"),
                ModelManagerError::Io(io_err) => format!("Failed to delete model: {io_err}"),
                other => format!("Delete failed: {other}"),
            })
    }
}
