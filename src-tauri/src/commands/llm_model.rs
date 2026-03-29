//! Tauri commands for local LLM model management (offline cleanup).
//!
//! ## Platform support
//!
//! - **Windows** (`windows` module): Downloads a single GGUF file for llama.cpp inference.
//! - **Android** (`android` module): Downloads the multi-file MNN model bundle
//!   (Qwen2.5-1.5B in MNN format) for `LocalLlmInference.kt`.
//!
//! Both modules expose the same three command names so the frontend can call
//! them identically on all platforms.
//!
//! ## Commands
//!
//! - [`get_llm_model_status`] -- check if the model is present on disk.
//! - [`download_llm_model`]   -- start a background download, emits progress events.
//! - [`delete_llm_model`]     -- remove the downloaded model file(s).
//!
//! ## Events emitted during download
//!
//! | Event | Payload |
//! |-------|---------|
//! | `klarvo://llm-model-download-progress` | `{ bytesReceived, totalBytes }` |
//! | `klarvo://llm-model-download-complete` | `{}` |
//! | `klarvo://llm-model-download-error`    | `{ error }` |

#[cfg(target_os = "windows")]
pub mod windows {
    use serde::{Deserialize, Serialize};
    use tauri::{AppHandle, Emitter, Manager};
    use tokio::io::AsyncWriteExt;

    // -----------------------------------------------------------------------
    // Model constants
    // -----------------------------------------------------------------------

    /// Model filename on disk and in the download URL.
    const MODEL_FILENAME: &str = "qwen2.5-1.5b-instruct-q4_k_m.gguf";

    /// Download URL for the Qwen2.5-1.5B-Instruct GGUF model.
    const MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf";

    /// Approximate size in bytes (~1.1 GB), used as a fallback when the server
    /// does not send a Content-Length header.
    const MODEL_APPROX_SIZE: u64 = 1_100_000_000;

    // -----------------------------------------------------------------------
    // Event names
    // -----------------------------------------------------------------------

    const EVENT_PROGRESS: &str = "klarvo://llm-model-download-progress";
    const EVENT_COMPLETE: &str = "klarvo://llm-model-download-complete";
    const EVENT_ERROR: &str = "klarvo://llm-model-download-error";

    // -----------------------------------------------------------------------
    // Event payloads
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ProgressPayload {
        bytes_received: u64,
        total_bytes: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CompletePayload {}

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ErrorPayload {
        error: String,
    }

    // -----------------------------------------------------------------------
    // Response types
    // -----------------------------------------------------------------------

    /// Status of the local LLM model file.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LlmModelStatus {
        /// Whether the model file exists on disk and is ready to use.
        pub downloaded: bool,
        /// File size in bytes. `None` when the file is not present.
        pub size_bytes: Option<u64>,
        /// Absolute path where the model is (or will be) stored.
        pub path: String,
    }

    // -----------------------------------------------------------------------
    // Path helper
    // -----------------------------------------------------------------------

    /// Returns the absolute path where the LLM model file is stored.
    ///
    /// Path: `{app_data_dir}/models/{MODEL_FILENAME}`
    fn model_file_path(handle: &AppHandle) -> Result<std::path::PathBuf, String> {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
        Ok(app_data_dir.join("models").join(MODEL_FILENAME))
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    /// Returns the current download status of the local LLM model.
    ///
    /// Checks whether the model file exists on disk and reports its size.
    #[tauri::command]
    pub fn get_llm_model_status(handle: AppHandle) -> Result<LlmModelStatus, String> {
        let path = model_file_path(&handle)?;

        let (downloaded, size_bytes) = if path.exists() {
            let size = std::fs::metadata(&path)
                .map(|m| m.len())
                .ok();
            (true, size)
        } else {
            (false, None)
        };

        Ok(LlmModelStatus {
            downloaded,
            size_bytes,
            path: path.display().to_string(),
        })
    }

    /// Starts a background download for the Qwen2.5-1.5B-Instruct GGUF model.
    ///
    /// Returns immediately (non-blocking). Progress is reported via
    /// `klarvo://llm-model-download-progress` events. On completion
    /// `klarvo://llm-model-download-complete` is emitted; on failure
    /// `klarvo://llm-model-download-error` is emitted.
    ///
    /// Uses an atomic write strategy: bytes are written to a `.part` file and
    /// renamed to the final path only on success. This prevents the
    /// `LocalLlmCleanup` provider from picking up a partial model file.
    #[tauri::command]
    pub fn download_llm_model(handle: AppHandle) -> Result<(), String> {
        let final_path = model_file_path(&handle)?;

        // Ensure the models directory exists before spawning.
        let models_dir = final_path
            .parent()
            .ok_or("Could not determine models directory")?;
        std::fs::create_dir_all(models_dir)
            .map_err(|e| format!("Failed to create models directory: {e}"))?;

        let part_path = final_path.with_extension("gguf.part");
        let handle_clone = handle.clone();

        tauri::async_runtime::spawn(async move {
            let result = run_download(&handle_clone, &final_path, &part_path).await;

            match result {
                Ok(()) => {
                    log::info!("[llm_model] Download complete: {}", MODEL_FILENAME);
                    let _ = handle_clone.emit(EVENT_COMPLETE, CompletePayload {});
                }
                Err(e) => {
                    log::warn!("[llm_model] Download failed: {e}");
                    // Best-effort cleanup of the .part file on failure.
                    let _ = std::fs::remove_file(&part_path);
                    let _ = handle_clone.emit(EVENT_ERROR, ErrorPayload { error: e });
                }
            }
        });

        Ok(())
    }

    /// Deletes the downloaded LLM model file.
    ///
    /// Idempotent: succeeds even when the file does not exist.
    #[tauri::command]
    pub fn delete_llm_model(handle: AppHandle) -> Result<(), String> {
        let path = model_file_path(&handle)?;

        match std::fs::remove_file(&path) {
            Ok(()) => {
                log::info!("[llm_model] Deleted model: {}", path.display());
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Failed to delete model: {e}")),
        }
    }

    // -----------------------------------------------------------------------
    // Internal download logic
    // -----------------------------------------------------------------------

    /// Performs the actual HTTP download and writes to a `.part` file.
    ///
    /// On success the `.part` file is renamed to `final_path`. Callers are
    /// responsible for cleaning up the `.part` file on failure.
    async fn run_download(
        handle: &AppHandle,
        final_path: &std::path::Path,
        part_path: &std::path::Path,
    ) -> Result<(), String> {
        log::info!("[llm_model] Downloading {} from {}", MODEL_FILENAME, MODEL_URL);

        let client = reqwest::Client::new();
        let mut response = client
            .get(MODEL_URL)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("Server returned status {}", status.as_u16()));
        }

        let total_bytes = response.content_length().unwrap_or(MODEL_APPROX_SIZE);

        let mut file = tokio::fs::File::create(part_path)
            .await
            .map_err(|e| format!("Failed to create temp file: {e}"))?;

        let mut bytes_received: u64 = 0;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Download interrupted: {e}"))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write failed: {e}"))?;

            bytes_received += chunk.len() as u64;

            let _ = handle.emit(
                EVENT_PROGRESS,
                ProgressPayload {
                    bytes_received,
                    total_bytes,
                },
            );
        }

        // Flush and close before rename.
        file.flush()
            .await
            .map_err(|e| format!("Flush failed: {e}"))?;
        drop(file);

        // Atomic rename: .part -> .gguf
        tokio::fs::rename(part_path, final_path)
            .await
            .map_err(|e| format!("Failed to finalize model file: {e}"))?;

        log::info!(
            "[llm_model] Download complete: {} ({} bytes)",
            MODEL_FILENAME,
            bytes_received
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[cfg(test)]
    mod tests {
        use super::*;

        // --- Path construction ---

        /// MODEL_FILENAME must end with .gguf (required by llama-cpp-2).
        #[test]
        fn model_filename_ends_with_gguf() {
            assert!(
                MODEL_FILENAME.ends_with(".gguf"),
                "MODEL_FILENAME must end with .gguf, got: {MODEL_FILENAME}"
            );
        }

        /// MODEL_URL must point to a HuggingFace GGUF file.
        #[test]
        fn model_url_is_huggingface_gguf() {
            assert!(
                MODEL_URL.starts_with("https://huggingface.co/"),
                "MODEL_URL must be a HuggingFace URL"
            );
            assert!(
                MODEL_URL.ends_with(".gguf"),
                "MODEL_URL must point to a .gguf file"
            );
        }

        /// MODEL_URL filename matches MODEL_FILENAME.
        #[test]
        fn model_url_filename_matches_constant() {
            assert!(
                MODEL_URL.ends_with(MODEL_FILENAME),
                "MODEL_URL should end with MODEL_FILENAME ({MODEL_FILENAME})"
            );
        }

        /// Approximate size must be at least 1 GB (sanity check for a 1.5B model).
        #[test]
        fn model_approx_size_is_reasonable() {
            assert!(
                MODEL_APPROX_SIZE >= 1_000_000_000,
                "MODEL_APPROX_SIZE should be at least 1 GB for a 1.5B-parameter model"
            );
        }

        // --- LlmModelStatus ---

        /// Status for a non-existent path must report not downloaded.
        #[test]
        fn status_not_downloaded_for_missing_file() {
            let path = std::path::PathBuf::from("/tmp/klarvo-llm-no-such-model-12345/models")
                .join(MODEL_FILENAME);

            let (downloaded, size_bytes) = if path.exists() {
                (true, std::fs::metadata(&path).map(|m| m.len()).ok())
            } else {
                (false, None)
            };

            let status = LlmModelStatus {
                downloaded,
                size_bytes,
                path: path.display().to_string(),
            };

            assert!(!status.downloaded, "model should not be downloaded");
            assert!(status.size_bytes.is_none(), "size_bytes should be None");
            assert!(
                status.path.contains(MODEL_FILENAME),
                "path should contain the model filename"
            );
        }

        /// Status for an existing dummy file must report downloaded with a size.
        #[test]
        fn status_downloaded_for_existing_file() {
            let dir = tempfile::tempdir().expect("tempdir must succeed");
            let models_dir = dir.path().join("models");
            std::fs::create_dir_all(&models_dir).expect("create models dir");
            let path = models_dir.join(MODEL_FILENAME);
            std::fs::write(&path, b"dummy gguf content").expect("write dummy model");

            let size = std::fs::metadata(&path).map(|m| m.len()).ok();
            let status = LlmModelStatus {
                downloaded: true,
                size_bytes: size,
                path: path.display().to_string(),
            };

            assert!(status.downloaded);
            assert!(status.size_bytes.is_some());
            assert!(status.size_bytes.unwrap() > 0);
        }

        // --- delete_llm_model (unit-level logic) ---

        /// Deleting a non-existent file should succeed (idempotent).
        #[test]
        fn delete_nonexistent_file_is_ok() {
            let path = std::path::PathBuf::from("/tmp/klarvo-llm-no-such-99999.gguf");
            let result = match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(format!("Failed to delete model: {e}")),
            };
            assert!(result.is_ok(), "idempotent delete must not error");
        }

        /// Deleting an existing file removes it from disk.
        #[test]
        fn delete_existing_file_removes_it() {
            let dir = tempfile::tempdir().expect("tempdir must succeed");
            let path = dir.path().join("test-model.gguf");
            std::fs::write(&path, b"dummy").expect("write dummy");

            assert!(path.exists(), "file must exist before delete");

            std::fs::remove_file(&path).expect("remove must succeed");

            assert!(!path.exists(), "file must be gone after delete");
        }
    }
}

// ---------------------------------------------------------------------------
// Android: MNN model (multi-file download)
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
pub mod android {
    use serde::{Deserialize, Serialize};
    use tauri::{AppHandle, Emitter, Manager};
    use tokio::io::AsyncWriteExt;

    /// Base URL for the Qwen2.5-1.5B-Instruct MNN model on HuggingFace.
    const BASE_URL: &str =
        "https://huggingface.co/taobao-mnn/Qwen2.5-1.5B-Instruct-MNN/resolve/main/";

    /// Model files to download and their approximate sizes (for progress reporting).
    const MODEL_FILES: &[(&str, u64)] = &[
        ("config.json", 160),
        ("llm.mnn", 1_090_000),
        ("llm.mnn.weight", 828_260_000),
        ("llm_config.json", 384),
        ("tokenizer.txt", 3_050_000),
    ];

    /// Model subdirectory name inside `{app_data_dir}/models/`.
    const MODEL_DIR_NAME: &str = "qwen2.5-1.5b-mnn";

    const EVENT_PROGRESS: &str = "klarvo://llm-model-download-progress";
    const EVENT_COMPLETE: &str = "klarvo://llm-model-download-complete";
    const EVENT_ERROR: &str = "klarvo://llm-model-download-error";

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ProgressPayload {
        bytes_received: u64,
        total_bytes: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CompletePayload {}

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ErrorPayload {
        error: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LlmModelStatus {
        pub downloaded: bool,
        pub size_bytes: Option<u64>,
        pub path: String,
    }

    fn model_dir(handle: &AppHandle) -> Result<std::path::PathBuf, String> {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
        Ok(app_data_dir.join("models").join(MODEL_DIR_NAME))
    }

    #[tauri::command]
    pub fn get_llm_model_status(handle: AppHandle) -> Result<LlmModelStatus, String> {
        let dir = model_dir(&handle)?;
        let config_exists = dir.join("config.json").exists();
        let weight_exists = dir.join("llm.mnn.weight").exists();
        let downloaded = config_exists && weight_exists;

        let size_bytes = if downloaded {
            let size: u64 = MODEL_FILES
                .iter()
                .map(|(name, _)| {
                    std::fs::metadata(dir.join(name))
                        .map(|m| m.len())
                        .unwrap_or(0)
                })
                .sum();
            Some(size)
        } else {
            None
        };

        Ok(LlmModelStatus {
            downloaded,
            size_bytes,
            path: dir.display().to_string(),
        })
    }

    #[tauri::command]
    pub fn download_llm_model(handle: AppHandle) -> Result<(), String> {
        let dir = model_dir(&handle)?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create model directory: {e}"))?;

        let handle_clone = handle.clone();

        tauri::async_runtime::spawn(async move {
            let result = run_download(&handle_clone, &dir).await;
            match result {
                Ok(()) => {
                    log::info!("[llm_model] Android MNN model download complete");
                    let _ = handle_clone.emit(EVENT_COMPLETE, CompletePayload {});
                }
                Err(e) => {
                    log::warn!("[llm_model] Android MNN model download failed: {e}");
                    let _ = handle_clone.emit(EVENT_ERROR, ErrorPayload { error: e });
                }
            }
        });

        Ok(())
    }

    #[tauri::command]
    pub fn delete_llm_model(handle: AppHandle) -> Result<(), String> {
        let dir = model_dir(&handle)?;
        for (name, _) in MODEL_FILES {
            let path = dir.join(name);
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Failed to delete {name}: {e}")),
            }
        }
        log::info!("[llm_model] Android MNN model deleted");
        Ok(())
    }

    async fn run_download(
        handle: &AppHandle,
        dir: &std::path::Path,
    ) -> Result<(), String> {
        let client = reqwest::Client::new();
        let total_bytes: u64 = MODEL_FILES.iter().map(|(_, size)| size).sum();
        let mut cumulative: u64 = 0;

        for (name, _approx_size) in MODEL_FILES {
            let url = format!("{BASE_URL}{name}");
            let final_path = dir.join(name);
            let part_path = dir.join(format!("{name}.part"));

            log::info!("[llm_model] Downloading {name} from {url}");

            let mut response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("HTTP request failed for {name}: {e}"))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Server returned {} for {name}",
                    response.status().as_u16()
                ));
            }

            let mut file = tokio::fs::File::create(&part_path)
                .await
                .map_err(|e| format!("Failed to create {name}.part: {e}"))?;

            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| format!("Download interrupted for {name}: {e}"))?
            {
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Write failed for {name}: {e}"))?;

                cumulative += chunk.len() as u64;
                let _ = handle.emit(
                    EVENT_PROGRESS,
                    ProgressPayload {
                        bytes_received: cumulative,
                        total_bytes,
                    },
                );
            }

            file.flush()
                .await
                .map_err(|e| format!("Flush failed for {name}: {e}"))?;
            drop(file);

            tokio::fs::rename(&part_path, &final_path)
                .await
                .map_err(|e| format!("Failed to finalize {name}: {e}"))?;
        }

        Ok(())
    }
}
