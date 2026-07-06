//! Tauri commands for Whisper model management and offline transcription.
//!
//! ## Platform modules
//!
//! - [`windows`] -- model management commands for Windows (get/download/delete).
//! - [`android`] -- model management + `transcribe_local` command for Android.
//!
//! On Android, `KlarvoApi.kt` calls `transcribe_local` directly instead of
//! making Groq HTTP requests, enabling fully offline dictation.
//!
//! ## Commands (both platforms)
//!
//! - `get_whisper_models` -- list all catalogue models + download status.
//! - `download_whisper_model` -- start a background download, emits progress events.
//! - `delete_whisper_model` -- remove a downloaded model file.
//!
//! ## Commands (Android only)
//!
//! - `transcribe_local` -- transcribe Base64-encoded WAV bytes with the local model.
//!
//! ## Events emitted during download
//!
//! | Event | Payload |
//! |-------|---------|
//! | `klarvo://model-download-progress` | `{ modelId, bytesReceived, totalBytes }` |
//! | `klarvo://model-download-complete` | `{ modelId }` |
//! | `klarvo://model-download-error`    | `{ modelId, error }` |

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

    const EVENT_PROGRESS: &str = "klarvo://model-download-progress";
    const EVENT_COMPLETE: &str = "klarvo://model-download-complete";
    const EVENT_ERROR: &str = "klarvo://model-download-error";

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
    /// `klarvo://model-download-progress` events. On completion
    /// `klarvo://model-download-complete` is emitted; on failure
    /// `klarvo://model-download-error` is emitted.
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

// ---------------------------------------------------------------------------
// Android: model management + transcribe_local
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
pub mod android {
    use base64::Engine as _;
    use serde::Serialize;
    use tauri::{AppHandle, Emitter, Manager, State};

    use crate::license::{is_feature_allowed, LicensedFeature};
    use crate::stt::model_manager::{self, ModelManagerError, WhisperModelWithStatus};
    use crate::stt::{LocalWhisperProvider, SttProvider};
    use crate::AppState;

    // -----------------------------------------------------------------------
    // Event names (same as Windows to keep frontend symmetric)
    // -----------------------------------------------------------------------

    const EVENT_PROGRESS: &str = "klarvo://model-download-progress";
    const EVENT_COMPLETE: &str = "klarvo://model-download-complete";
    const EVENT_ERROR: &str = "klarvo://model-download-error";

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
    // Model management commands (identical semantics to Windows module)
    // -----------------------------------------------------------------------

    /// Returns all Whisper models from the catalogue together with their
    /// current download status (file present in `app_data_dir/models/` or not).
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
    /// `klarvo://model-download-progress` events. On completion
    /// `klarvo://model-download-complete` is emitted; on failure
    /// `klarvo://model-download-error` is emitted.
    ///
    /// License gate: `medium` and `large-v3` require an OfflineMode license.
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

        // Validate model_id eagerly before spawning.
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
                    log::info!("[whisper_cmd/android] Download complete: {}", model_id_clone);
                    let _ = handle_clone.emit(
                        EVENT_COMPLETE,
                        CompletePayload {
                            model_id: model_id_clone,
                        },
                    );
                }
                Err(e) => {
                    log::warn!(
                        "[whisper_cmd/android] Download failed for {}: {e}",
                        model_id_clone
                    );
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

    // -----------------------------------------------------------------------
    // transcribe_local -- Android-only offline STT command
    // -----------------------------------------------------------------------

    /// Transcribes WAV audio using the local whisper.cpp model.
    ///
    /// ## Why this command exists
    ///
    /// On Android, `KlarvoApi.kt` makes direct HTTP calls to Groq for STT.
    /// This command is the offline replacement: instead of Groq HTTP, Kotlin
    /// calls `transcribe_local` over the Tauri bridge, passing the recorded
    /// audio as a Base64-encoded WAV string.
    ///
    /// ## Parameters
    ///
    /// - `wav_base64`: Base64-encoded WAV bytes (Standard encoding, no line breaks).
    ///   The WAV must be 16 kHz mono as produced by the Android audio recorder.
    /// - `language`: ISO-639-1 code (`"de"`, `"en"`) or `None` for auto-detect.
    ///
    /// ## Returns
    ///
    /// The transcribed text string, trimmed of leading/trailing whitespace.
    ///
    /// ## Errors
    ///
    /// - `"base64_decode_error: ..."` -- invalid Base64 input.
    /// - `"model_not_found: ..."` -- model file missing; user must download first.
    /// - `"transcription_error: ..."` -- whisper.cpp inference failure.
    #[tauri::command]
    pub async fn transcribe_local(
        handle: AppHandle,
        wav_base64: String,
        language: Option<String>,
        state: State<'_, AppState>,
    ) -> Result<String, String> {
        // 1. Decode Base64 -> WAV bytes.
        let wav_bytes = base64::engine::general_purpose::STANDARD
            .decode(&wav_base64)
            .map_err(|e| format!("base64_decode_error: {e}"))?;

        if wav_bytes.is_empty() {
            return Err("base64_decode_error: decoded WAV is empty".to_string());
        }

        // 2. Resolve model path: {app_data_dir}/models/ggml-{model_id}.bin
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

        let model_id = {
            let cfg = state
                .config
                .lock()
                .map_err(|_| "Config lock poisoned".to_string())?;
            cfg.local_whisper_model.clone()
        };

        let model_path = app_data_dir
            .join("models")
            .join(format!("ggml-{model_id}.bin"));

        if !model_path.exists() {
            return Err(format!(
                "model_not_found: {} — download the model first",
                model_path.display()
            ));
        }

        log::info!(
            "[transcribe_local] model={}, wav_bytes={}, lang={:?}",
            model_path.display(),
            wav_bytes.len(),
            language
        );

        // 3. Create a fresh LocalWhisperProvider for this request.
        //    The model context is cached inside the provider; since we create a
        //    new instance per call here we pay the load cost each time. A future
        //    optimisation is to cache the provider in AppState on Android.
        let provider = LocalWhisperProvider::new(model_path.to_string_lossy().into_owned());

        let lang = language.as_deref().unwrap_or("").to_owned();

        // 4. Run transcription (blocks inside spawn_blocking inside the provider).
        let result = provider
            .transcribe(&wav_bytes, &lang, None)
            .await
            .map_err(|e| format!("transcription_error: {e}"))?;

        log::info!("[transcribe_local] result: {:?}", result);

        Ok(result)
    }
}
